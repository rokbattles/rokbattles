//! MongoDB access for raw TCP capture batches and processed packet output.

use core_bson::bson_to_u64;
use futures::TryStreamExt;
use mongodb::{
    Collection, IndexModel,
    bson::{Bson, DateTime, Document, doc, oid::ObjectId},
    options::FindOptions,
};
use serde_json::Value;

use crate::{
    error::ProcessorError,
    stream::{Direction, RawFragment},
};

pub const STATUS_PENDING: &str = "pending";

#[derive(Debug, Clone)]
pub struct Storage {
    raw: Collection<Document>,
    processed: Collection<Document>,
}

#[derive(Debug, Clone)]
pub struct ReadyCapture {
    pub capture_id: String,
}

#[derive(Debug, Clone)]
pub struct RawBatch {
    pub stream_ended: bool,
    pub handshake: RawHandshake,
    pub fragments: Vec<RawFragment>,
}

#[derive(Debug, Clone, Copy)]
pub struct RawHandshake {
    pub key1: u64,
    pub key2: u64,
}

#[derive(Debug, Clone)]
pub struct ProcessedPacket {
    pub api_id: u32,
    pub schema: String,
    pub value: Value,
}

impl Storage {
    pub fn new(db: mongodb::Database) -> Self {
        Self {
            raw: db.collection("tcp_streams_raw"),
            processed: db.collection("tcp_packets_processed"),
        }
    }

    pub async fn ensure_indexes(&self) -> mongodb::error::Result<()> {
        let raw_ready_index = IndexModel::builder()
            .keys(doc! { "status": 1, "stream_ended": 1, "updatedAt": 1 })
            .build();
        self.raw.create_index(raw_ready_index).await?;

        let processed_api = IndexModel::builder().keys(doc! { "apiId": 1, "createdAt": 1 }).build();
        let processed_group =
            IndexModel::builder().keys(doc! { "groupId": 1, "createdAt": 1 }).build();
        let processed_created = IndexModel::builder().keys(doc! { "createdAt": 1 }).build();
        self.processed.create_index(processed_api).await?;
        self.processed.create_index(processed_group).await?;
        self.processed.create_index(processed_created).await?;
        Ok(())
    }

    pub async fn find_ready_captures(
        &self,
        batch_size: i64,
    ) -> Result<Vec<ReadyCapture>, ProcessorError> {
        let filter = doc! {
            "status": STATUS_PENDING,
            "stream_ended": true,
        };
        let opts = FindOptions::builder()
            .limit(batch_size)
            .sort(doc! { "updatedAt": 1 })
            .projection(doc! { "capture_id": 1 })
            .build();
        let mut cursor = self.raw.find(filter).with_options(opts).await?;
        let mut captures = Vec::new();
        while let Some(doc) = cursor.try_next().await? {
            let capture_id = doc
                .get_str("capture_id")
                .map_err(|_error| ProcessorError::MissingField("capture_id"))?
                .to_string();
            captures.push(ReadyCapture { capture_id });
        }
        Ok(captures)
    }

    pub async fn load_capture_batches(
        &self,
        capture_id: &str,
    ) -> Result<Vec<RawBatch>, ProcessorError> {
        let filter = doc! {
            "capture_id": capture_id,
            "status": STATUS_PENDING,
        };
        let opts = FindOptions::builder().sort(doc! { "batch_index": 1 }).build();
        let mut cursor = self.raw.find(filter).with_options(opts).await?;
        let mut batches = Vec::new();
        while let Some(doc) = cursor.try_next().await? {
            batches.push(parse_raw_batch(doc)?);
        }
        Ok(batches)
    }

    pub async fn insert_processed(
        &self,
        packets: &[ProcessedPacket],
        group_id: ObjectId,
        now: DateTime,
    ) -> Result<(), ProcessorError> {
        if packets.is_empty() {
            return Ok(());
        }

        let mut docs = Vec::with_capacity(packets.len());
        for packet in packets {
            docs.push(processed_doc(packet, group_id, now));
        }
        self.processed.insert_many(docs).ordered(false).await?;
        Ok(())
    }

    pub async fn delete_capture(&self, capture_id: &str) -> mongodb::error::Result<()> {
        self.raw.delete_many(doc! { "capture_id": capture_id }).await?;
        Ok(())
    }
}

fn parse_raw_batch(doc: Document) -> Result<RawBatch, ProcessorError> {
    let stream_ended = doc.get_bool("stream_ended").unwrap_or(false);
    let handshake_doc = doc
        .get_document("handshake")
        .map_err(|_error| ProcessorError::MissingField("handshake"))?;
    let handshake = RawHandshake {
        key1: handshake_doc
            .get("key1")
            .and_then(bson_to_u64)
            .ok_or(ProcessorError::MissingField("handshake.key1"))?,
        key2: handshake_doc
            .get("key2")
            .and_then(bson_to_u64)
            .ok_or(ProcessorError::MissingField("handshake.key2"))?,
    };
    let fragments = parse_fragments(doc.get("fragments"))?;

    Ok(RawBatch { stream_ended, handshake, fragments })
}

fn parse_fragments(value: Option<&Bson>) -> Result<Vec<RawFragment>, ProcessorError> {
    let Some(Bson::Array(items)) = value else {
        return Ok(Vec::new());
    };
    let mut fragments = Vec::with_capacity(items.len());
    for item in items {
        let Bson::Document(doc) = item else {
            return Err(ProcessorError::InvalidField("fragments"));
        };
        let index = doc
            .get("index")
            .and_then(bson_to_u64)
            .ok_or(ProcessorError::MissingField("fragments.index"))?;
        let direction = match doc.get_str("direction").ok() {
            Some("client_to_server") => Direction::ClientToServer,
            Some("server_to_client") => Direction::ServerToClient,
            _ => return Err(ProcessorError::InvalidField("fragments.direction")),
        };
        let payload = match doc.get("payload") {
            Some(Bson::Binary(binary)) => binary.bytes.clone(),
            _ => return Err(ProcessorError::MissingField("fragments.payload")),
        };
        fragments.push(RawFragment { index, direction, payload });
    }
    Ok(fragments)
}

fn processed_doc(packet: &ProcessedPacket, group_id: ObjectId, now: DateTime) -> Document {
    doc! {
        "groupId": group_id,
        "apiId": i64::from(packet.api_id),
        "schema": &packet.schema,
        "value": json_to_bson(&packet.value),
        "createdAt": now,
    }
}

fn json_to_bson(value: &Value) -> Bson {
    match value {
        Value::Null => Bson::Null,
        Value::Bool(value) => Bson::Boolean(*value),
        Value::Number(number) => number_to_bson(number),
        Value::String(value) => Bson::String(value.clone()),
        Value::Array(values) => Bson::Array(values.iter().map(json_to_bson).collect()),
        Value::Object(object) => {
            let mut doc = Document::new();
            for (key, value) in object {
                doc.insert(key, json_to_bson(value));
            }
            Bson::Document(doc)
        }
    }
}

fn number_to_bson(number: &serde_json::Number) -> Bson {
    if let Some(value) = number.as_i64() {
        return Bson::Int64(value);
    }
    if let Some(value) = number.as_u64() {
        return match i64::try_from(value) {
            Ok(value) => Bson::Int64(value),
            Err(_error) => Bson::String(value.to_string()),
        };
    }
    if let Some(value) = number.as_f64() {
        return Bson::Double(value);
    }
    Bson::Null
}

#[cfg(test)]
mod tests {
    use mongodb::bson::{Binary, spec::BinarySubtype};

    use super::*;

    #[test]
    fn parse_fragments_reads_direction_and_payload() {
        let value = Bson::Array(vec![Bson::Document(doc! {
            "index": 0i64,
            "direction": "server_to_client",
            "payload": Bson::Binary(Binary {
                subtype: BinarySubtype::Generic,
                bytes: vec![0x00, 0x01, 0xaa],
            }),
        })]);

        let fragments = parse_fragments(Some(&value)).expect("fragments should parse");

        assert_eq!(fragments.len(), 1);
        assert_eq!(
            fragments.first().map(|item| item.direction),
            Some(crate::stream::Direction::ServerToClient)
        );
    }

    #[test]
    fn processed_doc_stores_only_processed_packet_value() {
        let now = DateTime::from_millis(1_700_000_000_000);
        let group_id = ObjectId::from_bytes([1; 12]);
        let packet = ProcessedPacket {
            api_id: 14,
            schema: "Test".to_string(),
            value: serde_json::json!({ "Name": "bob" }),
        };

        let doc = processed_doc(&packet, group_id, now);

        assert_eq!(doc.get_object_id("groupId"), Ok(group_id));
        assert_eq!(doc.get_i64("apiId"), Ok(14));
        assert_eq!(doc.get_str("schema"), Ok("Test"));
        assert!(doc.get("value").is_some());
        assert!(doc.get("decoded").is_none());
        assert!(doc.get("capture_id").is_none());
        assert!(doc.get("batch_index").is_none());
        assert!(doc.get("direction").is_none());
        assert!(doc.get("frame_index").is_none());
    }

    #[test]
    fn processed_doc_stringifies_unsigned_integers_that_exceed_bson_int64() {
        let now = DateTime::from_millis(1_700_000_000_000);
        let group_id = ObjectId::from_bytes([2; 12]);
        let packet = ProcessedPacket {
            api_id: 14,
            schema: "Test".to_string(),
            value: serde_json::json!({
                "min": 42_u64,
                "max": u64::MAX,
            }),
        };

        let doc = processed_doc(&packet, group_id, now);
        let value = doc.get_document("value").expect("value should be a document");

        assert_eq!(value.get_i64("min"), Ok(42));
        assert_eq!(value.get_str("max"), Ok("18446744073709551615"));
    }
}
