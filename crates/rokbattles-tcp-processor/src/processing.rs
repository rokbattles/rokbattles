//! Processing loop that turns completed TCP captures into decoded packet documents.

use mongodb::bson::{DateTime, oid::ObjectId};
use tracing::{debug, error, info, warn};

use crate::{
    api_map::ApiMap,
    artifact::RuntimeArtifact,
    config::Config,
    descriptor::DescriptorSet,
    error::ProcessorError,
    proto::unwrap_effective_payload,
    storage::{ProcessedPacket, RawBatch, Storage},
    stream::{DecryptedFrame, Direction, RawFragment, StreamDecryptor},
};

pub async fn process_loop(storage: Storage, config: Config) -> Result<(), ProcessorError> {
    let artifact = RuntimeArtifact::load_default()?;

    loop {
        match process_batch(&storage, &config, &artifact.api_map, &artifact.descriptors).await {
            Ok(0) => tokio::time::sleep(config.idle_sleep).await,
            Ok(count) => info!(count, "processed tcp captures"),
            Err(error) => {
                error!(error = %error, "tcp processing batch failed");
                tokio::time::sleep(config.idle_sleep).await;
            }
        }
    }
}

async fn process_batch(
    storage: &Storage,
    config: &Config,
    api_map: &ApiMap,
    descriptors: &DescriptorSet,
) -> Result<usize, ProcessorError> {
    let captures = storage.find_ready_captures(config.batch_size).await?;
    let mut processed = 0usize;

    for capture in captures {
        let batches = storage.load_capture_batches(&capture.capture_id).await?;
        if batches.is_empty() {
            continue;
        }

        match process_capture(&batches, config, api_map, descriptors) {
            Ok(packets) => {
                let group_id = ObjectId::new();
                storage.insert_processed(&packets, group_id, DateTime::now()).await?;
                storage.delete_capture(&capture.capture_id).await?;
                processed = processed.saturating_add(1);
                debug!(
                    capture_id = %capture.capture_id,
                    group_id = %group_id,
                    packet_count = packets.len(),
                    "processed tcp capture"
                );
            }
            Err(error) => {
                warn!(capture_id = %capture.capture_id, error = %error, "dropping tcp capture");
                storage.delete_capture(&capture.capture_id).await?;
            }
        }
    }

    Ok(processed)
}

fn process_capture(
    batches: &[RawBatch],
    config: &Config,
    api_map: &ApiMap,
    descriptors: &DescriptorSet,
) -> Result<Vec<ProcessedPacket>, ProcessorError> {
    let first = batches.first().ok_or(ProcessorError::MissingField("batch"))?;
    if !batches.iter().any(|batch| batch.stream_ended) {
        return Err(ProcessorError::Decode("capture is not complete".to_string()));
    }

    let mut decryptor = StreamDecryptor::new(first.handshake.key1, first.handshake.key2);
    let mut fragments = collect_fragments(batches);
    fragments.sort_by_key(|fragment| fragment.index);

    let mut packets = Vec::new();
    for fragment in fragments {
        let frames = decryptor.push(fragment).map_err(ProcessorError::Decode)?;
        for frame in frames {
            if let Some(packet) = process_frame(frame, config, api_map, descriptors)? {
                packets.push(packet);
            }
        }
    }

    Ok(packets)
}

fn collect_fragments(batches: &[RawBatch]) -> Vec<&RawFragment> {
    let mut fragments = Vec::new();
    for batch in batches {
        fragments.extend(&batch.fragments);
    }
    fragments
}

fn process_frame(
    frame: DecryptedFrame,
    config: &Config,
    api_map: &ApiMap,
    descriptors: &DescriptorSet,
) -> Result<Option<ProcessedPacket>, ProcessorError> {
    if frame.direction != Direction::ServerToClient {
        return Ok(None);
    }
    let Some(packet) = decode_server_frame_body(&frame.body, api_map, descriptors)? else {
        return Ok(None);
    };
    if !config.api_filter.accepts(packet.api_id) {
        return Ok(None);
    }
    Ok(Some(packet))
}

/// Decode one already-decrypted server frame body into a processed packet.
pub fn decode_server_frame_body(
    body: &[u8],
    api_map: &ApiMap,
    descriptors: &DescriptorSet,
) -> Result<Option<ProcessedPacket>, ProcessorError> {
    let unwrapped = unwrap_effective_payload(body, |api_id| api_map.get(api_id).is_some())
        .map_err(ProcessorError::Decode)?;
    let Some(api_id) = unwrapped.api_id else {
        return Ok(None);
    };
    let Some(mapping) = api_map.get(api_id) else {
        return Ok(None);
    };

    let decoded = descriptors.decode(mapping.descriptor(), &unwrapped.payload, Some(api_map));
    Ok(Some(ProcessedPacket { api_id, schema: mapping.schema().to_string(), value: decoded }))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::{
        api_map::ApiMapping,
        config::ApiFilter,
        descriptor::{DescriptorArtifact, Field, Message},
    };

    #[test]
    fn collect_fragments_keeps_all_batch_fragments() {
        let batches = vec![RawBatch {
            stream_ended: true,
            handshake: crate::storage::RawHandshake { key1: 1, key2: 2 },
            fragments: vec![RawFragment {
                index: 0,
                direction: crate::stream::Direction::ServerToClient,
                payload: vec![0],
            }],
        }];

        let fragments = collect_fragments(&batches);

        assert_eq!(fragments.len(), 1);
    }

    #[test]
    fn disabled_filter_accepts_unknown_future_ids() {
        let config = Config {
            mongo_uri: "mongodb://localhost/test".to_string(),
            sentry_dsn: None,
            batch_size: 1,
            idle_sleep: std::time::Duration::from_secs(1),
            api_filter: ApiFilter { enabled: false, allowed_api_ids: BTreeSet::new() },
        };

        assert!(config.api_filter.accepts(1234));
    }

    #[test]
    fn process_frame_decodes_mapped_payload() {
        let config = Config {
            mongo_uri: "mongodb://localhost/test".to_string(),
            sentry_dsn: None,
            batch_size: 1,
            idle_sleep: std::time::Duration::from_secs(1),
            api_filter: ApiFilter { enabled: false, allowed_api_ids: BTreeSet::new() },
        };
        let api_map = ApiMap::from_artifact(std::collections::BTreeMap::from([(
            "14".to_string(),
            ApiMapping { schema: "Test".to_string(), descriptor: "Test".to_string() },
        )]))
        .expect("api map should load");
        let descriptors = DescriptorSet::from_artifact(DescriptorArtifact {
            messages: vec![Message {
                name: "Test".to_string(),
                full_name: "Test".to_string(),
                fields: vec![Field {
                    name: "Name".to_string(),
                    number: Some(1),
                    r#type: Some(9),
                    type_name: None,
                }],
                nested: Vec::new(),
            }],
        });
        let frame = DecryptedFrame {
            direction: crate::stream::Direction::ServerToClient,
            index: 0,
            body: vec![0x08, 0x0e, 0x12, 0x05, 0x0a, 0x03, b'b', b'o', b'b'],
        };

        let packet = process_frame(frame, &config, &api_map, &descriptors)
            .expect("frame should process")
            .expect("packet should be decoded");

        assert_eq!(packet.schema, "Test");
        assert_eq!(packet.value.get("Name").and_then(serde_json::Value::as_str), Some("bob"));
    }

    #[test]
    fn process_frame_ignores_client_to_server_frames() {
        let config = sample_config(ApiFilter { enabled: false, allowed_api_ids: BTreeSet::new() });
        let api_map = ApiMap::from_artifact(std::collections::BTreeMap::new())
            .expect("empty api map should load");
        let descriptors = DescriptorSet::from_artifact(DescriptorArtifact { messages: Vec::new() });
        let frame = DecryptedFrame {
            direction: crate::stream::Direction::ClientToServer,
            index: 0,
            body: vec![0xff],
        };

        let packet = process_frame(frame, &config, &api_map, &descriptors)
            .expect("client frames should be skipped without decoding");

        assert!(packet.is_none());
    }

    #[test]
    fn process_frame_ignores_unmapped_api_id() {
        let config = sample_config(ApiFilter { enabled: false, allowed_api_ids: BTreeSet::new() });
        let api_map = ApiMap::from_artifact(std::collections::BTreeMap::new())
            .expect("empty api map should load");
        let descriptors = DescriptorSet::from_artifact(DescriptorArtifact { messages: Vec::new() });
        let frame = DecryptedFrame {
            direction: crate::stream::Direction::ServerToClient,
            index: 0,
            body: vec![0x08, 0x0e, 0x12, 0x00],
        };

        let packet =
            process_frame(frame, &config, &api_map, &descriptors).expect("frame should process");

        assert!(packet.is_none());
    }

    #[test]
    fn process_frame_respects_enabled_api_filter() {
        let config =
            sample_config(ApiFilter { enabled: true, allowed_api_ids: BTreeSet::from([99]) });
        let api_map = ApiMap::from_artifact(std::collections::BTreeMap::from([(
            "14".to_string(),
            ApiMapping { schema: "Test".to_string(), descriptor: "Test".to_string() },
        )]))
        .expect("api map should load");
        let descriptors = DescriptorSet::from_artifact(DescriptorArtifact { messages: Vec::new() });
        let frame = DecryptedFrame {
            direction: crate::stream::Direction::ServerToClient,
            index: 0,
            body: vec![0x08, 0x0e, 0x12, 0x00],
        };

        let packet =
            process_frame(frame, &config, &api_map, &descriptors).expect("frame should process");

        assert!(packet.is_none());
    }

    #[test]
    fn process_capture_rejects_incomplete_capture() {
        let config = sample_config(ApiFilter { enabled: false, allowed_api_ids: BTreeSet::new() });
        let api_map = ApiMap::from_artifact(std::collections::BTreeMap::new())
            .expect("empty api map should load");
        let descriptors = DescriptorSet::from_artifact(DescriptorArtifact { messages: Vec::new() });
        let batches = vec![RawBatch {
            stream_ended: false,
            handshake: crate::storage::RawHandshake { key1: 1, key2: 2 },
            fragments: Vec::new(),
        }];

        let error = process_capture(&batches, &config, &api_map, &descriptors)
            .expect_err("incomplete captures should be rejected");

        assert_eq!(error.to_string(), "decode failed: capture is not complete");
    }

    fn sample_config(api_filter: ApiFilter) -> Config {
        Config {
            mongo_uri: "mongodb://localhost/test".to_string(),
            sentry_dsn: None,
            batch_size: 1,
            idle_sleep: std::time::Duration::from_secs(1),
            api_filter,
        }
    }
}
