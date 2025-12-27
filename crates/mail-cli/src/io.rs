use anyhow::{Context, Result, anyhow, bail};
use serde_json::Value;
use std::{
    fs::{self, File},
    io::BufWriter,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub(crate) struct OutputPaths {
    pub raw: PathBuf,
    pub processed: PathBuf,
    pub processed_v2: Option<PathBuf>,
}

pub(crate) fn determine_output_paths(
    output: &Path,
    id: &str,
    include_v2: bool,
) -> Result<OutputPaths> {
    let looks_like_file = output.extension().is_some();

    if looks_like_file {
        let raw_out = output.to_path_buf();
        let processed_out = {
            let ext = output.extension().map(|e| e.to_os_string());
            let stem = output
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| id.to_string());
            let parent = output.parent().map(|d| d.to_path_buf()).unwrap_or_default();
            let mut fname = format!("{}-processed", stem);
            if let Some(e) = ext {
                fname.push('.');
                fname.push_str(&e.to_string_lossy());
            }
            parent.join(fname)
        };
        let processed_v2_out = if include_v2 {
            let ext = output.extension().map(|e| e.to_os_string());
            let stem = output
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| id.to_string());
            let parent = output.parent().map(|d| d.to_path_buf()).unwrap_or_default();
            let mut fname = format!("{}-processed-v2", stem);
            if let Some(e) = ext {
                fname.push('.');
                fname.push_str(&e.to_string_lossy());
            }
            Some(parent.join(fname))
        } else {
            None
        };
        Ok(OutputPaths {
            raw: raw_out,
            processed: processed_out,
            processed_v2: processed_v2_out,
        })
    } else {
        let raw_out = output.join(format!("{}.json", id));
        let processed_out = output.join(format!("{}-processed.json", id));
        let processed_v2_out = include_v2.then(|| output.join(format!("{}-processed-v2.json", id)));
        Ok(OutputPaths {
            raw: raw_out,
            processed: processed_out,
            processed_v2: processed_v2_out,
        })
    }
}

pub(crate) fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)
            .with_context(|| format!("failed to create output directory: {}", dir.display()))?;
    }
    Ok(())
}

pub(crate) fn write_json_file(path: &Path, value: &Value) -> Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, value)?;
    Ok(())
}

pub(crate) fn friendly_identifier_from_path(path: &Path) -> String {
    extract_mail_id(path).unwrap_or_else(|_| {
        path.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "mail".to_string())
    })
}

pub(crate) fn extract_mail_id(path: &Path) -> Result<String> {
    let file_name = path
        .file_name()
        .ok_or_else(|| anyhow!("missing file name"))?
        .to_string_lossy();

    let mut parts = file_name.split('.').collect::<Vec<_>>();
    if parts.is_empty() {
        bail!("unexpected filename format");
    }
    let last = parts.pop().unwrap();

    if last.chars().all(|c| c.is_ascii_digit()) {
        return Ok(last.to_string());
    }

    if let Some(segment) = parts
        .iter()
        .rev()
        .find(|segment| segment.chars().all(|c| c.is_ascii_digit()))
    {
        return Ok((*segment).to_string());
    }

    bail!("could not find id segment in filename: {}", file_name);
}

pub(crate) fn is_processed_filename(name: &str) -> bool {
    name.ends_with("-processed.json") || name.ends_with("-processed-v2.json")
}
