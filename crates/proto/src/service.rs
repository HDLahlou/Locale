//! App service API, credential format and proof of possession (digest §5.5, DU-31).
//!
//! This module fixes text and byte formats only. Signing and verification are
//! done elsewhere (task M1-2); [`Credential::decode`] checks structure, not the
//! signature.

use std::fmt;
use std::str::FromStr;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::encoding::{b64_decode, b64_decode_array, b64_encode, string_serde};
use crate::model::AccountId;
use crate::transport::{EventId, Fingerprint};
use crate::ParseError;

/// Service route: request a sign-in code.
pub const AUTH_START: &str = "/v1/auth/start";
/// Service route: verify a code, get account, credential and refresh token.
pub const AUTH_VERIFY: &str = "/v1/auth/verify";
/// Service route: refresh the credential.
pub const CREDENTIAL_REFRESH: &str = "/v1/credential/refresh";
/// Service route: change the display name.
pub const ACCOUNT: &str = "/v1/account";
/// Service route: email the desktop download link.
pub const HOST_LINK: &str = "/v1/host-link";

/// Requests that act on an account (`PATCH /v1/account`, `POST /v1/host-link`)
/// authenticate with `Authorization: Bearer <refresh_token>`.
pub const SERVICE_AUTH_SCHEME: &str = "Bearer <refresh_token>";

/// One row of the service API table, for the contract document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceEndpoint {
    /// HTTP method.
    pub method: &'static str,
    /// Path.
    pub path: &'static str,
    /// Request body type.
    pub request: &'static str,
    /// Success status and response body type.
    pub response: &'static str,
}

/// The service API table (§5.5), in digest order.
pub const SERVICE_ENDPOINTS: &[ServiceEndpoint] = &[
    ServiceEndpoint {
        method: "POST",
        path: AUTH_START,
        request: "AuthStartRequest",
        response: "204; emails a 6-digit code valid 10 min",
    },
    ServiceEndpoint {
        method: "POST",
        path: AUTH_VERIFY,
        request: "AuthVerifyRequest",
        response: "200 AuthVerifyResponse",
    },
    ServiceEndpoint {
        method: "POST",
        path: CREDENTIAL_REFRESH,
        request: "CredentialRefreshRequest",
        response: "200 CredentialRefreshResponse",
    },
    ServiceEndpoint {
        method: "PATCH",
        path: ACCOUNT,
        request: "AccountPatchRequest; Authorization: Bearer <refresh_token>",
        response: "200 AccountPatchResponse",
    },
    ServiceEndpoint {
        method: "POST",
        path: HOST_LINK,
        request: "no body; Authorization: Bearer <refresh_token>",
        response: "204; emails the desktop download link to the account's address",
    },
];

/// Length of an Ed25519 public key.
pub const DEVICE_PUBKEY_BYTES: usize = 32;
/// Length of an Ed25519 signature.
pub const SIGNATURE_BYTES: usize = 64;

/// A device's Ed25519 public key, base64url without padding.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct DevicePublicKey([u8; DEVICE_PUBKEY_BYTES]);

impl DevicePublicKey {
    /// Wraps raw key bytes.
    pub const fn from_bytes(bytes: [u8; DEVICE_PUBKEY_BYTES]) -> Self {
        DevicePublicKey(bytes)
    }
    /// The raw key bytes.
    pub const fn as_bytes(&self) -> &[u8; DEVICE_PUBKEY_BYTES] {
        &self.0
    }
}

impl fmt::Debug for DevicePublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DevicePublicKey({self})")
    }
}
impl fmt::Display for DevicePublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&b64_encode(&self.0))
    }
}
impl FromStr for DevicePublicKey {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        b64_decode_array(s)
            .map(DevicePublicKey)
            .ok_or(ParseError::DevicePublicKey)
    }
}
string_serde!(
    DevicePublicKey,
    "DevicePublicKey",
    "^[A-Za-z0-9_-]{43}$",
    "Ed25519 public key, base64url without padding."
);

/// An Ed25519 signature, base64url without padding.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Signature([u8; SIGNATURE_BYTES]);

impl Signature {
    /// Wraps raw signature bytes.
    pub const fn from_bytes(bytes: [u8; SIGNATURE_BYTES]) -> Self {
        Signature(bytes)
    }
    /// The raw signature bytes.
    pub const fn as_bytes(&self) -> &[u8; SIGNATURE_BYTES] {
        &self.0
    }
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Signature({self})")
    }
}
impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&b64_encode(&self.0))
    }
}
impl FromStr for Signature {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        b64_decode_array(s)
            .map(Signature)
            .ok_or(ParseError::Signature)
    }
}
string_serde!(
    Signature,
    "Signature",
    "^[A-Za-z0-9_-]{86}$",
    "Ed25519 signature, base64url without padding."
);

/// An account on the app service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Account {
    /// Account ID.
    pub id: AccountId,
    /// Display name.
    pub display_name: String,
    /// Email address.
    pub email: String,
    /// Creation time.
    pub created_at: i64,
}

/// `POST /v1/auth/start` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AuthStartRequest {
    /// Email address to send the code to.
    pub email: String,
}

/// `POST /v1/auth/verify` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AuthVerifyRequest {
    /// Email address.
    pub email: String,
    /// The 6-digit code.
    pub code: String,
    /// Display name, for a new account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// The device's public key, named in the credential.
    pub device_pubkey: DevicePublicKey,
}

/// `POST /v1/auth/verify` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AuthVerifyResponse {
    /// The account.
    pub account: Account,
    /// A fresh credential.
    pub credential: Credential,
    /// Refresh token for this device.
    pub refresh_token: String,
}

/// `POST /v1/credential/refresh` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CredentialRefreshRequest {
    /// Refresh token.
    pub refresh_token: String,
    /// The device's public key; must match the refresh token's.
    pub device_pubkey: DevicePublicKey,
}

/// `POST /v1/credential/refresh` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CredentialRefreshResponse {
    /// A fresh credential.
    pub credential: Credential,
}

/// `PATCH /v1/account` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AccountPatchRequest {
    /// New display name.
    pub display_name: String,
}

/// `PATCH /v1/account` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AccountPatchResponse {
    /// The updated account.
    pub account: Account,
    /// A credential carrying the new display name.
    pub credential: Credential,
}

closed_enum! {
    /// The role a credential grants. Closed.
    pub enum CredentialRole {
        /// A verified account holder.
        Verified = "verified",
    }
}

/// The signed payload of a credential.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CredentialPayload {
    /// ID of the service signing key, looked up in the host's key list.
    pub kid: String,
    /// Account ID.
    pub account_id: AccountId,
    /// Display name.
    pub display_name: String,
    /// Always `verified`.
    pub role: CredentialRole,
    /// The device key that must sign the join proof.
    pub device_pubkey: DevicePublicKey,
    /// Issue time.
    pub issued_at: i64,
    /// Expiry.
    pub expires_at: i64,
}

/// Version prefix of the credential text.
pub const CREDENTIAL_VERSION: &str = "v1";

/// An account credential: `v1.<base64url(payload JSON)>.<base64url(Ed25519 signature)>`,
/// base64url without padding.
///
/// The signature covers the ASCII bytes `v1.<base64url(payload JSON)>`
/// ([`Credential::signing_input`]), so the exact payload segment received is kept
/// and re-encoded unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credential {
    payload: CredentialPayload,
    payload_segment: String,
    signature: Signature,
}

impl Credential {
    /// The base64url payload segment for `payload` (compact JSON, field order as declared).
    pub fn payload_segment_for(payload: &CredentialPayload) -> String {
        // Serialising a plain struct of strings and integers cannot fail.
        let json = serde_json::to_vec(payload).unwrap_or_default();
        b64_encode(&json)
    }

    /// The bytes the service signs for a payload segment: `v1.` followed by the segment.
    pub fn signing_input_for(payload_segment: &str) -> Vec<u8> {
        format!("{CREDENTIAL_VERSION}.{payload_segment}").into_bytes()
    }

    /// Builds a credential from a payload and the signature over
    /// `signing_input_for(&payload_segment_for(&payload))`.
    pub fn from_signed(payload: CredentialPayload, signature: Signature) -> Self {
        let payload_segment = Self::payload_segment_for(&payload);
        Credential {
            payload,
            payload_segment,
            signature,
        }
    }

    /// Splits and validates the three parts. Does not check the signature.
    pub fn decode(text: &str) -> Result<Self, ParseError> {
        let mut parts = text.split('.');
        let (Some(version), Some(segment), Some(sig), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(ParseError::Credential("expected three dot-separated parts"));
        };
        if version != CREDENTIAL_VERSION {
            return Err(ParseError::Credential("unsupported version"));
        }
        let json = b64_decode(segment).ok_or(ParseError::Credential("payload is not base64url"))?;
        let payload: CredentialPayload = serde_json::from_slice(&json)
            .map_err(|_| ParseError::Credential("payload is not a credential payload"))?;
        let signature = sig
            .parse()
            .map_err(|_| ParseError::Credential("signature is not 64 bytes of base64url"))?;
        Ok(Credential {
            payload,
            payload_segment: segment.to_owned(),
            signature,
        })
    }

    /// The text form.
    pub fn encode(&self) -> String {
        format!(
            "{CREDENTIAL_VERSION}.{}.{}",
            self.payload_segment, self.signature
        )
    }

    /// The decoded payload.
    pub fn payload(&self) -> &CredentialPayload {
        &self.payload
    }

    /// The payload segment exactly as received or built.
    pub fn payload_segment(&self) -> &str {
        &self.payload_segment
    }

    /// The signature.
    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    /// The bytes the signature covers.
    pub fn signing_input(&self) -> Vec<u8> {
        Self::signing_input_for(&self.payload_segment)
    }
}

impl fmt::Display for Credential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.encode())
    }
}

impl FromStr for Credential {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Credential::decode(s)
    }
}

string_serde!(
    Credential,
    "Credential",
    "^v1\\.[A-Za-z0-9_-]+\\.[A-Za-z0-9_-]{86}$",
    "Account credential: v1.<base64url(payload JSON)>.<base64url(Ed25519 signature)>."
);

/// Domain tag at the start of the proof message.
pub const PROOF_DOMAIN: &[u8] = b"locale-join-v1";

/// A proof timestamp is accepted within this distance of the host's clock: ±5 minutes.
pub const PROOF_WINDOW_MS: i64 = 5 * 60 * 1000;

/// Length of [`proof_message`]: 14 + 16 + 32 + 8 bytes.
pub const PROOF_MESSAGE_LEN: usize = PROOF_DOMAIN.len() + 16 + 32 + 8;

/// The exact bytes the device key signs for `proof` in `POST /join`:
/// `"locale-join-v1" ‖ event_id ‖ cert_fingerprint ‖ timestamp`, where
/// - `"locale-join-v1"` is 14 ASCII bytes,
/// - `event_id` is the 16 ASCII bytes of the event ID (`evt_` + 12 hex),
/// - `cert_fingerprint` is the 32 raw bytes of the SHA-256 digest (not its text form),
/// - `timestamp` is Unix milliseconds as an 8-byte big-endian signed integer.
///
/// Every part has a fixed length, so the concatenation is unambiguous.
pub fn proof_message(
    event_id: &EventId,
    cert_fingerprint: &Fingerprint,
    timestamp_ms: i64,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(PROOF_MESSAGE_LEN);
    out.extend_from_slice(PROOF_DOMAIN);
    out.extend_from_slice(event_id.as_str().as_bytes());
    out.extend_from_slice(cert_fingerprint.as_bytes());
    out.extend_from_slice(&timestamp_ms.to_be_bytes());
    out
}

/// Whether a proof timestamp is within [`PROOF_WINDOW_MS`] of `now_ms` (inclusive).
pub fn proof_timestamp_in_window(now_ms: i64, timestamp_ms: i64) -> bool {
    now_ms.abs_diff(timestamp_ms) <= PROOF_WINDOW_MS as u64
}

/// The `proof` field of `POST /join`: the signing time and the device key's
/// signature over [`proof_message`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct JoinProof {
    /// Unix milliseconds when the proof was made.
    pub timestamp: i64,
    /// Ed25519 signature over [`proof_message`].
    pub signature: Signature,
}
