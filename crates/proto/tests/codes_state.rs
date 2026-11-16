//! §5.2 codes and §7 state machine.

use proto::codes::*;
use proto::model::ItemState;
use proto::state::{self, Actor, TRANSITIONS};

#[test]
fn item_reasons_and_retryable() {
    let all: Vec<&str> = ItemReason::ALL.iter().map(|r| r.as_str()).collect();
    assert_eq!(
        all,
        [
            "type",
            "size",
            "storage_full",
            "abandoned",
            "stalled",
            "cancelled",
            "unauthorized",
            "invalid_image",
            "integrity",
            "duplicate"
        ]
    );
    let retryable: Vec<&str> = ItemReason::ALL
        .iter()
        .filter(|r| r.retryable())
        .map(|r| r.as_str())
        .collect();
    assert_eq!(retryable, ["abandoned", "stalled", "integrity"]);
    assert!(!ItemReason::Unknown.retryable());
}

#[test]
fn join_failures() {
    let host: Vec<&str> = JoinFailure::ALL
        .iter()
        .filter(|f| !f.is_client_side())
        .map(|f| f.as_str())
        .collect();
    assert_eq!(
        host,
        [
            "wrong_password",
            "token_invalid",
            "removed",
            "event_ended",
            "browser_not_allowed",
            "rate_limited"
        ]
    );
    let client: Vec<&str> = JoinFailure::ALL
        .iter()
        .filter(|f| f.is_client_side())
        .map(|f| f.as_str())
        .collect();
    assert_eq!(
        client,
        [
            "local_network_denied",
            "fingerprint_mismatch",
            "unreachable",
            "token_expired"
        ]
    );
    for f in JoinFailure::ALL {
        assert_eq!(f.http_status().is_none(), f.is_client_side(), "{f}");
    }
    assert_eq!(JoinFailure::RateLimited.http_status(), Some(429));
}

#[test]
fn api_codes_and_statuses() {
    let pairs: Vec<(&str, u16)> = ApiCode::ALL
        .iter()
        .map(|c| (c.as_str(), c.http_status()))
        .collect();
    assert_eq!(
        pairs,
        [
            ("session_invalid", 401),
            ("role_denied", 403),
            ("not_found", 404),
            ("not_available", 409),
            ("offset_mismatch", 409),
            ("rate_limited", 429),
            ("bad_request", 400),
            ("proto_unsupported", 426),
        ]
    );
}

#[test]
fn error_detail_code_helpers() {
    let body = ErrorBody::api(ApiCode::RoleDenied, "no");
    assert_eq!(body.error.api_code(), Some(ApiCode::RoleDenied));
    assert_eq!(body.error.join_failure(), None);
    let body = ErrorBody::join(JoinFailure::Removed, "no");
    assert_eq!(body.error.join_failure(), Some(JoinFailure::Removed));
    // An unknown code still parses as a body.
    let body: ErrorBody = serde_json::from_str(
        r#"{"error": {"code": "brand_new", "message": "?", "retryable": false}}"#,
    )
    .unwrap();
    assert_eq!(body.error.code, "brand_new");
    assert_eq!(body.error.api_code(), None);
}

use ItemState::*;

/// The digest §7 table, written out independently of the crate's table.
#[allow(clippy::type_complexity)]
const DIGEST_ROWS: &[(Option<ItemState>, ItemState, Actor, &[&str], Option<bool>)] = &[
    (None, Pending, Actor::Host, &[], None),
    (Some(Pending), Uploading, Actor::Host, &[], None),
    (
        Some(Pending),
        Failed,
        Actor::Host,
        &["type", "size", "storage_full"],
        Some(false),
    ),
    (
        Some(Pending),
        Failed,
        Actor::Sweeper,
        &["abandoned"],
        Some(true),
    ),
    (Some(Uploading), Validating, Actor::Host, &[], None),
    (
        Some(Uploading),
        Failed,
        Actor::Sweeper,
        &["stalled"],
        Some(true),
    ),
    (
        Some(Uploading),
        Failed,
        Actor::HostOnRequest,
        &["cancelled"],
        Some(false),
    ),
    (
        Some(Uploading),
        Failed,
        Actor::Host,
        &["unauthorized"],
        Some(false),
    ),
    (Some(Failed), Uploading, Actor::Host, &[], None),
    (Some(Validating), Available, Actor::Host, &[], None),
    (
        Some(Validating),
        Failed,
        Actor::Host,
        &["invalid_image"],
        Some(false),
    ),
    (
        Some(Validating),
        Failed,
        Actor::Host,
        &["integrity"],
        Some(true),
    ),
    (Some(Validating), Uploading, Actor::Host, &[], None),
    (Some(Available), Deleted, Actor::Host, &[], None),
];

#[test]
fn state_table_has_exactly_the_digest_rows() {
    assert_eq!(TRANSITIONS.len(), DIGEST_ROWS.len());
    for (t, (from, to, actor, reasons, retryable)) in TRANSITIONS.iter().zip(DIGEST_ROWS) {
        assert_eq!(t.from, *from);
        assert_eq!(t.to, *to);
        assert_eq!(t.actor, *actor, "{from:?} -> {to:?}");
        let got: Vec<&str> = t.reasons.iter().map(|r| r.as_str()).collect();
        assert_eq!(got, *reasons, "{from:?} -> {to:?}");
        assert_eq!(t.retryable, *retryable, "{from:?} -> {to:?}");
        // A row's Retryable column agrees with the reason codes' retryable flag.
        for r in t.reasons {
            assert_eq!(Some(r.retryable()), t.retryable, "{r}");
        }
    }
}

#[test]
fn every_other_pair_is_illegal() {
    let legal: Vec<(Option<ItemState>, ItemState)> =
        DIGEST_ROWS.iter().map(|r| (r.0, r.1)).collect();
    let froms = std::iter::once(None).chain(ItemState::ALL.iter().copied().map(Some));
    let mut checked = 0;
    for from in froms {
        for &to in ItemState::ALL {
            assert_eq!(
                state::is_legal(from, to),
                legal.contains(&(from, to)),
                "{from:?} -> {to:?}"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 7 * 6);
    // Spot checks of illegal moves.
    assert!(!state::is_legal(Some(Available), Failed));
    assert!(!state::is_legal(Some(Deleted), Available));
    assert!(!state::is_legal(Some(Pending), Available));
    assert!(!state::is_legal(None, Uploading));
}

#[test]
fn find_matches_reason_codes() {
    let row = state::find(Some(Pending), Failed, Some(ItemReason::Size)).unwrap();
    assert_eq!(row.retryable, Some(false));
    let row = state::find(Some(Pending), Failed, Some(ItemReason::Abandoned)).unwrap();
    assert_eq!(row.actor, Actor::Sweeper);
    assert!(state::find(Some(Pending), Failed, Some(ItemReason::Stalled)).is_none());
    assert!(state::find(Some(Pending), Failed, None).is_none());
    assert!(state::find(Some(Uploading), Validating, None).is_some());
    assert!(state::find(Some(Uploading), Validating, Some(ItemReason::Type)).is_none());
}
