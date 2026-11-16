//! §5.1 and DU-11: event ID, join token, fingerprint, host code, join URL, deep link.

use proto::transport::*;
use proto::ParseError;

fn fp_4f2a() -> Fingerprint {
    let mut d = [0u8; 32];
    d[..6].copy_from_slice(&[0x4f, 0x2a, 0x9c, 0x1e, 0x77, 0xb0]);
    d[6] = 0xff;
    Fingerprint::from_digest(d)
}

#[test]
fn constants() {
    assert_eq!(DEFAULT_PORT, 7420);
    assert_eq!(PROTO_VERSION, 1);
    assert_eq!(proto::api::headers::X_LOCALE_PROTO, "X-Locale-Proto");
    assert_eq!(TLS_FIRST_BYTE, 0x16);
}

#[test]
fn event_id_validation() {
    assert!(EventId::parse("evt_0123456789ab").is_ok());
    assert!(EventId::parse("evt_abcdefabcdef").is_ok());
    for bad in [
        "",
        "evt_",
        "evt_0123456789a",   // 11 hex
        "evt_0123456789abc", // 13 hex
        "evt_0123456789AB",  // uppercase
        "evt_0123456789ag",  // not hex
        "EVT_0123456789ab",
        "ev_0123456789abc",
        "0123456789abcdef",
    ] {
        assert_eq!(
            EventId::parse(bad),
            Err(ParseError::EventId(bad.to_owned())),
            "{bad}"
        );
    }
    assert_eq!(
        EventId::from_random_bytes([0x3f, 0x9a, 0x0c, 0x12, 0xb7, 0xde]).as_str(),
        "evt_3f9a0c12b7de"
    );
    assert!(serde_json::from_str::<EventId>("\"evt_XYZ\"").is_err());
}

#[test]
fn join_token_is_128_bits_base64url() {
    let t = JoinToken::from_bytes([0xfb; 16]);
    let s = t.to_string();
    assert_eq!(s.len(), 22);
    assert!(!s.contains('='));
    assert!(
        !s.contains('+') && !s.contains('/'),
        "url-safe alphabet: {s}"
    );
    assert_eq!(s.parse::<JoinToken>().unwrap(), t);
    // Wrong length, padding and standard alphabet are rejected.
    assert!("q6urq6urq6urq6urq6urqw==".parse::<JoinToken>().is_err());
    assert!("q6urq6urq6urq6urq6ur".parse::<JoinToken>().is_err());
    assert!("-_urq6urq6urq6urq6urq6ur".parse::<JoinToken>().is_err());
    assert!("+/urq6urq6urq6urq6urqw".parse::<JoinToken>().is_err());
}

#[test]
fn fingerprint_text_and_host_code() {
    let fp = fp_4f2a();
    let s = fp.to_string();
    assert_eq!(s.len(), 43);
    assert!(!s.contains('='));
    assert_eq!(s.parse::<Fingerprint>().unwrap(), fp);
    assert_eq!(fp.host_code(), "4F2A · 9C1E · 77B0");
    assert_eq!(fp.host_code(), "4F2A \u{b7} 9C1E \u{b7} 77B0");
    assert_eq!(fp.host_code_hex(), "4F2A9C1E77B0");
    assert_eq!(fp.as_bytes()[6], 0xff);
    assert!("AAAA".parse::<Fingerprint>().is_err());
}

const URL: &str = "http://192.168.1.20:7420/e/evt_3f9a0c12b7de#t=q6urq6urq6urq6urq6urqw&fp=AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8";

#[test]
fn join_url_parses_and_round_trips() {
    let u = JoinUrl::parse(URL).unwrap();
    assert_eq!(u.host.to_string(), "192.168.1.20");
    assert_eq!(u.port, 7420);
    assert_eq!(u.event_id.as_str(), "evt_3f9a0c12b7de");
    assert_eq!(u.join_token, JoinToken::from_bytes([0xab; 16]));
    let mut d = [0u8; 32];
    for (i, b) in d.iter_mut().enumerate() {
        *b = i as u8;
    }
    assert_eq!(u.fingerprint, Fingerprint::from_digest(d));
    assert_eq!(u.to_string(), URL);
    assert_eq!(u.page_url(), "http://192.168.1.20:7420/e/evt_3f9a0c12b7de");
    // Fragment keys in either order, unknown keys ignored.
    let swapped = "http://192.168.1.20:7420/e/evt_3f9a0c12b7de#fp=AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8&x=1&t=q6urq6urq6urq6urq6urqw";
    assert_eq!(JoinUrl::parse(swapped).unwrap(), u);
}

#[test]
fn join_url_ipv6_round_trips() {
    let s = "http://[fe80::1]:7421/e/evt_3f9a0c12b7de#t=q6urq6urq6urq6urq6urqw&fp=AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8";
    let u = JoinUrl::parse(s).unwrap();
    assert_eq!(u.port, 7421);
    assert_eq!(u.to_string(), s);
}

#[test]
fn join_url_rejects_malformed() {
    let fp = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8";
    let t = "q6urq6urq6urq6urq6urqw";
    for bad in [
        format!("https://192.168.1.20:7420/e/evt_3f9a0c12b7de#t={t}&fp={fp}"),
        format!("http://192.168.1.20/e/evt_3f9a0c12b7de#t={t}&fp={fp}"),
        format!("http://host.local:7420/e/evt_3f9a0c12b7de#t={t}&fp={fp}"),
        format!("http://192.168.1.20:0/e/evt_3f9a0c12b7de#t={t}&fp={fp}"),
        format!("http://192.168.1.20:7420/x/evt_3f9a0c12b7de#t={t}&fp={fp}"),
        format!("http://192.168.1.20:7420/e/evt_3f9a0c12b7de/#t={t}&fp={fp}"),
        format!("http://192.168.1.20:7420/e/evt_bad#t={t}&fp={fp}"),
        "http://192.168.1.20:7420/e/evt_3f9a0c12b7de".to_owned(),
        format!("http://192.168.1.20:7420/e/evt_3f9a0c12b7de#fp={fp}"),
        format!("http://192.168.1.20:7420/e/evt_3f9a0c12b7de#t={t}"),
        format!("http://192.168.1.20:7420/e/evt_3f9a0c12b7de#t={t}&t={t}&fp={fp}"),
        format!("http://192.168.1.20:7420/e/evt_3f9a0c12b7de#t=abc&fp={fp}"),
        format!("http://192.168.1.20:7420/e/evt_3f9a0c12b7de#t={t}&fp=abc"),
    ] {
        assert!(
            matches!(JoinUrl::parse(&bad), Err(ParseError::JoinUrl(_))),
            "{bad}"
        );
    }
}

#[test]
fn deep_link_format_and_parse() {
    let u = JoinUrl::parse(URL).unwrap();
    let link = JoinDeepLink {
        join_url: u.clone(),
    };
    let expected = "locale://join?u=http%3A%2F%2F192.168.1.20%3A7420%2Fe%2Fevt_3f9a0c12b7de%23t%3Dq6urq6urq6urq6urq6urqw%26fp%3DAAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8";
    assert_eq!(link.to_string(), expected);
    assert_eq!(JoinDeepLink::parse(expected).unwrap().join_url, u);
    assert_eq!(DEEP_LINK_PREFIX, "locale://join?u=");
    // Other query keys are ignored.
    let with_extra = expected.replace("join?u=", "join?src=web&u=");
    assert_eq!(JoinDeepLink::parse(&with_extra).unwrap().join_url, u);
}

#[test]
fn deep_link_rejects_malformed() {
    for bad in [
        "locale://open?u=x",
        "locale://join?",
        "locale://join?u=not-a-url",
        "https://join?u=x",
        "locale://join?u=%FF",
    ] {
        assert!(
            matches!(JoinDeepLink::parse(bad), Err(ParseError::DeepLink(_))),
            "{bad}"
        );
    }
}
