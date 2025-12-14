use anyhow::{Context, Result, anyhow, bail};
use clap::{Parser, ValueEnum};
use mongodb::{bson::Document, bson::doc};
use serde_json::Value;
use std::{
    fs,
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum Mode {
    Binary,
    Json,
    Mongo,
}

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Decode/process ROK mail from binary/json/mongo and write raw (and optionally processed) JSON"
)]
struct Cli {
    /// binary (ROK mail), json (decoded JSON file), or mongo (hash lookup)
    #[arg(short = 'm', long = "mode", value_enum, default_value_t = Mode::Binary)]
    mode: Mode,

    #[arg(short = 'i', long = "input")]
    input: Option<String>,

    #[arg(short = 'o', long = "output")]
    output: PathBuf,

    /// Only write the decoded/raw JSON; skip processing.
    #[arg(short = 'r', long = "raw", default_value_t = false)]
    raw_only: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let (id, raw_json_text) = match cli.mode {
        Mode::Binary => {
            let path: PathBuf = if let Some(inp) = &cli.input {
                PathBuf::from(inp)
            } else {
                bail!("binary mode requires --input <path>");
            };

            let bytes = fs::read(&path)
                .with_context(|| format!("failed to read input file: {}", path.display()))?;
            let id = friendly_identifier_from_path(&path);
            let decoded_mail = mail_decoder::decode(&bytes)?.into_owned();
            let decoded_mail_json = serde_json::to_value(decoded_mail)?;
            let decoded_mail_json_text = serde_json::to_string(&decoded_mail_json)?;
            (id, decoded_mail_json_text)
        }
        Mode::Json => {
            let inp = cli
                .input
                .as_ref()
                .map(PathBuf::from)
                .ok_or_else(|| anyhow!("json mode requires --input <path>"))?;
            let text = fs::read_to_string(&inp)
                .with_context(|| format!("failed to read input file: {}", inp.display()))?;
            let id = friendly_identifier_from_path(&inp);
            (id, text)
        }
        Mode::Mongo => {
            let hash = cli
                .input
                .as_ref()
                .map(|s| s.to_string())
                .ok_or_else(|| anyhow!("mongo mode requires --input <hash>"))?;
            let (id, text) = fetch_mail_from_mongo(&hash).await?;
            (id, text)
        }
    };

    let decoded_mail_json: Value = serde_json::from_str(&raw_json_text)?;

    let (raw_out, processed_out) = determine_output_paths(&cli.output, &id)?;

    if let Some(dir) = raw_out.parent() {
        fs::create_dir_all(dir).ok();
    }

    write_json_file(&raw_out, &decoded_mail_json)
        .with_context(|| format!("failed to write raw output file: {}", raw_out.display()))?;

    if cli.raw_only {
        println!("Successfully wrote raw file to: '{}'", raw_out.display());
        return Ok(());
    }

    let processed_mail = mail_processor::process(&raw_json_text)?;
    let processed_mail_json = serde_json::to_value(processed_mail)?;

    if let Some(dir) = processed_out.parent() {
        fs::create_dir_all(dir).ok();
    }

    write_json_file(&processed_out, &processed_mail_json).with_context(|| {
        format!(
            "failed to write processed output file: {}",
            processed_out.display()
        )
    })?;

    println!(
        "Successfully wrote files to: '{}' and '{}'",
        raw_out.display(),
        processed_out.display()
    );

    Ok(())
}

fn extract_mail_id(path: &Path) -> Result<String> {
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

fn write_json_file(path: &Path, value: &Value) -> Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, value)?;
    Ok(())
}

fn friendly_identifier_from_path(path: &Path) -> String {
    extract_mail_id(path).unwrap_or_else(|_| {
        path.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "mail".to_string())
    })
}

fn determine_output_paths(output: &Path, id: &str) -> Result<(PathBuf, PathBuf)> {
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
        Ok((raw_out, processed_out))
    } else {
        fs::create_dir_all(output)
            .with_context(|| format!("failed to create output directory: {}", output.display()))?;
        let raw_out = output.join(format!("{}.json", id));
        let processed_out = output.join(format!("{}-processed.json", id));
        Ok((raw_out, processed_out))
    }
}

async fn fetch_mail_from_mongo(hash: &str) -> Result<(String, String)> {
    let mongo_uri =
        std::env::var("MONGO_URI").context("MONGO_URI environment variable must be set")?;
    let client = mongodb::Client::with_uri_str(&mongo_uri)
        .await
        .context("failed to create MongoDB client")?;
    let db = client
        .default_database()
        .ok_or_else(|| anyhow!("MONGO_URI must include a database name"))?;

    let col = db.collection::<Document>("mails");
    let filter = doc! { "mail.hash": hash };
    let doc_opt = col
        .find_one(filter)
        .await
        .context("failed to query MongoDB")?;
    let doc = doc_opt.ok_or_else(|| anyhow!("mail not found for hash: {}", hash))?;

    let mail = doc
        .get_document("mail")
        .map_err(|_| anyhow!("invalid document: missing mail"))?;

    let codec = mail.get_str("codec").unwrap_or("zstd");
    let mail_time = mail
        .get_i64("time")
        .map_err(|_| anyhow!("invalid document: missing mail.time"))?;
    let raw = mail
        .get_binary_generic("value")
        .map_err(|_| anyhow!("invalid document: missing binary mail value"))?;

    match codec {
        "zstd" => {
            let bytes = zstd::decode_all(&raw[..])?;
            let s = String::from_utf8(bytes).context("invalid utf-8 in decoded mail")?;
            Ok((mail_time.to_string(), s))
        }
        other => bail!("unsupported codec: {}", other),
    }
}
