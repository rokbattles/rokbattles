use anyhow::{Context, Result, bail};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{sync::Semaphore, task::JoinSet};

use crate::{
    io,
    processing::{self, InputFormat},
};

pub(crate) struct DirectoryJob {
    pub(crate) input_dir: PathBuf,
    pub(crate) output_dir: PathBuf,
    pub(crate) raw_only: bool,
    pub(crate) concurrency: usize,
}

#[derive(Debug, Default)]
pub(crate) struct DirectorySummary {
    pub(crate) processed: usize,
    pub(crate) skipped: usize,
    pub(crate) failed: usize,
}

#[derive(Debug)]
struct DirectoryItem {
    path: PathBuf,
    format: InputFormat,
}

pub(crate) async fn process_directory(job: DirectoryJob) -> Result<DirectorySummary> {
    let (items, skipped) = collect_directory_items(&job.input_dir)?;

    if items.is_empty() {
        return Ok(DirectorySummary {
            processed: 0,
            skipped,
            failed: 0,
        });
    }

    fs::create_dir_all(&job.output_dir).with_context(|| {
        format!(
            "failed to create output directory: {}",
            job.output_dir.display()
        )
    })?;

    let semaphore = Arc::new(Semaphore::new(job.concurrency));
    let mut join_set = JoinSet::new();

    for item in items {
        let permit = semaphore.clone().acquire_owned().await?;
        let output_dir = job.output_dir.clone();
        let raw_only = job.raw_only;
        let path_display = item.path.display().to_string();

        join_set.spawn(async move {
            let _permit = permit;
            // Spawn blocking work to keep the async runtime responsive.
            tokio::task::spawn_blocking(move || {
                processing::process_local_file(&item.path, &output_dir, raw_only, item.format)
            })
            .await
            .context("processing task panicked")?
            .with_context(|| format!("failed to process {}", path_display))
        });
    }

    let mut summary = DirectorySummary {
        skipped,
        ..DirectorySummary::default()
    };

    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(_paths)) => {
                summary.processed += 1;
            }
            Ok(Err(err)) => {
                summary.failed += 1;
                eprintln!("{err}");
            }
            Err(err) => {
                summary.failed += 1;
                eprintln!("processing task failed: {err}");
            }
        }
    }

    if summary.failed > 0 {
        bail!("{} mail(s) failed to process", summary.failed);
    }

    Ok(summary)
}

fn collect_directory_items(input_dir: &Path) -> Result<(Vec<DirectoryItem>, usize)> {
    let mut items = Vec::new();
    let mut skipped = 0;

    for entry in fs::read_dir(input_dir)
        .with_context(|| format!("failed to read input directory: {}", input_dir.display()))?
    {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            skipped += 1;
            continue;
        }
        let path = entry.path();
        match classify_path(&path) {
            Some(format) => items.push(DirectoryItem { path, format }),
            None => skipped += 1,
        }
    }

    Ok((items, skipped))
}

pub(crate) fn classify_path(path: &Path) -> Option<InputFormat> {
    let name = path.file_name()?.to_string_lossy();
    if io::is_processed_filename(&name) {
        return None;
    }

    let is_json = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("json"))
        .unwrap_or(false);

    // Directory mode accepts both decoded JSON and raw binary mail payloads.
    Some(if is_json {
        InputFormat::Json
    } else {
        InputFormat::Binary
    })
}
