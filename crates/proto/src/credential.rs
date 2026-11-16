//! Credential signing and offline verification, and the device's proof of
//! possession at join (digest §5.5, §5.8, §8 item 3, §4.3 step 4(a); DU-31).
//!
//! - The account service signs a [`CredentialPayload`] with a
//!   [`ServiceSigningKey`] ([`sign_credential`]).
//! - The host verifies the credential text offline against a [`KeyList`] of
//!   service public keys compiled into the app ([`verify_credential`]).
//! - The device signs [`proof_message`] with its [`DeviceSigningKey`]
//!   ([`sign_join_proof`]); the host checks it against the credential's
//!   `device_pubkey` ([`verify_join_proof`]).
//! - [`evaluate_identity`] is the host's whole decision: a verified identity or
//!   "treat as guest" with a reason. A failure never rejects the join.
//!
//! Ed25519 throughout; verification is strict (non-canonical signatures and
//! small-order keys are rejected). No I/O, no clock reads, no async: callers pass
//! the current time and a random source, so the module runs unchanged on phones.

use std::borrow::Cow;
use std::fmt;

use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use serde::Deserialize;

use crate::encoding::b64_decode;
use crate::model::AccountId;
use crate::service::{
    proof_message, proof_timestamp_in_window, Credential, CredentialPayload, DevicePublicKey,
    JoinProof, Signature, CREDENTIAL_VERSION, DEVICE_PUBKEY_BYTES,
};
use crate::transport::{EventId, Fingerprint};

/// Length of an Ed25519 secret key seed.
pub const SECRET_KEY_BYTES: usize = 32;

/// A credential issued more than this far ahead of the host's clock is not yet
/// valid: 5 minutes.
pub const CREDENTIAL_CLOCK_SKEW_MS: i64 = 5 * 60 * 1000;

/// A source of cryptographically secure random bytes, supplied by the caller
/// (for example the operating system's generator). Any `FnMut(&mut [u8])` is one.
pub trait RandomSource {
    /// Fills `dest` with random bytes.
    fn fill_bytes(&mut self, dest: &mut [u8]);
}

impl<F: FnMut(&mut [u8])> RandomSource for F {
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self(dest)
    }
}

/// Builds a signing key from fresh random bytes and clears the local seed copy.
fn generate_signing_key<R: RandomSource + ?Sized>(rng: &mut R) -> SigningKey {
    let mut seed = [0u8; SECRET_KEY_BYTES];
    rng.fill_bytes(&mut seed);
    let key = SigningKey::from_bytes(&seed);
    seed.fill(0);
    // Keeps the compiler from removing the clearing write as dead.
    std::hint::black_box(&seed);
    key
}

/// The account service's signing key and its key ID (`kid`).
///
/// The secret is zeroed on drop, never printed by `Debug`, and has no serde
/// implementation.
pub struct ServiceSigningKey {
    kid: String,
    key: SigningKey,
}

impl ServiceSigningKey {
    /// Generates a new key with the given `kid` from the caller's random source.
    pub fn generate<R: RandomSource + ?Sized>(kid: impl Into<String>, rng: &mut R) -> Self {
        ServiceSigningKey {
            kid: kid.into(),
            key: generate_signing_key(rng),
        }
    }

    /// Loads a key from its 32-byte secret seed (as stored by the service).
    pub fn from_secret_bytes(kid: impl Into<String>, secret: &[u8; SECRET_KEY_BYTES]) -> Self {
        ServiceSigningKey {
            kid: kid.into(),
            key: SigningKey::from_bytes(secret),
        }
    }

    /// The development signing key, whose public key is the only entry of
    /// [`DEVELOPMENT_KEY_LIST`]. For tests and local runs only: its secret is in
    /// the source code.
    pub fn development() -> Self {
        Self::from_secret_bytes(DEVELOPMENT_KID, &DEVELOPMENT_SECRET)
    }

    /// The key ID.
    pub fn kid(&self) -> &str {
        &self.kid
    }

    /// The public half, as a key list entry.
    pub fn public_key(&self) -> ServicePublicKey {
        ServicePublicKey::new(self.kid.clone(), self.key.verifying_key().to_bytes())
    }

    /// The 32-byte secret seed, for storing the key. Handle with care.
    pub fn as_secret_bytes(&self) -> &[u8; SECRET_KEY_BYTES] {
        self.key.as_bytes()
    }
}

impl fmt::Debug for ServiceSigningKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServiceSigningKey")
            .field("kid", &self.kid)
            .field(
                "public_key",
                &DevicePublicKey::from_bytes(self.key.verifying_key().to_bytes()),
            )
            .finish_non_exhaustive()
    }
}

/// A device's signing key. Its public half goes into the credential
/// (`device_pubkey`) and it signs the join proof.
///
/// The secret is zeroed on drop, never printed by `Debug`, and has no serde
/// implementation.
pub struct DeviceSigningKey {
    key: SigningKey,
}

impl DeviceSigningKey {
    /// Generates a new key from the caller's random source.
    pub fn generate<R: RandomSource + ?Sized>(rng: &mut R) -> Self {
        DeviceSigningKey {
            key: generate_signing_key(rng),
        }
    }

    /// Loads a key from its 32-byte secret seed (as stored on the device).
    pub fn from_secret_bytes(secret: &[u8; SECRET_KEY_BYTES]) -> Self {
        DeviceSigningKey {
            key: SigningKey::from_bytes(secret),
        }
    }

    /// The public key, as sent in `device_pubkey`.
    pub fn public_key(&self) -> DevicePublicKey {
        DevicePublicKey::from_bytes(self.key.verifying_key().to_bytes())
    }

    /// The 32-byte secret seed, for storing the key. Handle with care.
    pub fn as_secret_bytes(&self) -> &[u8; SECRET_KEY_BYTES] {
        self.key.as_bytes()
    }
}

impl fmt::Debug for DeviceSigningKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeviceSigningKey")
            .field("public_key", &self.public_key())
            .finish_non_exhaustive()
    }
}

/// One service public key in a [`KeyList`].
#[derive(Clone, PartialEq, Eq)]
pub struct ServicePublicKey {
    kid: Cow<'static, str>,
    key: [u8; DEVICE_PUBKEY_BYTES],
}

impl ServicePublicKey {
    /// An entry from static data, usable in a `const` key list.
    pub const fn from_static(kid: &'static str, key: [u8; DEVICE_PUBKEY_BYTES]) -> Self {
        ServicePublicKey {
            kid: Cow::Borrowed(kid),
            key,
        }
    }

    /// An entry built at run time.
    pub fn new(kid: impl Into<String>, key: [u8; DEVICE_PUBKEY_BYTES]) -> Self {
        ServicePublicKey {
            kid: Cow::Owned(kid.into()),
            key,
        }
    }

    /// The key ID.
    pub fn kid(&self) -> &str {
        &self.kid
    }

    /// The raw Ed25519 public key.
    pub fn key_bytes(&self) -> &[u8; DEVICE_PUBKEY_BYTES] {
        &self.key
    }
}

impl fmt::Debug for ServicePublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ServicePublicKey({}, {})",
            self.kid,
            DevicePublicKey::from_bytes(self.key)
        )
    }
}

/// The service public keys a host accepts, looked up by `kid`. The host app
/// compiles its list in; rotating a key is an app update (§5.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyList {
    keys: Cow<'static, [ServicePublicKey]>,
}

impl KeyList {
    /// A key list over static data, usable in a `const`.
    pub const fn from_static(keys: &'static [ServicePublicKey]) -> Self {
        KeyList {
            keys: Cow::Borrowed(keys),
        }
    }

    /// A key list built at run time.
    pub fn new(keys: Vec<ServicePublicKey>) -> Self {
        KeyList {
            keys: Cow::Owned(keys),
        }
    }

    /// The entry for `kid`; the first one if the list repeats a `kid`.
    pub fn get(&self, kid: &str) -> Option<&ServicePublicKey> {
        self.keys.iter().find(|k| k.kid() == kid)
    }

    /// All entries, in order.
    pub fn keys(&self) -> &[ServicePublicKey] {
        &self.keys
    }
}

/// `kid` of the development service key.
pub const DEVELOPMENT_KID: &str = "dev-1";

/// Secret seed of the development service key. Public on purpose: it exists so
/// tests and local runs of the service and host agree without real secrets.
const DEVELOPMENT_SECRET: [u8; SECRET_KEY_BYTES] = *b"locale dev service key, not prod";

/// The development key list: the public half of
/// [`ServiceSigningKey::development`] and nothing else. For tests and local runs;
/// a production host must never include it.
pub static DEVELOPMENT_KEY_LIST: KeyList = KeyList::from_static(&DEVELOPMENT_KEYS);

static DEVELOPMENT_KEYS: [ServicePublicKey; 1] = [ServicePublicKey::from_static(
    DEVELOPMENT_KID,
    DEVELOPMENT_PUBLIC,
)];

/// Public half of `DEVELOPMENT_SECRET`
/// (`Keoa0vi2hNTSPKaLIsfhpWJNqMM9ANmKBH9qbCYoc-8`).
const DEVELOPMENT_PUBLIC: [u8; DEVICE_PUBKEY_BYTES] = [
    0x29, 0xea, 0x1a, 0xd2, 0xf8, 0xb6, 0x84, 0xd4, 0xd2, 0x3c, 0xa6, 0x8b, 0x22, 0xc7, 0xe1, 0xa5,
    0x62, 0x4d, 0xa8, 0xc3, 0x3d, 0x00, 0xd9, 0x8a, 0x04, 0x7f, 0x6a, 0x6c, 0x26, 0x28, 0x73, 0xef,
];

/// Signs `payload` with the service key: Ed25519 over the ASCII bytes
/// `v1.<payload segment>`, the segment being the compact JSON of `payload` in
/// declared field order, base64url without padding.
///
/// `payload.kid` must be `signing_key.kid()`, or no host will accept the result.
pub fn sign_credential(payload: CredentialPayload, signing_key: &ServiceSigningKey) -> Credential {
    debug_assert_eq!(payload.kid, signing_key.kid, "payload kid differs from key");
    let segment = Credential::payload_segment_for(&payload);
    let signature = signing_key
        .key
        .sign(&Credential::signing_input_for(&segment));
    Credential::from_signed(payload, Signature::from_bytes(signature.to_bytes()))
}

/// Why a credential did not verify.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CredentialError {
    /// The text is not `v1.<base64url JSON payload>.<base64url signature>` with a
    /// well-formed payload.
    #[error("malformed credential: {0}")]
    Malformed(&'static str),
    /// The `kid` is not in the host's key list.
    #[error("credential signed by an unknown key")]
    UnknownKid,
    /// The signature does not verify under the key named by `kid`.
    #[error("credential signature does not verify")]
    BadSignature,
    /// `expires_at` is at or before the host's clock.
    #[error("credential expired")]
    Expired,
    /// `issued_at` is more than [`CREDENTIAL_CLOCK_SKEW_MS`] ahead of the host's clock.
    #[error("credential not yet valid")]
    NotYetValid,
    /// The signed payload names a role other than `verified`.
    #[error("credential role is not verified")]
    WrongRole,
}

impl CredentialError {
    /// Stable snake_case code for the event log and telemetry.
    pub const fn as_str(&self) -> &'static str {
        match self {
            CredentialError::Malformed(_) => "credential_malformed",
            CredentialError::UnknownKid => "credential_unknown_kid",
            CredentialError::BadSignature => "credential_bad_signature",
            CredentialError::Expired => "credential_expired",
            CredentialError::NotYetValid => "credential_not_yet_valid",
            CredentialError::WrongRole => "credential_wrong_role",
        }
    }
}

/// Strict Ed25519 verification: rejects keys that are not valid points,
/// small-order keys and `R` values, and non-canonical `s`.
fn verify_strict(
    public_key: &[u8; DEVICE_PUBKEY_BYTES],
    message: &[u8],
    signature: &Signature,
) -> bool {
    let Ok(key) = VerifyingKey::from_bytes(public_key) else {
        return false;
    };
    let signature = ed25519_dalek::Signature::from_bytes(signature.as_bytes());
    key.verify_strict(message, &signature).is_ok()
}

/// Verifies credential text offline against `key_list` at host time `now_ms`.
///
/// Checks, in order: structure, `kid` in the list, signature over the payload
/// segment exactly as received (never re-serialised), role, expiry
/// (`expires_at <= now` fails), and issue time (more than 5 minutes ahead fails).
pub fn verify_credential(
    text: &str,
    key_list: &KeyList,
    now_ms: i64,
) -> Result<CredentialPayload, CredentialError> {
    #[derive(Deserialize)]
    struct KidOnly {
        kid: String,
    }
    #[derive(Deserialize)]
    struct RoleOnly {
        role: String,
    }

    let mut parts = text.split('.');
    let (Some(version), Some(segment), Some(sig), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(CredentialError::Malformed(
            "expected three dot-separated parts",
        ));
    };
    if version != CREDENTIAL_VERSION {
        return Err(CredentialError::Malformed("unsupported version"));
    }
    let json = b64_decode(segment).ok_or(CredentialError::Malformed("payload is not base64url"))?;
    let signature: Signature = sig
        .parse()
        .map_err(|_| CredentialError::Malformed("signature is not 64 bytes of base64url"))?;
    let KidOnly { kid } = serde_json::from_slice(&json)
        .map_err(|_| CredentialError::Malformed("payload has no kid"))?;

    let key = key_list.get(&kid).ok_or(CredentialError::UnknownKid)?;
    if !verify_strict(
        key.key_bytes(),
        &Credential::signing_input_for(segment),
        &signature,
    ) {
        return Err(CredentialError::BadSignature);
    }

    let credential = match Credential::decode(text) {
        Ok(c) => c,
        Err(_) => {
            return Err(match serde_json::from_slice::<RoleOnly>(&json) {
                Ok(r) if r.role != crate::service::CredentialRole::Verified.as_str() => {
                    CredentialError::WrongRole
                }
                _ => CredentialError::Malformed("payload is not a credential payload"),
            });
        }
    };
    let payload = credential.payload().clone();
    if payload.expires_at <= now_ms {
        return Err(CredentialError::Expired);
    }
    if payload.issued_at > now_ms.saturating_add(CREDENTIAL_CLOCK_SKEW_MS) {
        return Err(CredentialError::NotYetValid);
    }
    Ok(payload)
}

/// Signs the join proof for `event_id` and the pinned certificate `fingerprint`
/// at `timestamp_ms`: Ed25519 over [`proof_message`].
pub fn sign_join_proof(
    device_key: &DeviceSigningKey,
    event_id: &EventId,
    fingerprint: &Fingerprint,
    timestamp_ms: i64,
) -> JoinProof {
    let signature = device_key
        .key
        .sign(&proof_message(event_id, fingerprint, timestamp_ms));
    JoinProof {
        timestamp: timestamp_ms,
        signature: Signature::from_bytes(signature.to_bytes()),
    }
}

/// Why a join proof did not verify.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ProofError {
    /// The signature does not verify under the credential's `device_pubkey` for
    /// this event and certificate.
    #[error("join proof signature does not verify")]
    BadSignature,
    /// The timestamp is more than 5 minutes behind the host's clock.
    #[error("join proof timestamp is too old")]
    Stale,
    /// The timestamp is more than 5 minutes ahead of the host's clock.
    #[error("join proof timestamp is in the future")]
    Future,
}

impl ProofError {
    /// Stable snake_case code for the event log and telemetry.
    pub const fn as_str(&self) -> &'static str {
        match self {
            ProofError::BadSignature => "proof_bad_signature",
            ProofError::Stale => "proof_stale",
            ProofError::Future => "proof_future",
        }
    }
}

/// Verifies a join proof against the verified credential `payload`: the
/// signature by `payload.device_pubkey` over [`proof_message`] for this event and
/// certificate, then the timestamp within ±5 minutes of `now_ms` (inclusive,
/// [`crate::service::PROOF_WINDOW_MS`]).
pub fn verify_join_proof(
    payload: &CredentialPayload,
    proof: &JoinProof,
    event_id: &EventId,
    fingerprint: &Fingerprint,
    now_ms: i64,
) -> Result<(), ProofError> {
    let message = proof_message(event_id, fingerprint, proof.timestamp);
    if !verify_strict(payload.device_pubkey.as_bytes(), &message, &proof.signature) {
        return Err(ProofError::BadSignature);
    }
    if !proof_timestamp_in_window(now_ms, proof.timestamp) {
        return Err(if proof.timestamp < now_ms {
            ProofError::Stale
        } else {
            ProofError::Future
        });
    }
    Ok(())
}

/// An attendee identity the host verified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedIdentity {
    /// Account ID.
    pub account_id: AccountId,
    /// Display name from the credential.
    pub display_name: String,
    /// The device key that signed the proof.
    pub device_pubkey: DevicePublicKey,
}

/// Why the host treats a joining device as a guest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GuestReason {
    /// No credential was sent.
    #[error("no credential")]
    NoCredential,
    /// The credential did not verify.
    #[error(transparent)]
    Credential(CredentialError),
    /// A credential was sent without a proof.
    #[error("credential sent without a join proof")]
    MissingProof,
    /// The proof did not verify.
    #[error(transparent)]
    Proof(ProofError),
}

impl GuestReason {
    /// Stable snake_case code for the event log and telemetry.
    pub const fn as_str(&self) -> &'static str {
        match self {
            GuestReason::NoCredential => "no_credential",
            GuestReason::Credential(e) => e.as_str(),
            GuestReason::MissingProof => "proof_missing",
            GuestReason::Proof(e) => e.as_str(),
        }
    }
}

/// The host's identity decision for a join.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityDecision {
    /// Credential and proof verified.
    Verified(VerifiedIdentity),
    /// Treat as a guest; the reason is for the event log and telemetry only.
    Guest(GuestReason),
}

/// The host's decision at Access Validation step (a) (§4.3 step 4(a), §8 item 3):
/// verify the credential, then the proof of possession. Every failure means
/// guest, never a rejected join.
pub fn evaluate_identity(
    credential_text: Option<&str>,
    proof: Option<&JoinProof>,
    key_list: &KeyList,
    event_id: &EventId,
    fingerprint: &Fingerprint,
    now_ms: i64,
) -> IdentityDecision {
    let Some(text) = credential_text else {
        return IdentityDecision::Guest(GuestReason::NoCredential);
    };
    let payload = match verify_credential(text, key_list, now_ms) {
        Ok(p) => p,
        Err(e) => return IdentityDecision::Guest(GuestReason::Credential(e)),
    };
    let Some(proof) = proof else {
        return IdentityDecision::Guest(GuestReason::MissingProof);
    };
    if let Err(e) = verify_join_proof(&payload, proof, event_id, fingerprint, now_ms) {
        return IdentityDecision::Guest(GuestReason::Proof(e));
    }
    IdentityDecision::Verified(VerifiedIdentity {
        account_id: payload.account_id,
        display_name: payload.display_name,
        device_pubkey: payload.device_pubkey,
    })
}
