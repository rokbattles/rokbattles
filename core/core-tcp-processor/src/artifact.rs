//! Loader for the generated runtime artifact used by the processor.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::{
    api_map::{ApiMap, ApiMapping},
    descriptor::{DescriptorArtifact, DescriptorSet},
    error::ProcessorError,
};

const CURRENT_SCHEMA_VERSION: u32 = 1;

/// API mappings and descriptor messages loaded from the generated runtime artifact.
#[derive(Debug, Clone)]
pub struct RuntimeArtifact {
    pub api_map: ApiMap,
    pub descriptors: DescriptorSet,
}

#[derive(Debug, Deserialize)]
struct RuntimeArtifactFile {
    schema_version: u32,
    api_map: BTreeMap<String, ApiMapping>,
    descriptors: DescriptorArtifact,
}

impl RuntimeArtifact {
    /// Load the artifact shipped beside this crate.
    pub fn load_default() -> Result<Self, ProcessorError> {
        Self::load(&default_artifact_path())
    }

    /// Load one artifact file from disk and validate its schema version.
    pub fn load(path: &Path) -> Result<Self, ProcessorError> {
        let raw = fs::read_to_string(path)?;
        let artifact: RuntimeArtifactFile = serde_json::from_str(&raw)?;
        if artifact.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(ProcessorError::InvalidField("schema_version"));
        }

        Ok(Self {
            api_map: ApiMap::from_artifact(artifact.api_map)?,
            descriptors: DescriptorSet::from_artifact(artifact.descriptors),
        })
    }
}

fn default_artifact_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("artifacts/tcp-processor-artifact.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_reads_runtime_artifact() {
        let path = write_fixture(
            "valid",
            r#"{
  "schema_version": 1,
  "api_map": {
    "14": { "schema": "Test", "descriptor": "Test" }
  },
  "descriptors": {
    "messages": [
      {
        "name": "Test",
        "full_name": "Test",
        "fields": [
          { "name": "Name", "number": 1, "type": 9, "type_name": "" }
        ],
        "nested": []
      }
    ]
  }
}"#,
        );

        let artifact = RuntimeArtifact::load(&path).expect("artifact should load");
        let decoded = artifact.descriptors.decode("Test", &[0x0a, 0x03, b'b', b'o', b'b']);
        fs::remove_file(path).expect("artifact fixture should clean up");

        assert_eq!(artifact.api_map.get(14).map(ApiMapping::schema), Some("Test"));
        assert_eq!(decoded.get("Name").and_then(serde_json::Value::as_str), Some("bob"));
    }

    #[test]
    fn load_rejects_unknown_schema_version() {
        let path = write_fixture(
            "bad-version",
            r#"{
  "schema_version": 99,
  "api_map": {},
  "descriptors": { "messages": [] }
}"#,
        );

        let error = RuntimeArtifact::load(&path).expect_err("version should fail");
        fs::remove_file(path).expect("artifact fixture should clean up");

        assert_eq!(error.to_string(), "invalid field: schema_version");
    }

    fn write_fixture(name: &str, body: &str) -> PathBuf {
        let path = std::env::temp_dir()
            .join(format!("tcp-processor-artifact-{name}-{}.json", std::process::id()));
        fs::write(&path, body).expect("artifact fixture should write");
        path
    }
}
