use std::fs;

use crate::{Config, MailCliError, RunSummary};

mod decode;
mod paths;
mod process;

use decode::decode_file;
use paths::collect_input_files;

/// Decode each mail buffer in the input directory into JSON.
pub fn run(config: &Config) -> Result<RunSummary, MailCliError> {
    crate::fs_utils::ensure_directory(&config.input_dir)?;

    fs::create_dir_all(&config.output_dir)
        .map_err(|source| MailCliError::Io { source, path: config.output_dir.clone() })?;

    let input_files = collect_input_files(&config.input_dir)?;
    let mut decoded_files = 0;

    for input in input_files {
        decode_file(&input, &config.output_dir, config.pretty)?;
        decoded_files += 1;
    }

    Ok(RunSummary { decoded_files })
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, File},
        io::Write,
        path::Path,
    };

    use super::*;

    fn write_bytes(path: &Path, bytes: &[u8]) {
        let mut file = File::create(path).expect("create file");
        file.write_all(bytes).expect("write bytes");
    }

    #[test]
    fn run_decodes_and_writes_pretty_json() {
        let input_dir = tempfile::tempdir().expect("input dir");
        let output_dir = tempfile::tempdir().expect("output dir");
        let input_path = input_dir.path().join("sample.mail");

        let buffer = vec![0x05, 0x04, 0x01, 0, 0, 0, b'a', 0x01, 1, 0xff];
        write_bytes(&input_path, &buffer);

        let config = Config {
            input_dir: input_dir.path().to_path_buf(),
            output_dir: output_dir.path().to_path_buf(),
            pretty: true,
        };
        let summary = run(&config).unwrap();
        assert_eq!(summary.decoded_files, 1);

        let output_path = output_dir.path().join("sample.mail.json");
        let json = fs::read_to_string(output_path).expect("read output");
        assert_eq!(json, "{\n  \"a\": true\n}");
    }

    #[test]
    fn run_decodes_and_writes_compact_json_when_pretty_is_false() {
        let input_dir = tempfile::tempdir().expect("input dir");
        let output_dir = tempfile::tempdir().expect("output dir");
        let input_path = input_dir.path().join("sample.mail");

        let buffer = vec![0x05, 0x04, 0x01, 0, 0, 0, b'a', 0x01, 1, 0xff];
        write_bytes(&input_path, &buffer);

        let config = Config {
            input_dir: input_dir.path().to_path_buf(),
            output_dir: output_dir.path().to_path_buf(),
            pretty: false,
        };
        let summary = run(&config).unwrap();
        assert_eq!(summary.decoded_files, 1);

        let output_path = output_dir.path().join("sample.mail.json");
        let json = fs::read_to_string(output_path).expect("read output");
        assert_eq!(json, "{\"a\":true}");
    }

    #[test]
    fn run_rejects_non_directory_input() {
        let temp = tempfile::tempdir().expect("temp dir");
        let file_path = temp.path().join("file.bin");
        write_bytes(&file_path, &[0x01, 1]);

        let config =
            Config { input_dir: file_path, output_dir: temp.path().join("out"), pretty: true };
        let err = run(&config).unwrap_err();
        assert!(matches!(err, MailCliError::InvalidInputDir { .. }));
    }

    #[test]
    fn run_returns_zero_when_input_directory_is_empty() {
        let input_dir = tempfile::tempdir().expect("input dir");
        let output_dir = tempfile::tempdir().expect("output dir");

        let config = Config {
            input_dir: input_dir.path().to_path_buf(),
            output_dir: output_dir.path().to_path_buf(),
            pretty: true,
        };
        let summary = run(&config).unwrap();
        assert_eq!(summary.decoded_files, 0);
    }
}
