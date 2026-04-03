use std::path::{Path, PathBuf};

use crate::{
    MailCliError,
    fs_utils::{collect_sorted_files, is_json_file, output_path_with_suffix},
};

pub(super) fn collect_input_files(dir: &Path) -> Result<Vec<PathBuf>, MailCliError> {
    collect_sorted_files(dir, |path| !is_json_file(path))
}

pub(super) fn output_path(output_dir: &Path, input_path: &Path) -> Result<PathBuf, MailCliError> {
    output_path_with_suffix(output_dir, input_path, "", "json")
}

pub(super) fn processed_output_path(
    output_dir: &Path,
    input_path: &Path,
) -> Result<PathBuf, MailCliError> {
    output_path_with_suffix(output_dir, input_path, "-processed", "json")
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    #[test]
    fn output_path_appends_json_extension() {
        let output = PathBuf::from("/tmp/out");
        let input = PathBuf::from("/tmp/in/sample.bin");
        let result = output_path(&output, &input).unwrap();
        assert_eq!(result, PathBuf::from("/tmp/out/sample.bin.json"));
    }

    #[test]
    fn processed_output_path_appends_suffix() {
        let output = PathBuf::from("/tmp/out");
        let input = PathBuf::from("/tmp/in/sample.bin");
        let result = processed_output_path(&output, &input).unwrap();
        assert_eq!(result, PathBuf::from("/tmp/out/sample.bin-processed.json"));
    }

    #[test]
    fn collect_input_files_skips_json() {
        let temp = tempfile::tempdir().expect("temp dir");
        let dir = temp.path();
        std::fs::write(dir.join("a.bin"), [0x01, 1]).expect("write a.bin");
        std::fs::write(dir.join("b.json"), b"{}").expect("write b.json");
        std::fs::write(dir.join("c"), [0x01, 0]).expect("write c");

        let files = collect_input_files(dir).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|path| path.ends_with("a.bin")));
        assert!(files.iter().any(|path| path.ends_with("c")));
    }

    #[test]
    fn collect_input_files_returns_sorted_paths() {
        let temp = tempfile::tempdir().expect("temp dir");
        let dir = temp.path();
        std::fs::write(dir.join("z.mail"), [0x01, 1]).expect("write z.mail");
        std::fs::write(dir.join("a.mail"), [0x01, 1]).expect("write a.mail");

        let files = collect_input_files(dir).unwrap();
        let file_names: Vec<_> =
            files.iter().map(|path| path.file_name().unwrap().to_str().unwrap()).collect();
        assert_eq!(file_names, vec!["a.mail", "z.mail"]);
    }

    #[test]
    fn output_path_returns_error_when_input_path_has_no_file_name() {
        let err = output_path(Path::new("/tmp/out"), Path::new("..")).unwrap_err();
        assert!(matches!(err, MailCliError::MissingFileName { .. }));
    }
}
