//! Packet API id lookup loaded from the generated runtime artifact.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

/// Runtime mapping from packet API id to the message used for storage and decode.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiMapping {
    /// Stable message name stored with processed packets.
    pub schema: String,
    /// Descriptor message name used to decode the protobuf payload.
    pub descriptor: String,
}

impl ApiMapping {
    /// Message name written to the processed packet document.
    pub fn schema(&self) -> &str {
        &self.schema
    }

    /// Descriptor message name used when decoding the packet payload.
    pub fn descriptor(&self) -> &str {
        &self.descriptor
    }
}

/// Fast lookup table keyed by numeric packet API id.
#[derive(Debug, Clone)]
pub struct ApiMap {
    entries: HashMap<u32, ApiMapping>,
}

impl ApiMap {
    /// Convert the JSON artifact map into numeric ids used at runtime.
    pub fn from_artifact(
        api_map: BTreeMap<String, ApiMapping>,
    ) -> Result<Self, crate::error::ProcessorError> {
        let mut entries = HashMap::new();
        for (key, value) in api_map {
            let api_id = key
                .parse::<u32>()
                .map_err(|_error| crate::error::ProcessorError::InvalidField("api_map key"))?;
            entries.insert(api_id, value);
        }
        Ok(Self { entries })
    }

    /// Return the mapping for one packet API id, if this artifact knows it.
    pub fn get(&self, api_id: u32) -> Option<&ApiMapping> {
        self.entries.get(&api_id)
    }

    /// Return every descriptor name registered in the extracted API table.
    pub fn descriptor_names(&self) -> impl Iterator<Item = &str> {
        self.entries.values().map(ApiMapping::descriptor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_artifact_rejects_non_numeric_api_id() {
        let error = ApiMap::from_artifact(BTreeMap::from([(
            "bad".to_string(),
            ApiMapping { schema: "Test".to_string(), descriptor: "Test".to_string() },
        )]))
        .expect_err("invalid api id should fail");

        assert_eq!(error.to_string(), "invalid field: api_map key");
    }
}
