//! §10/§11 configuration, and tolerance of unknown values in open enums.

use proto::codes::{ApiCode, ItemReason, JoinFailure};
use proto::config::*;
use proto::model::{AccessType, ItemState, Role};
use proto::ws::{
    ByeReason, Change, ChangeBody, ChangeType, ClientMessage, NoticeKind, ServerMessage,
};
use serde_json::json;

#[test]
fn compiled_defaults_and_bounds() {
    let c = Config::default();
    assert_eq!(c.transfer_config.chunk_bytes, 1_048_576);
    assert_eq!(c.transfer_config.stall_timeout_ms, 120_000);
    assert_eq!(c.transfer_config.transfer_retries, 3);
    assert_eq!(c.transfer_config.backoff_base_ms, 500);
    assert_eq!(c.outbox_config.max_attempts, 10);
    assert_eq!(c.outbox_config.max_age_ms, 24 * 3_600_000);
    assert_eq!(c.rate_limits.uploads_per_window, 30);
    assert_eq!(c.rate_limits.upload_window_ms, 300_000);
    assert!(c.browser_guests_available);
    assert!(c.quick_session_enabled);
    assert!(c.desktop_download_all);
    assert_eq!(c.reachability_test_mode, ReachabilityTestMode::Full);
    assert_eq!(c.compatible_copy_strategy, CompatibleCopyStrategy::OnDemand);
    assert_eq!((CHUNK_BYTES.min, CHUNK_BYTES.max), (262_144, 4_194_304));
    for b in TUNABLES {
        assert!(b.min <= b.default && b.default <= b.max, "{}", b.key);
    }
}

#[test]
fn fixed_constants() {
    assert_eq!(MAX_PHOTO_BYTES, 52_428_800);
    assert_eq!(PENDING_TIMEOUT_MS, 600_000);
    assert_eq!(FAILED_CLEANUP_MS, 3_600_000);
    assert_eq!(SWEEPER_INTERVAL_MS, 60_000);
    assert_eq!(SESSION_TTL_MS, 12 * 3_600_000);
    assert_eq!(BROWSER_SESSION_TTL_MS, 2 * 3_600_000);
    assert_eq!(CREDENTIAL_LIFETIME_MS, 7 * 86_400_000);
    assert_eq!(CREDENTIAL_REFRESH_REMAINING_MS, 2 * 86_400_000);
    assert_eq!(COUNT_BATCH_INTERVAL_MS, 30_000);
    assert_eq!(HOT_SCORE_HALF_LIFE_MS, 900_000);
    assert_eq!(PREVIEW_MAX_EDGE_PX, 1024);
    assert_eq!(COMPATIBLE_JPEG_QUALITY, 92);
    assert_eq!(PREVIEW_JPEG_QUALITY, 80);
    assert_eq!(MAX_INTEGRITY_ATTEMPTS, 3);
    assert_eq!(DEFAULT_STORAGE_LIMIT_BYTES, 20 * 1024 * 1024 * 1024);
    assert_eq!(QUICK_SESSION_STORAGE_LIMIT_BYTES, 5 * 1024 * 1024 * 1024);
    assert_eq!(MAX_EVENT_LIFETIME_MS, 7 * 86_400_000);
    assert_eq!(LOGIN_CODE_DIGITS, 6);
    assert_eq!(LOGIN_CODE_TTL_MS, 600_000);
    assert_eq!(proto::ws::HELLO_TIMEOUT_MS, 5_000);
    assert!(post_text_valid("a"));
    assert!(post_text_valid(&"é".repeat(500)));
    assert!(!post_text_valid(""));
    assert!(!post_text_valid(&"a".repeat(501)));
}

#[test]
fn sanitize_replaces_out_of_range_values() {
    let mut c = Config::default();
    c.transfer_config.chunk_bytes = 8 * 1024 * 1024; // above 4 MiB
    c.transfer_config.stall_timeout_ms = 1; // below min
    c.transfer_config.transfer_retries = 0;
    c.transfer_config.backoff_base_ms = 300; // in range: kept
    c.outbox_config.max_attempts = 1000;
    c.outbox_config.max_age_ms = 2 * 3_600_000; // in range: kept
    c.rate_limits.uploads_per_window = 0;
    c.rate_limits.upload_window_ms = 10 * 3_600_000;
    c.reachability_test_mode = ReachabilityTestMode::Unknown;
    c.compatible_copy_strategy = CompatibleCopyStrategy::Unknown;
    c.quick_session_enabled = false; // flags are kept
    let s = c.sanitize();
    let d = Config::default();
    assert_eq!(s.transfer_config.chunk_bytes, d.transfer_config.chunk_bytes);
    assert_eq!(
        s.transfer_config.stall_timeout_ms,
        d.transfer_config.stall_timeout_ms
    );
    assert_eq!(
        s.transfer_config.transfer_retries,
        d.transfer_config.transfer_retries
    );
    assert_eq!(s.transfer_config.backoff_base_ms, 300);
    assert_eq!(s.outbox_config.max_attempts, d.outbox_config.max_attempts);
    assert_eq!(s.outbox_config.max_age_ms, 2 * 3_600_000);
    assert_eq!(
        s.rate_limits.uploads_per_window,
        d.rate_limits.uploads_per_window
    );
    assert_eq!(
        s.rate_limits.upload_window_ms,
        d.rate_limits.upload_window_ms
    );
    assert_eq!(s.reachability_test_mode, ReachabilityTestMode::Full);
    assert_eq!(s.compatible_copy_strategy, CompatibleCopyStrategy::OnDemand);
    assert!(!s.quick_session_enabled);
    // Bounds are inclusive.
    let mut edge = Config::default();
    edge.transfer_config.chunk_bytes = CHUNK_BYTES.min;
    assert_eq!(edge.sanitize().transfer_config.chunk_bytes, 262_144);
    edge.transfer_config.chunk_bytes = CHUNK_BYTES.max;
    assert_eq!(edge.sanitize().transfer_config.chunk_bytes, 4_194_304);
}

#[test]
fn apply_remote_ignores_bad_values() {
    let remote = json!({
        "browser_guests_available": false,
        "quick_session_enabled": "no",          // wrong type: ignored
        "reachability_test_mode": "lite",       // unknown: ignored
        "compatible_copy_strategy": "on_demand",
        "transfer_config": {
            "chunk_bytes": 2097152,             // in range
            "stall_timeout_ms": 5,              // below min: ignored
            "transfer_retries": 4,
            "backoff_base_ms": -1               // not u64: ignored
        },
        "outbox_config": {"max_attempts": 51},  // above max: ignored
        "rate_limits": {"upload_window_ms": 120000},
        "something_new": 1
    });
    let mut c = Config::default();
    c.apply_remote(&remote);
    let d = Config::default();
    assert!(!c.browser_guests_available);
    assert!(c.quick_session_enabled);
    assert_eq!(c.reachability_test_mode, ReachabilityTestMode::Full);
    assert_eq!(c.transfer_config.chunk_bytes, 2_097_152);
    assert_eq!(
        c.transfer_config.stall_timeout_ms,
        d.transfer_config.stall_timeout_ms
    );
    assert_eq!(c.transfer_config.transfer_retries, 4);
    assert_eq!(
        c.transfer_config.backoff_base_ms,
        d.transfer_config.backoff_base_ms
    );
    assert_eq!(c.outbox_config, d.outbox_config);
    assert_eq!(c.rate_limits.upload_window_ms, 120_000);
    assert_eq!(c, Config::from_remote(&remote));

    // Layering: cached then fresh; a bad fresh value keeps the cached one.
    let mut layered = Config::from_remote(&json!({"transfer_config": {"chunk_bytes": 524288}}));
    layered.apply_remote(&json!({"transfer_config": {"chunk_bytes": 1}}));
    assert_eq!(layered.transfer_config.chunk_bytes, 524_288);
    // Not an object: nothing changes.
    let mut same = Config::default();
    same.apply_remote(&json!([1, 2]));
    assert_eq!(same, Config::default());
}

#[test]
fn config_missing_keys_take_defaults() {
    let c: Config =
        serde_json::from_str(r#"{"transfer_config": {"chunk_bytes": 524288}}"#).unwrap();
    assert_eq!(c.transfer_config.chunk_bytes, 524_288);
    assert_eq!(c.transfer_config.transfer_retries, 3);
    assert_eq!(c.outbox_config, OutboxConfig::default());
}

// ---------------------------------------------------------------- open / closed

#[test]
fn unknown_ws_message_types_parse_to_unknown() {
    let m: ServerMessage = serde_json::from_str(r#"{"t": "typing", "who": "x"}"#).unwrap();
    assert_eq!(m, ServerMessage::Unknown);
    let m: ClientMessage = serde_json::from_str(r#"{"t": "subscribe", "topic": 1}"#).unwrap();
    assert_eq!(m, ClientMessage::Unknown);
}

#[test]
fn unknown_change_types_keep_their_payload() {
    let raw =
        r#"{"version": 7, "type": "reaction_added", "payload": {"id": "x", "emoji": 1}, "ts": 5}"#;
    let c: Change = serde_json::from_str(raw).unwrap();
    assert_eq!(c.version, 7);
    assert_eq!(c.body.change_type(), ChangeType::Unknown);
    assert_eq!(
        c.body,
        ChangeBody::Unknown {
            change_type: "reaction_added".into(),
            payload: json!({"id": "x", "emoji": 1}),
        }
    );
    // Re-serialises unchanged.
    assert_eq!(
        serde_json::to_value(&c).unwrap(),
        serde_json::from_str::<serde_json::Value>(raw).unwrap()
    );
    // Inside a catch-up batch, known and unknown changes both parse.
    let batch = r#"{"t": "changes", "changes": [
        {"version": 7, "type": "reaction_added", "payload": {}, "ts": 5},
        {"version": 8, "type": "item_deleted", "payload": {"id": "a"}, "ts": 6}]}"#;
    let ServerMessage::Changes { changes } = serde_json::from_str(batch).unwrap() else {
        panic!("not changes");
    };
    assert_eq!(changes.len(), 2);
    assert_eq!(changes[1].body.change_type(), ChangeType::ItemDeleted);
}

#[test]
fn known_change_type_with_bad_payload_is_an_error() {
    let raw = r#"{"version": 1, "type": "item_deleted", "payload": {"nope": 1}, "ts": 5}"#;
    assert!(serde_json::from_str::<Change>(raw).is_err());
}

#[test]
fn open_enums_tolerate_unknown_values() {
    assert_eq!(
        serde_json::from_str::<ItemReason>("\"virus\"").unwrap(),
        ItemReason::Unknown
    );
    assert_eq!(
        serde_json::from_str::<ChangeType>("\"x\"").unwrap(),
        ChangeType::Unknown
    );
    assert_eq!(
        serde_json::from_str::<ByeReason>("\"kicked\"").unwrap(),
        ByeReason::Unknown
    );
    assert_eq!(
        serde_json::from_str::<NoticeKind>("\"x\"").unwrap(),
        NoticeKind::Unknown
    );
    let m: ServerMessage = serde_json::from_str(r#"{"t": "bye", "reason": "kicked"}"#).unwrap();
    assert_eq!(
        m,
        ServerMessage::Bye {
            reason: ByeReason::Unknown
        }
    );
    let m: ServerMessage =
        serde_json::from_str(r#"{"t": "notice", "kind": "storage_low"}"#).unwrap();
    assert_eq!(
        m,
        ServerMessage::Notice {
            kind: NoticeKind::Unknown,
            expires_at: None
        }
    );
    const { assert!(ItemReason::OPEN && ChangeType::OPEN && ByeReason::OPEN && NoticeKind::OPEN) };
}

#[test]
fn closed_enums_reject_unknown_values() {
    assert!(serde_json::from_str::<ItemState>("\"QUEUED\"").is_err());
    assert!(serde_json::from_str::<ItemState>("\"pending\"").is_err());
    assert!(serde_json::from_str::<AccessType>("\"public\"").is_err());
    assert!(serde_json::from_str::<Role>("\"admin\"").is_err());
    assert!("nope".parse::<ApiCode>().is_err());
    assert!("nope".parse::<JoinFailure>().is_err());
    const { assert!(!ItemState::OPEN && !AccessType::OPEN && !Role::OPEN) };
}
