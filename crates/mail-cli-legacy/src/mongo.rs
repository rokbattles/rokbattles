use anyhow::{Context, Result, anyhow, bail};
use mongodb::{bson::Document, bson::doc};

pub(crate) async fn fetch_mail_from_mongo(hash: &str) -> Result<(String, String)> {
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
