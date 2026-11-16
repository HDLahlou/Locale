//! Credential signing and offline verification, and the join proof of
//! possession (task M1-2; digest §5.5, §5.8, §8 item 3, §4.3 step 4(a)).

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use proto::credential::{
    evaluate_identity, sign_credential, sign_join_proof, verify_credential, verify_join_proof,
    CredentialError, DeviceSigningKey, GuestReason, IdentityDecision, KeyList, ProofError,
    ServicePublicKey, ServiceSigningKey, VerifiedIdentity, CREDENTIAL_CLOCK_SKEW_MS,
    DEVELOPMENT_KEY_LIST, DEVELOPMENT_KID,
};
use proto::service::{
    Credential, CredentialPayload, CredentialRole, DevicePublicKey, JoinProof, Signature,
    PROOF_WINDOW_MS,
};
use proto::transport::{EventId, Fingerprint};

const NOW: i64 = 1_763_000_000_000;
const DAY_MS: i64 = 24 * 60 * 60 * 1000;
const KID: &str = "test-1";
const SERVICE_SEED: [u8; 32] = [7; 32];
const DEVICE_SEED: [u8; 32] = [9; 32];

fn b64(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

fn service_key() -> ServiceSigningKey {
    ServiceSigningKey::from_secret_bytes(KID, &SERVICE_SEED)
}

fn device_key() -> DeviceSigningKey {
    DeviceSigningKey::from_secret_bytes(&DEVICE_SEED)
}

fn key_list() -> KeyList {
    KeyList::new(vec![service_key().public_key()])
}

fn event_id() -> EventId {
    EventId::parse("evt_0123456789ab").unwrap()
}

fn fingerprint() -> Fingerprint {
    Fingerprint::from_digest([0x42; 32])
}

fn payload() -> CredentialPayload {
    CredentialPayload {
        kid: KID.to_owned(),
        account_id: "7f1c2e9a-3b4d-4e5f-8a6b-1c2d3e4f5a6b".to_owned(),
        display_name: "Ada".to_owned(),
        role: CredentialRole::Verified,
        device_pubkey: device_key().public_key(),
        issued_at: NOW - 1000,
        expires_at: NOW + 7 * DAY_MS,
    }
}

fn credential_text(p: CredentialPayload) -> String {
    sign_credential(p, &service_key()).encode()
}

fn proof_at(ts: i64) -> JoinProof {
    sign_join_proof(&device_key(), &event_id(), &fingerprint(), ts)
}

fn evaluate(text: &str, proof: Option<&JoinProof>) -> IdentityDecision {
    evaluate_identity(
        Some(text),
        proof,
        &key_list(),
        &event_id(),
        &fingerprint(),
        NOW,
    )
}

fn assert_guest(decision: IdentityDecision, reason: GuestReason) {
    assert_eq!(decision, IdentityDecision::Guest(reason));
}

fn cred_guest(e: CredentialError) -> GuestReason {
    GuestReason::Credential(e)
}

/// Replaces the payload segment of a credential, keeping its signature.
fn with_segment_json(text: &str, json: &str) -> String {
    let sig = text.rsplit('.').next().unwrap();
    format!("v1.{}.{sig}", b64(json.as_bytes()))
}

fn compact_json(p: &CredentialPayload) -> String {
    serde_json::to_string(p).unwrap()
}

#[test]
fn credential_valid_credential_and_proof_pass() {
    let text = credential_text(payload());
    assert_eq!(
        verify_credential(&text, &key_list(), NOW).unwrap(),
        payload()
    );
    let proof = proof_at(NOW);
    assert_eq!(
        evaluate(&text, Some(&proof)),
        IdentityDecision::Verified(VerifiedIdentity {
            account_id: payload().account_id,
            display_name: "Ada".to_owned(),
            device_pubkey: device_key().public_key(),
        })
    );
}

#[test]
fn credential_survives_decode_and_encode_unchanged() {
    let text = credential_text(payload());
    let reencoded = Credential::decode(&text).unwrap().encode();
    assert_eq!(reencoded, text);
    let via_json: Credential =
        serde_json::from_str(&serde_json::to_string(&text).unwrap()).unwrap();
    assert!(verify_credential(&via_json.encode(), &key_list(), NOW).is_ok());
}

#[test]
fn credential_signature_covers_v1_and_payload_segment() {
    let c = sign_credential(payload(), &service_key());
    let message = format!("v1.{}", b64(compact_json(&payload()).as_bytes()));
    let key =
        ed25519_dalek::VerifyingKey::from_bytes(service_key().public_key().key_bytes()).unwrap();
    let sig = ed25519_dalek::Signature::from_bytes(c.signature().as_bytes());
    key.verify_strict(message.as_bytes(), &sig).unwrap();
}

#[test]
fn credential_no_credential_is_guest() {
    let d = evaluate_identity(
        None,
        Some(&proof_at(NOW)),
        &key_list(),
        &event_id(),
        &fingerprint(),
        NOW,
    );
    assert_guest(d, GuestReason::NoCredential);
}

#[test]
fn credential_expired_is_guest() {
    let proof = proof_at(NOW);
    let mut p = payload();
    p.expires_at = NOW;
    assert_guest(
        evaluate(&credential_text(p.clone()), Some(&proof)),
        cred_guest(CredentialError::Expired),
    );
    p.expires_at = NOW - DAY_MS;
    assert_guest(
        evaluate(&credential_text(p.clone()), Some(&proof)),
        cred_guest(CredentialError::Expired),
    );
    p.expires_at = NOW + 1;
    assert!(matches!(
        evaluate(&credential_text(p), Some(&proof)),
        IdentityDecision::Verified(_)
    ));
}

#[test]
fn credential_issued_more_than_five_minutes_ahead_is_not_yet_valid() {
    let proof = proof_at(NOW);
    let mut p = payload();
    p.issued_at = NOW + CREDENTIAL_CLOCK_SKEW_MS + 1;
    assert_guest(
        evaluate(&credential_text(p.clone()), Some(&proof)),
        cred_guest(CredentialError::NotYetValid),
    );
    p.issued_at = NOW + CREDENTIAL_CLOCK_SKEW_MS;
    assert!(matches!(
        evaluate(&credential_text(p), Some(&proof)),
        IdentityDecision::Verified(_)
    ));
    assert_eq!(CREDENTIAL_CLOCK_SKEW_MS, 5 * 60 * 1000);
}

#[test]
fn credential_signed_by_key_not_in_list_is_guest() {
    let other = ServiceSigningKey::from_secret_bytes(KID, &[8; 32]);
    let text = sign_credential(payload(), &other).encode();
    assert_guest(
        evaluate(&text, Some(&proof_at(NOW))),
        cred_guest(CredentialError::BadSignature),
    );
}

#[test]
fn credential_unknown_kid_is_guest() {
    let other = ServiceSigningKey::from_secret_bytes("other-7", &SERVICE_SEED);
    let mut p = payload();
    p.kid = "other-7".to_owned();
    let text = sign_credential(p, &other).encode();
    assert_guest(
        evaluate(&text, Some(&proof_at(NOW))),
        cred_guest(CredentialError::UnknownKid),
    );
}

type Alteration = Box<dyn Fn(&mut CredentialPayload)>;

#[test]
fn credential_payload_altered_after_signing_is_guest() {
    let text = credential_text(payload());
    let proof = proof_at(NOW);
    let other_device = DeviceSigningKey::from_secret_bytes(&[3; 32]).public_key();
    let alterations: Vec<(&str, Alteration)> = vec![
        ("account_id", Box::new(|p| p.account_id.push('0'))),
        ("display_name", Box::new(|p| p.display_name = "Eve".into())),
        (
            "device_pubkey",
            Box::new(move |p| p.device_pubkey = other_device),
        ),
        ("issued_at", Box::new(|p| p.issued_at -= 1)),
        ("expires_at", Box::new(|p| p.expires_at += DAY_MS)),
    ];
    for (field, alter) in alterations {
        let mut p = payload();
        alter(&mut p);
        let forged = with_segment_json(&text, &compact_json(&p));
        assert_guest(
            evaluate(&forged, Some(&proof)),
            cred_guest(CredentialError::BadSignature),
        );
        assert_ne!(forged, text, "{field}");
    }
    // kid: a second key with the same public key under another kid.
    let mut list = key_list().keys().to_vec();
    list.push(ServicePublicKey::new(
        "test-2",
        *service_key().public_key().key_bytes(),
    ));
    let mut p = payload();
    p.kid = "test-2".into();
    let forged = with_segment_json(&text, &compact_json(&p));
    let d = evaluate_identity(
        Some(&forged),
        Some(&proof),
        &KeyList::new(list),
        &event_id(),
        &fingerprint(),
        NOW,
    );
    assert_guest(d, cred_guest(CredentialError::BadSignature));
    // role: the only other spelling the JSON can carry is a different string.
    let json = compact_json(&payload()).replace("\"role\":\"verified\"", "\"role\":\"admin\"");
    assert_guest(
        evaluate(&with_segment_json(&text, &json), Some(&proof)),
        cred_guest(CredentialError::BadSignature),
    );
}

#[test]
fn credential_signature_altered_is_guest() {
    let text = credential_text(payload());
    let c = Credential::decode(&text).unwrap();
    for i in [0usize, 31, 32, 63] {
        let mut bytes = *c.signature().as_bytes();
        bytes[i] ^= 0x01;
        let forged = format!(
            "v1.{}.{}",
            c.payload_segment(),
            Signature::from_bytes(bytes)
        );
        assert_guest(
            evaluate(&forged, Some(&proof_at(NOW))),
            cred_guest(CredentialError::BadSignature),
        );
    }
}

#[test]
fn credential_reencoded_with_different_json_is_guest() {
    let text = credential_text(payload());
    let proof = proof_at(NOW);
    let p = payload();
    let spaced = serde_json::to_string_pretty(&p).unwrap();
    let reordered = format!(
        "{{\"account_id\":\"{}\",\"kid\":\"{}\",\"display_name\":\"{}\",\"role\":\"verified\",\"device_pubkey\":\"{}\",\"issued_at\":{},\"expires_at\":{}}}",
        p.account_id, p.kid, p.display_name, p.device_pubkey, p.issued_at, p.expires_at
    );
    for json in [spaced, reordered] {
        // Same values: the payload still decodes to the original.
        let forged = with_segment_json(&text, &json);
        assert_eq!(Credential::decode(&forged).unwrap().payload(), &p);
        assert_guest(
            evaluate(&forged, Some(&proof)),
            cred_guest(CredentialError::BadSignature),
        );
    }
}

#[test]
fn credential_wrong_role_is_guest() {
    let json = compact_json(&payload()).replace("\"role\":\"verified\"", "\"role\":\"host\"");
    let segment = b64(json.as_bytes());
    let raw = ed25519_dalek::SigningKey::from_bytes(&SERVICE_SEED);
    let sig = ed25519_dalek::Signer::sign(&raw, format!("v1.{segment}").as_bytes());
    let text = format!("v1.{segment}.{}", b64(&sig.to_bytes()));
    assert_eq!(
        verify_credential(&text, &key_list(), NOW),
        Err(CredentialError::WrongRole)
    );
    assert_guest(
        evaluate(&text, Some(&proof_at(NOW))),
        cred_guest(CredentialError::WrongRole),
    );
}

#[test]
fn credential_malformed_text_is_guest() {
    let text = credential_text(payload());
    let (_, rest) = text.split_once('.').unwrap();
    let sig = text.rsplit('.').next().unwrap();
    let cases = [
        String::new(),
        "garbage".to_owned(),
        format!("v2.{rest}"),
        format!("{text}.extra"),
        format!("v1.{}.{sig}", b64(b"not json")),
        format!("v1.{}.{sig}", b64(b"{\"kid\":\"test-1\"")),
        format!("v1.!!!.{sig}"),
        format!("v1.{}.short", text.split('.').nth(1).unwrap()),
    ];
    for case in &cases {
        let d = evaluate(case, Some(&proof_at(NOW)));
        assert!(
            matches!(
                d,
                IdentityDecision::Guest(GuestReason::Credential(CredentialError::Malformed(_)))
            ),
            "{case:?}: {d:?}"
        );
    }
}

/// Little-endian bytes of the Ed25519 group order L.
const ORDER_L: [u8; 32] = [
    0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10,
];

/// The compressed base point B.
const BASE_POINT: [u8; 32] = [
    0x58, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
    0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
];

/// The identity point: a small-order public key.
const IDENTITY_POINT: [u8; 32] = {
    let mut b = [0u8; 32];
    b[0] = 1;
    b
};

/// With public key = identity, (R = B, s = 1) satisfies the cofactorless
/// equation for every message, so lax verification accepts it.
fn weak_key_signature() -> [u8; 64] {
    let mut sig = [0u8; 64];
    sig[..32].copy_from_slice(&BASE_POINT);
    sig[32] = 1;
    sig
}

#[test]
fn credential_non_canonical_signature_is_rejected() {
    let text = credential_text(payload());
    let c = Credential::decode(&text).unwrap();
    let mut bytes = *c.signature().as_bytes();
    let mut carry = 0u16;
    for i in 0..32 {
        let sum = bytes[32 + i] as u16 + ORDER_L[i] as u16 + carry;
        bytes[32 + i] = sum as u8;
        carry = sum >> 8;
    }
    assert_eq!(carry, 0);
    let forged = format!(
        "v1.{}.{}",
        c.payload_segment(),
        Signature::from_bytes(bytes)
    );
    assert_eq!(
        verify_credential(&forged, &key_list(), NOW),
        Err(CredentialError::BadSignature)
    );
}

#[test]
fn credential_small_order_service_key_is_rejected() {
    let mut p = payload();
    p.kid = "weak".into();
    let segment = Credential::payload_segment_for(&p);
    let sig = weak_key_signature();
    // Lax verification accepts this forgery; strict verification must not.
    let lax = ed25519_dalek::VerifyingKey::from_bytes(&IDENTITY_POINT).unwrap();
    assert!(ed25519_dalek::Verifier::verify(
        &lax,
        format!("v1.{segment}").as_bytes(),
        &ed25519_dalek::Signature::from_bytes(&sig)
    )
    .is_ok());
    let text = format!("v1.{segment}.{}", b64(&sig));
    let list = KeyList::new(vec![ServicePublicKey::new("weak", IDENTITY_POINT)]);
    assert_eq!(
        verify_credential(&text, &list, NOW),
        Err(CredentialError::BadSignature)
    );
}

#[test]
fn credential_proof_small_order_device_key_is_rejected() {
    let mut p = payload();
    p.device_pubkey = DevicePublicKey::from_bytes(IDENTITY_POINT);
    let proof = JoinProof {
        timestamp: NOW,
        signature: Signature::from_bytes(weak_key_signature()),
    };
    assert_eq!(
        verify_join_proof(&p, &proof, &event_id(), &fingerprint(), NOW),
        Err(ProofError::BadSignature)
    );
}

#[test]
fn credential_proof_by_different_device_key_is_guest() {
    let other = DeviceSigningKey::from_secret_bytes(&[3; 32]);
    let proof = sign_join_proof(&other, &event_id(), &fingerprint(), NOW);
    assert_guest(
        evaluate(&credential_text(payload()), Some(&proof)),
        GuestReason::Proof(ProofError::BadSignature),
    );
}

#[test]
fn credential_proof_for_different_event_is_guest() {
    let other_event = EventId::parse("evt_0123456789ac").unwrap();
    let proof = sign_join_proof(&device_key(), &other_event, &fingerprint(), NOW);
    assert_guest(
        evaluate(&credential_text(payload()), Some(&proof)),
        GuestReason::Proof(ProofError::BadSignature),
    );
}

#[test]
fn credential_proof_for_different_fingerprint_is_guest() {
    let mut digest = [0x42; 32];
    digest[31] = 0x43;
    let proof = sign_join_proof(
        &device_key(),
        &event_id(),
        &Fingerprint::from_digest(digest),
        NOW,
    );
    assert_guest(
        evaluate(&credential_text(payload()), Some(&proof)),
        GuestReason::Proof(ProofError::BadSignature),
    );
}

#[test]
fn credential_proof_with_altered_timestamp_is_guest() {
    let mut proof = proof_at(NOW);
    proof.timestamp += 1;
    assert_guest(
        evaluate(&credential_text(payload()), Some(&proof)),
        GuestReason::Proof(ProofError::BadSignature),
    );
}

#[test]
fn credential_proof_window_is_five_minutes_inclusive() {
    let text = credential_text(payload());
    assert_eq!(PROOF_WINDOW_MS, 300_000);
    assert_guest(
        evaluate(&text, Some(&proof_at(NOW - PROOF_WINDOW_MS - 1))),
        GuestReason::Proof(ProofError::Stale),
    );
    assert_guest(
        evaluate(&text, Some(&proof_at(NOW + PROOF_WINDOW_MS + 1))),
        GuestReason::Proof(ProofError::Future),
    );
    for ts in [NOW - PROOF_WINDOW_MS, NOW + PROOF_WINDOW_MS] {
        assert!(
            matches!(
                evaluate(&text, Some(&proof_at(ts))),
                IdentityDecision::Verified(_)
            ),
            "{ts}"
        );
    }
}

#[test]
fn credential_proof_extreme_timestamps_do_not_overflow() {
    let p = payload();
    for (ts, err) in [
        (i64::MIN, ProofError::Stale),
        (i64::MAX, ProofError::Future),
    ] {
        let proof = proof_at(ts);
        assert_eq!(
            verify_join_proof(&p, &proof, &event_id(), &fingerprint(), NOW),
            Err(err)
        );
    }
}

#[test]
fn credential_missing_proof_is_guest() {
    assert_guest(
        evaluate(&credential_text(payload()), None),
        GuestReason::MissingProof,
    );
}

#[test]
fn credential_guest_reasons_have_distinct_codes() {
    let reasons = [
        GuestReason::NoCredential,
        cred_guest(CredentialError::Malformed("x")),
        cred_guest(CredentialError::UnknownKid),
        cred_guest(CredentialError::BadSignature),
        cred_guest(CredentialError::Expired),
        cred_guest(CredentialError::NotYetValid),
        cred_guest(CredentialError::WrongRole),
        GuestReason::MissingProof,
        GuestReason::Proof(ProofError::BadSignature),
        GuestReason::Proof(ProofError::Stale),
        GuestReason::Proof(ProofError::Future),
    ];
    let codes: std::collections::BTreeSet<&str> = reasons.iter().map(|r| r.as_str()).collect();
    assert_eq!(codes.len(), reasons.len());
    for c in codes {
        assert!(
            c.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'),
            "{c}"
        );
    }
}

/// Known answer: fixed keys, fixed payload, literal output. Any change to the
/// signed bytes, the JSON form or the encoding fails here.
#[test]
fn credential_known_answer() {
    const CREDENTIAL: &str = "v1.eyJraWQiOiJ0ZXN0LTEiLCJhY2NvdW50X2lkIjoiN2YxYzJlOWEtM2I0ZC00ZTVmLThhNmItMWMyZDNlNGY1YTZiIiwiZGlzcGxheV9uYW1lIjoiQWRhIiwicm9sZSI6InZlcmlmaWVkIiwiZGV2aWNlX3B1YmtleSI6Il9SY2tPRnFneDF0ay0zak5ZQy1oMlpIOTZfZHJFOFdPMXdMcXlEWHA5aGciLCJpc3N1ZWRfYXQiOjE3NjI5OTk5OTkwMDAsImV4cGlyZXNfYXQiOjE3NjM2MDQ4MDAwMDB9.oL2gw_P-nnKBCQXNFkPMe__TuIAgAFuy72a-a1-ZXnxVt6VPNKX-oMkPq7d8DH8UKoN6aoD-KyywB03Da0ZRCg";
    const PROOF_SIGNATURE: &str =
        "QrZTsCK3wVimdpJwCW2yy7agyP9pHDoIcTqQ-WS2P82HtDZDcJkE2RuWziXWwsJqOrrVuGPCBFLnvcOf6q4eCw";
    assert_eq!(credential_text(payload()), CREDENTIAL);
    let proof = proof_at(NOW);
    assert_eq!(proof.timestamp, NOW);
    assert_eq!(proof.signature.to_string(), PROOF_SIGNATURE);
    assert_eq!(
        serde_json::to_string(&proof).unwrap(),
        format!("{{\"timestamp\":{NOW},\"signature\":\"{PROOF_SIGNATURE}\"}}")
    );
    // The literals verify on their own.
    let parsed: JoinProof = serde_json::from_str(&format!(
        "{{\"timestamp\":{NOW},\"signature\":\"{PROOF_SIGNATURE}\"}}"
    ))
    .unwrap();
    assert!(matches!(
        evaluate(CREDENTIAL, Some(&parsed)),
        IdentityDecision::Verified(_)
    ));
}

#[test]
fn credential_key_generation_uses_the_callers_random_source() {
    let mut calls = 0;
    let mut fill = |b: &mut [u8]| {
        calls += 1;
        b.fill(7);
    };
    let service = ServiceSigningKey::generate("svc-9", &mut fill);
    assert_eq!(service.kid(), "svc-9");
    assert_eq!(
        service.public_key().key_bytes(),
        service_key().public_key().key_bytes()
    );
    assert_eq!(service.as_secret_bytes(), &SERVICE_SEED);
    let mut fill9 = |b: &mut [u8]| b.fill(9);
    let device = DeviceSigningKey::generate(&mut fill9);
    assert_eq!(device.public_key(), device_key().public_key());
    assert_eq!(device.as_secret_bytes(), &DEVICE_SEED);
    assert_eq!(calls, 1);
    // Different randomness, different key.
    let mut counter = 0u8;
    let mut fill_seq = |b: &mut [u8]| {
        for x in b.iter_mut() {
            counter = counter.wrapping_add(1);
            *x = counter;
        }
    };
    let a = DeviceSigningKey::generate(&mut fill_seq);
    let b = DeviceSigningKey::generate(&mut fill_seq);
    assert_ne!(a.public_key(), b.public_key());
}

static STATIC_KEYS: [ServicePublicKey; 1] = [ServicePublicKey::from_static("static-1", [0x5a; 32])];
static STATIC_LIST: KeyList = KeyList::from_static(&STATIC_KEYS);

#[test]
fn credential_key_list_from_static_data() {
    assert_eq!(STATIC_LIST.keys().len(), 1);
    assert_eq!(
        STATIC_LIST.get("static-1").unwrap().key_bytes(),
        &[0x5a; 32]
    );
    assert!(STATIC_LIST.get("static-2").is_none());
    assert_eq!(
        STATIC_LIST,
        KeyList::new(vec![ServicePublicKey::new("static-1", [0x5a; 32])])
    );
}

#[test]
fn credential_development_key_list_is_separate() {
    let dev = ServiceSigningKey::development();
    assert_eq!(dev.kid(), DEVELOPMENT_KID);
    assert_eq!(DEVELOPMENT_KEY_LIST.keys(), &[dev.public_key()]);
    assert_eq!(
        DevicePublicKey::from_bytes(*DEVELOPMENT_KEY_LIST.keys()[0].key_bytes()).to_string(),
        "Keoa0vi2hNTSPKaLIsfhpWJNqMM9ANmKBH9qbCYoc-8"
    );
    let mut p = payload();
    p.kid = DEVELOPMENT_KID.into();
    let text = sign_credential(p.clone(), &dev).encode();
    assert_eq!(verify_credential(&text, &DEVELOPMENT_KEY_LIST, NOW), Ok(p));
    // The development key is not accepted by another list, and vice versa.
    assert_eq!(
        verify_credential(&text, &key_list(), NOW),
        Err(CredentialError::UnknownKid)
    );
    assert_eq!(
        verify_credential(&credential_text(payload()), &DEVELOPMENT_KEY_LIST, NOW),
        Err(CredentialError::UnknownKid)
    );
}

#[test]
fn credential_secret_keys_have_no_debug_bytes() {
    let seed: [u8; 32] = std::array::from_fn(|i| (i as u8).wrapping_mul(37).wrapping_add(11));
    let hex: String = seed.iter().map(|b| format!("{b:02x}")).collect();
    let service = ServiceSigningKey::from_secret_bytes("svc-1", &seed);
    let device = DeviceSigningKey::from_secret_bytes(&seed);
    for text in [
        format!("{service:?}"),
        format!("{device:?}"),
        format!("{service:#?}"),
    ] {
        assert!(!text.contains(&format!("{seed:?}")), "{text}");
        assert!(
            !text.contains(&format!("{:?}", &seed[..8])[1..20]),
            "{text}"
        );
        assert!(!text.contains(&hex[..16]), "{text}");
        assert!(!text.contains(&b64(&seed)[..16]), "{text}");
        assert!(text.contains(&device.public_key().to_string()), "{text}");
    }
}

/// Compiles only while `T` implements neither `Serialize` nor `Deserialize`:
/// with an implementation, the `_` below is ambiguous.
macro_rules! assert_not_serde {
    ($t:ty) => {{
        trait NotSerialize<A> {
            fn check() {}
        }
        impl<T: ?Sized> NotSerialize<()> for T {}
        #[allow(dead_code)]
        struct IsSerialize;
        impl<T: ?Sized + serde::Serialize> NotSerialize<IsSerialize> for T {}
        <$t as NotSerialize<_>>::check();

        trait NotDeserialize<A> {
            fn check() {}
        }
        impl<T: ?Sized> NotDeserialize<()> for T {}
        #[allow(dead_code)]
        struct IsDeserialize;
        impl<T: ?Sized + serde::de::DeserializeOwned> NotDeserialize<IsDeserialize> for T {}
        <$t as NotDeserialize<_>>::check();
    }};
}

#[test]
fn credential_secret_keys_are_not_serde() {
    assert_not_serde!(ServiceSigningKey);
    assert_not_serde!(DeviceSigningKey);
}

#[test]
fn credential_secret_keys_zeroize_on_drop() {
    // ed25519-dalek zeroes `SigningKey` on drop only with its `zeroize` feature.
    let manifest =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.toml")).unwrap();
    let line = manifest
        .lines()
        .find(|l| l.starts_with("ed25519-dalek"))
        .expect("workspace declares ed25519-dalek");
    assert!(line.contains("\"zeroize\""), "{line}");
}

#[test]
fn credential_module_reads_no_clock_and_does_no_io() {
    let src = include_str!("../src/credential.rs");
    for word in [
        "SystemTime",
        "Instant",
        "std::fs",
        "std::net",
        "std::io",
        "std::thread",
        "std::env",
        "async ",
        ".await",
        "unsafe",
    ] {
        assert!(!src.contains(word), "credential.rs uses {word}");
    }
}
