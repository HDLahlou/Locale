//! Parse errors for the text forms defined in this crate.

/// A value did not match the format the contract fixes.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    /// Not `evt_` followed by 12 lowercase hex digits.
    #[error("invalid event ID: {0:?}")]
    EventId(String),
    /// Not 16 bytes in unpadded base64url.
    #[error("invalid join token")]
    JoinToken,
    /// Not 32 bytes in unpadded base64url.
    #[error("invalid certificate fingerprint")]
    Fingerprint,
    /// Not 32 bytes in unpadded base64url.
    #[error("invalid device public key")]
    DevicePublicKey,
    /// Not 64 bytes in unpadded base64url.
    #[error("invalid signature")]
    Signature,
    /// The join URL does not match `http://<ip>:<port>/e/<event-id>#t=<token>&fp=<fp>`.
    #[error("invalid join URL: {0}")]
    JoinUrl(&'static str),
    /// The deep link does not match `locale://join?u=<url-encoded join URL>`.
    #[error("invalid deep link: {0}")]
    DeepLink(&'static str),
    /// The credential text does not match `v1.<payload>.<signature>`.
    #[error("invalid credential: {0}")]
    Credential(&'static str),
    /// A TXT record is missing a key or has a malformed value.
    #[error("invalid TXT record: {0}")]
    TxtRecord(&'static str),
    /// A string is not one of the values of a closed enum.
    #[error("unknown {kind} value: {value:?}")]
    UnknownValue {
        /// The enum's type name.
        kind: &'static str,
        /// The value that was received.
        value: String,
    },
}
