use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    MailCliError,
    fs_utils::{collect_sorted_files, is_json_file},
};

pub(super) fn collect_lossless_json_files(path: &Path) -> Result<Vec<PathBuf>, MailCliError> {
    let metadata = fs::metadata(path)
        .map_err(|source| MailCliError::Io { source, path: path.to_path_buf() })?;
    if metadata.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }
    if !metadata.is_dir() {
        return Err(MailCliError::InvalidInputPath { path: path.to_path_buf() });
    }

    collect_sorted_files(path, is_json_file)
}

pub(super) fn resolve_output_dir_for(
    input_path: &Path,
    output_dir: &Option<PathBuf>,
) -> Result<PathBuf, MailCliError> {
    match output_dir {
        Some(dir) => Ok(dir.clone()),
        None => {
            let metadata = fs::metadata(input_path)
                .map_err(|source| MailCliError::Io { source, path: input_path.to_path_buf() })?;
            if metadata.is_file() {
                Ok(input_path.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from(".")))
            } else {
                Ok(input_path.to_path_buf())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_lossless_json_files_only_includes_json() {
        let temp = tempfile::tempdir().expect("temp dir");
        let dir = temp.path();
        fs::File::create(dir.join("a.json")).expect("create a.json");
        fs::File::create(dir.join("b.txt")).expect("create b.txt");
        fs::File::create(dir.join("c.JSON")).expect("create c.JSON");

        let files = collect_lossless_json_files(dir).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|path| path.ends_with("a.json")));
        assert!(files.iter().any(|path| path.ends_with("c.JSON")));
    }

    #[test]
    fn collect_lossless_json_files_returns_sorted_paths() {
        let temp = tempfile::tempdir().expect("temp dir");
        let dir = temp.path();
        fs::File::create(dir.join("z.json")).expect("create z.json");
        fs::File::create(dir.join("a.json")).expect("create a.json");

        let files = collect_lossless_json_files(dir).unwrap();
        let file_names: Vec<_> =
            files.iter().map(|path| path.file_name().unwrap().to_str().unwrap()).collect();
        assert_eq!(file_names, vec!["a.json", "z.json"]);
    }
}
