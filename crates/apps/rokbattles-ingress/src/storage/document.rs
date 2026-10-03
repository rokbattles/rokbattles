//! BSON document construction, checksums, and zstd compression for stored mail.
//!
//! Builds documents without accessing MongoDB. The parent module handles insertion
//! and duplicate IDs.

use std::io::Cursor;

use mongodb::bson::{Binary, Bson, DateTime, Document, doc, spec::BinarySubtype};
use sha2::{Digest, Sha256};

use crate::{error::ApiError, mail::metadata::MailMetadata};

/// Builds a new `g_rok_mails` document from the exact uploaded bytes.
///
/// Checksum and size describe the uncompressed bytes. Both `createdAt` and
/// `updatedAt` receive `input.now`.
pub(crate) fn build(input: DocumentInput<'_>) -> Result<Document, ApiError> {
    let size = i64::try_from(input.original_bytes.len())
        .map_err(|_error| ApiError::internal("mail binary is too large to store size"))?;
    let compressed = compress_raw_mail(input.original_bytes)?;
    let checksum = sha256_hex(input.original_bytes);

    let document = doc! {
        "metadata": {
            "userAgent": input.user_agent,
            "checksum": checksum,
            "size": size,
            "algo": "zstd",
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

    Ok(document)
}

/// Uploaded bytes and metadata for a new `g_rok_mails` document.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DocumentInput<'a> {
    pub original_bytes: &'a [u8],
    pub user_agent: &'a str,
    pub mail: &'a MailMetadata,
    pub status: &'a str,
    pub now: DateTime,
}

/// Returns the lowercase hexadecimal SHA-256 checksum of the supplied bytes.
#[must_use]
fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

/// Compresses a raw mail buffer with zstd.
fn compress_raw_mail(bytes: &[u8]) -> Result<Vec<u8>, ApiError> {
    zstd::stream::encode_all(Cursor::new(bytes), 6)
        .map_err(|error| ApiError::internal(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mail::metadata::extract_metadata;

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
        let compressed = compress_raw_mail(raw).expect("compress");

        assert_eq!(decompress(&compressed), raw);
    }

    #[test]
    fn builds_v2_document_shape() {
        let now = DateTime::now();
        let mail = MailMetadata {
            id: "12345".to_string(),
            time: 1772127772844751,
            receiver: "player_71738515".to_string(),
        };
        let doc = build(DocumentInput {
            original_bytes: b"raw-binary",
            user_agent: "ROKBattles/0.1.0",
            mail: &mail,
            status: "pending",
            now,
        })
        .expect("doc");

        assert_eq!(doc.get_str("metadata.userAgent").ok(), None);
        assert_eq!(
            doc.get_document("metadata").unwrap().get_str("userAgent").unwrap(),
            "ROKBattles/0.1.0"
        );
        assert_eq!(
            doc.get_document("metadata").unwrap().get_str("checksum").unwrap(),
            sha256_hex(b"raw-binary")
        );
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
        assert_eq!(doc.get_datetime("createdAt").unwrap(), &now);
        assert_eq!(doc.get_datetime("updatedAt").unwrap(), &now);
        assert_eq!(
            decompress(doc.get_document("mail").unwrap().get_binary_generic("binary").unwrap()),
            b"raw-binary"
        );
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
            let decoded = rokbattles_mail_codec::decode(&bytes).expect("decode sample");
            extract_metadata(&decoded).expect("extract metadata");

            let compressed = compress_raw_mail(&bytes).expect("compress sample");
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
