use std::path::PathBuf;

/// Errors produced while loading an artifact or reconstructing an entry.
#[derive(Debug, thiserror::Error)]
pub enum ReconstructionError {
    /// The artifact could not be read.
    #[error("failed to read runtime artifact {}: {source}", path.display())]
    ReadArtifact {
        /// Path that could not be read.
        path: PathBuf,
        /// Underlying filesystem error.
        #[source]
        source: std::io::Error,
    },
    /// The artifact exceeded its startup bound.
    #[error("runtime artifact {} exceeds the {max}-byte limit", path.display())]
    ArtifactTooLarge {
        /// Artifact path.
        path: PathBuf,
        /// Maximum accepted bytes.
        max: u64,
    },
    /// The artifact was not valid JSON.
    #[error("runtime artifact contains invalid JSON: {0}")]
    InvalidArtifactJson(#[source] serde_json::Error),
    /// The artifact schema version is unsupported.
    #[error("unsupported runtime artifact schema version {actual}; expected {expected}")]
    UnsupportedArtifactVersion {
        /// Version found in the artifact.
        actual: u32,
        /// Version supported by this library.
        expected: u32,
    },
    /// A required artifact relationship was missing or incompatible.
    #[error("invalid runtime artifact: {0}")]
    InvalidArtifact(&'static str),
    /// The protobuf entry was malformed.
    #[error("invalid mail protobuf: {0}")]
    InvalidProtobuf(&'static str),
    /// A protobuf integer did not fit its destination type.
    #[error("mail protobuf integer is out of range")]
    IntegerOutOfRange,
    /// A required protobuf field was absent.
    #[error("mail protobuf is missing required field {0}")]
    MissingField(&'static str),
    /// The entry exceeded the accepted mail bound.
    #[error("mail entry exceeds the {max}-byte limit")]
    MailTooLarge {
        /// Maximum accepted mail bytes.
        max: usize,
    },
    /// The mail type has not yet received a verified reconstruction.
    #[error("mail type is not yet supported for reconstruction: {0}")]
    UnsupportedMailType(String),
    /// Neither the entry nor its connection context supplied a server ID.
    #[error("mail entry and connection context are both missing a server ID")]
    MissingServerId,
    /// The compressed body did not declare a usable output length.
    #[error("compressed mail body has an invalid declared length")]
    InvalidInflatedLength,
    /// The compressed body could not be inflated.
    #[error("mail body could not be inflated: {0}")]
    Inflate(#[source] std::io::Error),
    /// The inflated length differed from the protobuf declaration.
    #[error("mail body inflated to {actual} bytes; expected {expected}")]
    InflatedLengthMismatch {
        /// Declared byte length.
        expected: usize,
        /// Actual byte length.
        actual: usize,
    },
    /// The primary body was not valid JSON.
    #[error("mail body contains invalid JSON: {0}")]
    InvalidBodyJson(#[source] serde_json::Error),
    /// An attack body was not valid JSON.
    #[error("mail attack body contains invalid JSON: {0}")]
    InvalidAttackJson(#[source] serde_json::Error),
    /// Split attack bodies could not be merged into the expected body shape.
    #[error("mail body is missing its Attacks object")]
    MissingAttacksObject,
    /// A category adapter received an incompatible decoded object.
    #[error("decoded mail body has an incompatible object shape: {0}")]
    InvalidBodyShape(&'static str),
    /// The reconstructed value could not be represented as a Persistent.Mail file.
    #[error("failed to encode persistent mail: {0}")]
    PersistentEncoding(#[source] rokbattles_mail_encoder::EncodeError),
}
