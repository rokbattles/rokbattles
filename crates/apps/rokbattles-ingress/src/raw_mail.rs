//! V2 raw binary mail storage helpers.

use std::io::Cursor;

use mongodb::bson::{Binary, Bson, DateTime, Document, doc, spec::BinarySubtype};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::error::ApiError;

const ZSTD_ALGO: &str = "zstd";

/// Metadata extracted from a validated decoded mail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawMailMetadata {
    pub id: String,
    pub time: i64,
    pub receiver: String,
}

/// Build the document inserted into `g_rok_mails`.
pub fn build_raw_mail_doc(input: RawMailDocumentInput<'_>) -> Result<Document, ApiError> {
    let size = i64::try_from(input.original_bytes.len())
        .map_err(|_error| ApiError::internal("mail binary is too large to store size"))?;
    let compressed = compress_raw_mail(input.original_bytes, input.zstd_level)?;

    let mut document = doc! {
        "metadata": {
            "userAgent": input.user_agent,
            "checksum": input.checksum,
            "size": size,
            "algo": ZSTD_ALGO,
        },
        "mail": {
            "id": &input.mail.id,
            "time": input.mail.time,
            "receiver": &input.mail.receiver,
            "binary": Bson::Binary(Binary {
                subtype: BinarySubtype::Generic,
                bytes: compressed,
            }),
        },
        "status": input.status,
        "createdAt": input.now,
        "updatedAt": input.now,
    };

    if let Some(entity) = input.network_entity {
        document.insert(
            "network",
            doc! {
                "entity": Bson::Binary(Binary {
                    subtype: BinarySubtype::Generic,
                    bytes: compress_raw_mail(entity, input.zstd_level)?,
                }),
            },
        );
    }

    Ok(document)
}

/// Inputs needed to build the V2 raw mail document.
#[derive(Debug, Clone, Copy)]
pub struct RawMailDocumentInput<'a> {
    pub original_bytes: &'a [u8],
    pub network_entity: Option<&'a [u8]>,
    pub user_agent: &'a str,
    pub checksum: &'a str,
    pub mail: &'a RawMailMetadata,
    pub status: &'a str,
    pub now: DateTime,
    pub zstd_level: i32,
}

/// Extract the fields required by the V2 `mail` subdocument.
pub fn extract_raw_mail_metadata(decoded: &Value) -> Result<RawMailMetadata, ApiError> {
    let root = rokbattles_mail_registry::normalize_mail_root(decoded)
        .ok_or_else(|| ApiError::bad_request("invalid mail root"))?;
    let object = root.as_object().ok_or_else(|| ApiError::bad_request("invalid mail root"))?;

    let id = object
        .get("id")
        .and_then(value_to_string)
        .or_else(|| object.get("mail_id").and_then(value_to_string))
        .or_else(|| {
            object.get("metadata").and_then(|meta| meta.get("mail_id")).and_then(value_to_string)
        })
        .ok_or_else(|| ApiError::bad_request("missing mail id"))?;
    let time = object
        .get("time")
        .and_then(value_to_i64)
        .or_else(|| {
            object.get("metadata").and_then(|meta| meta.get("mail_time")).and_then(value_to_i64)
        })
        .ok_or_else(|| ApiError::bad_request("missing mail time"))?;
    let receiver = extract_receiver_identity(object)?;

    Ok(RawMailMetadata { id, time, receiver })
}

/// Return a stable SHA-256 checksum for the exact uploaded binary.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

/// Compress a raw mail buffer with zstd.
pub fn compress_raw_mail(bytes: &[u8], zstd_level: i32) -> Result<Vec<u8>, ApiError> {
    zstd::stream::encode_all(Cursor::new(bytes), zstd_level)
        .map_err(|error| ApiError::internal(error.to_string()))
}

fn extract_receiver_identity(object: &serde_json::Map<String, Value>) -> Result<String, ApiError> {
    object
        .get("receiver")
        .and_then(Value::as_str)
        .filter(|value| value.starts_with("player_"))
        .map(str::to_string)
        .ok_or_else(|| ApiError::bad_request("missing mail receiver"))
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn value_to_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Number(value) => {
            value.as_i64().or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn decompress(compressed: &[u8]) -> Vec<u8> {
        zstd::stream::decode_all(Cursor::new(compressed)).expect("decode zstd")
    }

    #[test]
    fn sha256_hex_hashes_original_bytes() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn compression_roundtrips() {
        let raw = b"small mail payload";
        let compressed = compress_raw_mail(raw, 3).expect("compress");

        assert_eq!(decompress(&compressed), raw);
    }

    #[test]
    fn extracts_v2_metadata_from_decoded_mail() {
        let decoded = json!({
            "id": "12345",
            "time": 1772127772844751_u64,
            "sender": "system",
            "receiver": "player_71738515"
        });

        let metadata = extract_raw_mail_metadata(&decoded).expect("metadata");

        assert_eq!(
            metadata,
            RawMailMetadata {
                id: "12345".to_string(),
                time: 1772127772844751,
                receiver: "player_71738515".to_string(),
            }
        );
    }

    #[test]
    fn rejects_v2_metadata_from_singleton_array() {
        let decoded = json!([{
            "id": "12345",
            "time": 1772127772844751_u64,
            "sender": "player_11",
            "receiver": "player_22"
        }]);

        extract_raw_mail_metadata(&decoded).unwrap_err();
    }

    #[test]
    fn builds_v2_document_shape() {
        let now = DateTime::now();
        let mail = RawMailMetadata {
            id: "12345".to_string(),
            time: 1772127772844751,
            receiver: "player_71738515".to_string(),
        };
        let doc = build_raw_mail_doc(RawMailDocumentInput {
            original_bytes: b"raw-binary",
            network_entity: None,
            user_agent: "ROKBattles/0.1.0",
            checksum: "checksum",
            mail: &mail,
            status: "pending",
            now,
            zstd_level: 3,
        })
        .expect("doc");

        assert_eq!(doc.get_str("metadata.userAgent").ok(), None);
        assert_eq!(
            doc.get_document("metadata").unwrap().get_str("userAgent").unwrap(),
            "ROKBattles/0.1.0"
        );
        assert_eq!(doc.get_document("metadata").unwrap().get_str("checksum").unwrap(), "checksum");
        assert_eq!(doc.get_document("metadata").unwrap().get_str("algo").unwrap(), "zstd");
        assert_eq!(doc.get_document("metadata").unwrap().get_i64("size").unwrap(), 10);
        assert_eq!(doc.get_document("mail").unwrap().get_str("id").unwrap(), "12345");
        assert_eq!(
            doc.get_document("mail").unwrap().get_str("receiver").unwrap(),
            "player_71738515"
        );
        assert!(matches!(doc.get_document("mail").unwrap().get("binary"), Some(Bson::Binary(_))));
        assert!(!doc.contains_key("network"));
        assert_eq!(doc.get_str("status").unwrap(), "pending");
    }

    #[test]
    fn builds_relay_document_with_compressed_network_entity() {
        let now = DateTime::now();
        let mail = RawMailMetadata {
            id: "12345".to_string(),
            time: 1772127772844751,
            receiver: "player_71738515".to_string(),
        };
        let network_entity = b"raw network MailEntity";
        let doc = build_raw_mail_doc(RawMailDocumentInput {
            original_bytes: b"reconstructed-binary",
            network_entity: Some(network_entity),
            user_agent: "ROKBattles/0.1.0 (Relay)",
            checksum: "checksum",
            mail: &mail,
            status: "pending",
            now,
            zstd_level: 3,
        })
        .expect("doc");

        let compressed_entity = doc
            .get_document("network")
            .expect("network document")
            .get_binary_generic("entity")
            .expect("network entity");

        assert_eq!(decompress(compressed_entity), network_entity);
    }

    #[test]
    fn selected_samples_decode_and_raw_compression_roundtrips() {
        let samples = [
            "../../../samples/Rss/Persistent.Mail.118801516499340535",
            "../../../samples/Battle/Persistent.Mail.100439187175234501131",
            "../../../samples/Battle/Persistent.Mail.18895907175034307923",
        ];

        for sample in samples {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(sample);
            let bytes = std::fs::read(path).expect("read sample");
            let decoded = rokbattles_mail_decoder::decode(&bytes).expect("decode sample");
            extract_raw_mail_metadata(&decoded).expect("extract metadata");

            let compressed = compress_raw_mail(&bytes, 3).expect("compress sample");
            assert_eq!(decompress(&compressed), bytes);
        }
    }

    #[test]
    #[ignore = "prints compression benchmark data for local cutoff decisions"]
    fn benchmark_sample_compression() {
        let sample_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../samples");
        let mut samples = Vec::new();
        collect_raw_samples(&sample_root, &mut samples);
        samples.sort();

        let mut rows = Vec::new();
        for path in samples {
            let bytes = std::fs::read(&path).expect("read sample");
            let zstd_levels = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 15, 19];
            let zstd_results = zstd_levels.map(|level| {
                let start = std::time::Instant::now();
                let compressed =
                    zstd::stream::encode_all(Cursor::new(&bytes), level).expect("zstd");
                (level, compressed.len(), start.elapsed())
            });
            rows.push((bytes.len(), zstd_results));
        }

        let total_original: usize = rows.iter().map(|row| row.0).sum();
        println!("samples: {} original_bytes: {}", rows.len(), total_original);
        for level in [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 15, 19] {
            let total_size: usize = rows
                .iter()
                .map(|(_, zstd_results)| {
                    zstd_results.iter().find(|result| result.0 == level).unwrap().1
                })
                .sum();
            let total_time: std::time::Duration = rows
                .iter()
                .map(|(_, zstd_results)| {
                    zstd_results.iter().find(|result| result.0 == level).unwrap().2
                })
                .sum();
            println!("zstd-{level}: bytes={total_size} elapsed_ms={}", total_time.as_millis());
        }
    }

    fn collect_raw_samples(root: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(root).expect("read samples dir") {
            let entry = entry.expect("sample entry");
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().and_then(|name| name.to_str()) != Some("game") {
                    collect_raw_samples(&path, out);
                }
                continue;
            }
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if name.starts_with("Persistent.Mail.") && !name.ends_with(".json") {
                out.push(path);
            }
        }
    }
}
