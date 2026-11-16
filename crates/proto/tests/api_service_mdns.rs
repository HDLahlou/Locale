//! §5.3 routes, headers and rate limits; §5.5 credential and proof; §5.6 mDNS.

use proto::api::{self, headers, routes, ENDPOINTS};
use proto::mdns::{self, TxtRecord};
use proto::model::AccessType;
use proto::service::*;
use proto::transport::{EventId, Fingerprint};
use proto::ParseError;

fn evt() -> EventId {
    EventId::parse("evt_3f9a0c12b7de").unwrap()
}

#[test]
fn header_names() {
    assert_eq!(headers::UPLOAD_TOKEN, "Upload-Token");
    assert_eq!(headers::UPLOAD_OFFSET, "Upload-Offset");
    assert_eq!(headers::X_LOCALE_SHA256, "X-Locale-Sha256");
    assert_eq!(headers::X_JOIN_TOKEN, "X-Join-Token");
    assert_eq!(headers::X_LOCALE_PROTO, "X-Locale-Proto");
}

#[test]
fn rate_limits() {
    assert_eq!(api::UPLOADS_PER_WINDOW, 30);
    assert_eq!(api::UPLOAD_WINDOW_MS, 5 * 60 * 1000);
    assert_eq!(api::JOINS_PER_MINUTE_PER_IP, 10);
    assert_eq!(api::PASSWORD_ATTEMPTS_PER_MINUTE_PER_IP, 5);
}

#[test]
fn route_builders() {
    let e = evt();
    let id = "7d1e5f0a-2b3c-4d5e-8f90-a1b2c3d4e5f6".to_owned();
    let sub = "c0ffee00-1234-4abc-8def-0123456789ab".to_owned();
    let p = "/e/evt_3f9a0c12b7de";
    assert_eq!(routes::event_base(&e), p);
    assert_eq!(routes::meta(&e), format!("{p}/meta"));
    assert_eq!(routes::join(&e), format!("{p}/join"));
    assert_eq!(routes::session_refresh(&e), format!("{p}/session/refresh"));
    assert_eq!(routes::ws(&e), format!("{p}/ws"));
    assert_eq!(routes::items(&e), format!("{p}/items"));
    assert_eq!(routes::upload(&e, &sub), format!("{p}/uploads/{sub}"));
    assert_eq!(
        routes::submission(&e, &sub),
        format!("{p}/submissions/{sub}")
    );
    assert_eq!(
        routes::item_preview(&e, &id),
        format!("{p}/items/{id}/preview")
    );
    assert_eq!(
        routes::item_original(&e, &id, false),
        format!("{p}/items/{id}/original")
    );
    assert_eq!(
        routes::item_original(&e, &id, true),
        format!("{p}/items/{id}/original?save=1")
    );
    assert_eq!(
        routes::item_compatible(&e, &id, true),
        format!("{p}/items/{id}/compatible?save=1")
    );
    assert_eq!(routes::item_saved(&e, &id), format!("{p}/items/{id}/saved"));
    assert_eq!(routes::item(&e, &id), format!("{p}/items/{id}"));
    assert_eq!(routes::posts(&e), format!("{p}/posts"));
    assert_eq!(routes::post(&e, &id), format!("{p}/posts/{id}"));
}

#[test]
fn endpoint_table_matches_digest_rows() {
    // §5.3 table, with `/items/{id}/original · /compatible` and
    // `POST /posts · DELETE /posts/{id}` split into one row each.
    let expected = [
        ("GET", "/"),
        ("GET", "/meta"),
        ("POST", "/join"),
        ("POST", "/session/refresh"),
        ("GET", "/ws"),
        ("POST", "/items"),
        ("HEAD", "/uploads/{submission_id}"),
        ("PUT", "/uploads/{submission_id}"),
        ("DELETE", "/uploads/{submission_id}"),
        ("GET", "/submissions/{submission_id}"),
        ("GET", "/items/{id}/preview"),
        ("GET", "/items/{id}/original"),
        ("GET", "/items/{id}/compatible"),
        ("POST", "/items/{id}/saved"),
        ("DELETE", "/items/{id}"),
        ("POST", "/posts"),
        ("DELETE", "/posts/{id}"),
    ];
    let got: Vec<(&str, String)> = ENDPOINTS
        .iter()
        .map(|e| {
            let rest = e.path.strip_prefix("/e/{event_id}").unwrap();
            (
                e.method,
                if rest.is_empty() {
                    "/".into()
                } else {
                    rest.to_owned()
                },
            )
        })
        .collect();
    let want: Vec<(&str, String)> = expected.iter().map(|(m, p)| (*m, p.to_string())).collect();
    assert_eq!(got, want);
}

#[test]
fn service_routes() {
    let got: Vec<(&str, &str)> = SERVICE_ENDPOINTS
        .iter()
        .map(|e| (e.method, e.path))
        .collect();
    assert_eq!(
        got,
        [
            ("POST", "/v1/auth/start"),
            ("POST", "/v1/auth/verify"),
            ("POST", "/v1/credential/refresh"),
            ("PATCH", "/v1/account"),
            ("POST", "/v1/host-link"),
        ]
    );
}

fn payload() -> CredentialPayload {
    CredentialPayload {
        kid: "k1".into(),
        account_id: "2e3f4a5b-6c7d-4e8f-9a0b-1c2d3e4f5a6b".into(),
        display_name: "Leo".into(),
        role: CredentialRole::Verified,
        device_pubkey: DevicePublicKey::from_bytes([9; 32]),
        issued_at: 1_763_200_000_000,
        expires_at: 1_763_804_800_000,
    }
}

const PAYLOAD_JSON: &str = r#"{"kid":"k1","account_id":"2e3f4a5b-6c7d-4e8f-9a0b-1c2d3e4f5a6b","display_name":"Leo","role":"verified","device_pubkey":"CQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQk","issued_at":1763200000000,"expires_at":1763804800000}"#;
const PAYLOAD_SEG: &str = "eyJraWQiOiJrMSIsImFjY291bnRfaWQiOiIyZTNmNGE1Yi02YzdkLTRlOGYtOWEwYi0xYzJkM2U0ZjVhNmIiLCJkaXNwbGF5X25hbWUiOiJMZW8iLCJyb2xlIjoidmVyaWZpZWQiLCJkZXZpY2VfcHVia2V5IjoiQ1FrSkNRa0pDUWtKQ1FrSkNRa0pDUWtKQ1FrSkNRa0pDUWtKQ1FrSkNRayIsImlzc3VlZF9hdCI6MTc2MzIwMDAwMDAwMCwiZXhwaXJlc19hdCI6MTc2MzgwNDgwMDAwMH0";
const SIG_1: &str =
    "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQ";

#[test]
fn credential_encode_decode() {
    let c = Credential::from_signed(payload(), Signature::from_bytes([1; 64]));
    assert_eq!(c.payload_segment(), PAYLOAD_SEG);
    assert_eq!(serde_json::to_string(c.payload()).unwrap(), PAYLOAD_JSON);
    let text = format!("v1.{PAYLOAD_SEG}.{SIG_1}");
    assert_eq!(c.encode(), text);
    let d = Credential::decode(&text).unwrap();
    assert_eq!(d, c);
    assert_eq!(d.payload(), &payload());
    assert_eq!(d.signature().as_bytes(), &[1; 64]);
    assert_eq!(d.signing_input(), format!("v1.{PAYLOAD_SEG}").into_bytes());
    assert_eq!(serde_json::to_string(&d).unwrap(), format!("\"{text}\""));
}

#[test]
fn credential_keeps_received_payload_bytes() {
    // A payload with different whitespace still decodes and re-encodes unchanged,
    // so the signature input is the bytes that were signed.
    let spaced = PAYLOAD_JSON.replace(',', ", ");
    let seg = base64_url(spaced.as_bytes());
    let text = format!("v1.{seg}.{SIG_1}");
    let d = Credential::decode(&text).unwrap();
    assert_eq!(d.encode(), text);
    assert_eq!(d.payload(), &payload());
    assert_eq!(d.signing_input(), format!("v1.{seg}").into_bytes());
}

fn base64_url(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        let chars = chunk.len() + 1;
        for i in 0..chars {
            out.push(A[((n >> (18 - 6 * i)) & 63) as usize] as char);
        }
    }
    out
}

#[test]
fn credential_decode_rejects_malformed() {
    let cases = [
        format!("{PAYLOAD_SEG}.{SIG_1}"),
        format!("v1.{PAYLOAD_SEG}.{SIG_1}.x"),
        format!("v2.{PAYLOAD_SEG}.{SIG_1}"),
        format!("v1.{PAYLOAD_SEG}=.{SIG_1}"),
        format!("v1.e30.{SIG_1}"), // "{}": not a payload
        format!("v1.{PAYLOAD_SEG}.AQEB"),
        format!("v1.{PAYLOAD_SEG}.{SIG_1}AA"),
        format!("v1.{PAYLOAD_SEG}.{}", SIG_1.replace('A', "+")),
        String::new(),
    ];
    for bad in cases {
        assert!(
            matches!(Credential::decode(&bad), Err(ParseError::Credential(_))),
            "{bad}"
        );
    }
}

#[test]
fn proof_message_bytes() {
    let e = evt();
    let mut d = [0u8; 32];
    for (i, b) in d.iter_mut().enumerate() {
        *b = i as u8;
    }
    let fp = Fingerprint::from_digest(d);
    let ts: i64 = 1_763_200_000_000; // 0x0000019A_86E8_E000
    let msg = proof_message(&e, &fp, ts);
    let mut expected = b"locale-join-v1".to_vec();
    expected.extend_from_slice(b"evt_3f9a0c12b7de");
    expected.extend_from_slice(&d);
    expected.extend_from_slice(&[0x00, 0x00, 0x01, 0x9a, 0x86, 0xe8, 0xe0, 0x00]);
    assert_eq!(msg, expected);
    assert_eq!(msg.len(), 70);
    assert_eq!(PROOF_MESSAGE_LEN, 70);
}

#[test]
fn proof_window_is_five_minutes() {
    assert_eq!(PROOF_WINDOW_MS, 300_000);
    let now = 1_763_200_000_000;
    assert!(proof_timestamp_in_window(now, now));
    assert!(proof_timestamp_in_window(now, now - 300_000));
    assert!(proof_timestamp_in_window(now, now + 300_000));
    assert!(!proof_timestamp_in_window(now, now - 300_001));
    assert!(!proof_timestamp_in_window(now, now + 300_001));
}

#[test]
fn device_key_and_signature_lengths() {
    assert!("CQkJ".parse::<DevicePublicKey>().is_err());
    assert!(SIG_1[..85].parse::<Signature>().is_err());
    assert!(SIG_1.parse::<Signature>().is_ok());
}

fn txt() -> TxtRecord {
    TxtRecord {
        v: 1,
        id: evt(),
        name: "Rooftop".into(),
        host: "Maya".into(),
        access: AccessType::Password,
        st: 1_763_190_000_000,
        en: 1_763_204_400_000,
        fp: "4F2A9C1E77B0".into(),
    }
}

#[test]
fn mdns_txt_pairs() {
    assert_eq!(mdns::SERVICE_TYPE, "_locale._tcp.local.");
    let pairs = txt().to_pairs();
    let expected = [
        ("v", "1"),
        ("id", "evt_3f9a0c12b7de"),
        ("name", "Rooftop"),
        ("host", "Maya"),
        ("access", "password"),
        ("st", "1763190000000"),
        ("en", "1763204400000"),
        ("fp", "4F2A9C1E77B0"),
    ];
    let got: Vec<(&str, &str)> = pairs.iter().map(|(k, v)| (*k, v.as_str())).collect();
    assert_eq!(got, expected);
    assert_eq!(TxtRecord::from_pairs(expected).unwrap(), txt());
    assert_eq!(txt().instance_name(), "evt_3f9a0c12b7de");
    // Unknown keys are ignored; order does not matter.
    let mut shuffled: Vec<(&str, &str)> = expected.iter().rev().copied().collect();
    shuffled.push(("zz", "1"));
    assert_eq!(TxtRecord::from_pairs(shuffled).unwrap(), txt());
}

#[test]
fn mdns_txt_rejects_bad_values() {
    let base = txt().to_pairs();
    let with = |key: &str, value: &str| -> Vec<(String, String)> {
        base.iter()
            .map(|(k, v)| {
                let v = if *k == key {
                    value.to_owned()
                } else {
                    v.clone()
                };
                (k.to_string(), v)
            })
            .collect()
    };
    for (k, v) in [
        ("access", "invite"),
        ("access", "secret"),
        ("id", "evt_x"),
        ("st", "soon"),
        ("en", ""),
        ("fp", "4f2a9c1e77b0"),
        ("fp", "4F2A9C1E77"),
        ("v", "one"),
    ] {
        assert!(
            matches!(
                TxtRecord::from_pairs(with(k, v)),
                Err(ParseError::TxtRecord(_))
            ),
            "{k}={v}"
        );
    }
    let missing: Vec<_> = base.iter().filter(|(k, _)| *k != "fp").cloned().collect();
    assert!(TxtRecord::from_pairs(missing).is_err());
    assert!(mdns::is_advertised(AccessType::Open));
    assert!(mdns::is_advertised(AccessType::Password));
    assert!(!mdns::is_advertised(AccessType::Invite));
}

#[test]
fn txt_fp_matches_fingerprint_host_code() {
    let mut d = [0u8; 32];
    d[..6].copy_from_slice(&[0x4f, 0x2a, 0x9c, 0x1e, 0x77, 0xb0]);
    assert_eq!(Fingerprint::from_digest(d).host_code_hex(), txt().fp);
}
