//! Round-trip serialisation of every wire type against literal JSON written from
//! the digest (§5, §6, §10, §11). The literals are not produced by the code under test.

mod common;

use common::roundtrip;
use proto::api::*;
use proto::codes::*;
use proto::config::*;
use proto::model::*;
use proto::service::*;
use proto::transport::*;
use proto::ws::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;

const EVT: &str = "evt_3f9a0c12b7de";
const ITEM: &str = "7d1e5f0a-2b3c-4d5e-8f90-a1b2c3d4e5f6";
const ATT: &str = "0b6f2c1d-3e4a-4b5c-9d6e-7f8091a2b3c4";
const SUB: &str = "c0ffee00-1234-4abc-8def-0123456789ab";
const POST_ID: &str = "5a4b3c2d-1e0f-4a9b-8c7d-6e5f4a3b2c1d";
const SHA: &str = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";
const PUBKEY_9: &str = "CQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQk";
const SIG_1: &str =
    "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQ";
const PAYLOAD_SEG: &str = "eyJraWQiOiJrMSIsImFjY291bnRfaWQiOiIyZTNmNGE1Yi02YzdkLTRlOGYtOWEwYi0xYzJkM2U0ZjVhNmIiLCJkaXNwbGF5X25hbWUiOiJMZW8iLCJyb2xlIjoidmVyaWZpZWQiLCJkZXZpY2VfcHVia2V5IjoiQ1FrSkNRa0pDUWtKQ1FrSkNRa0pDUWtKQ1FrSkNRa0pDUWtKQ1FrSkNRayIsImlzc3VlZF9hdCI6MTc2MzIwMDAwMDAwMCwiZXhwaXJlc19hdCI6MTc2MzgwNDgwMDAwMH0";

fn evt() -> EventId {
    EventId::parse(EVT).unwrap()
}

/// Checks each enum value against its literal wire string, both ways.
fn enum_strings<T>(pairs: &[(T, &str)])
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    for (value, s) in pairs {
        roundtrip(&format!("\"{s}\""), value);
    }
}

// ---------------------------------------------------------------- enums

#[test]
fn model_enums() {
    enum_strings(&[
        (AccessType::Open, "open"),
        (AccessType::Invite, "invite"),
        (AccessType::Password, "password"),
    ]);
    enum_strings(&[
        (EventStatus::Active, "active"),
        (EventStatus::Paused, "paused"),
        (EventStatus::Ended, "ended"),
    ]);
    enum_strings(&[(EventKind::Event, "event"), (EventKind::Quick, "quick")]);
    enum_strings(&[
        (Role::Host, "host"),
        (Role::Uploader, "uploader"),
        (Role::Viewer, "viewer"),
    ]);
    enum_strings(&[
        (ClientKind::Ios, "ios"),
        (ClientKind::Android, "android"),
        (ClientKind::Desktop, "desktop"),
        (ClientKind::Web, "web"),
    ]);
    enum_strings(&[(SessionScope::Full, "full"), (SessionScope::Read, "read")]);
    enum_strings(&[
        (ItemState::Pending, "PENDING"),
        (ItemState::Uploading, "UPLOADING"),
        (ItemState::Validating, "VALIDATING"),
        (ItemState::Available, "AVAILABLE"),
        (ItemState::Failed, "FAILED"),
        (ItemState::Deleted, "DELETED"),
    ]);
    enum_strings(&[(PostKind::Host, "host"), (PostKind::System, "system")]);
    enum_strings(&[
        (DownloadVersion::Original, "original"),
        (DownloadVersion::Compatible, "compatible"),
    ]);
    enum_strings(&[(CredentialRole::Verified, "verified")]);
}

#[test]
fn code_enums() {
    enum_strings(&[
        (ItemReason::Type, "type"),
        (ItemReason::Size, "size"),
        (ItemReason::StorageFull, "storage_full"),
        (ItemReason::Abandoned, "abandoned"),
        (ItemReason::Stalled, "stalled"),
        (ItemReason::Cancelled, "cancelled"),
        (ItemReason::Unauthorized, "unauthorized"),
        (ItemReason::InvalidImage, "invalid_image"),
        (ItemReason::Integrity, "integrity"),
        (ItemReason::Duplicate, "duplicate"),
    ]);
    enum_strings(&[
        (JoinFailure::WrongPassword, "wrong_password"),
        (JoinFailure::TokenInvalid, "token_invalid"),
        (JoinFailure::Removed, "removed"),
        (JoinFailure::EventEnded, "event_ended"),
        (JoinFailure::BrowserNotAllowed, "browser_not_allowed"),
        (JoinFailure::RateLimited, "rate_limited"),
        (JoinFailure::LocalNetworkDenied, "local_network_denied"),
        (JoinFailure::FingerprintMismatch, "fingerprint_mismatch"),
        (JoinFailure::Unreachable, "unreachable"),
        (JoinFailure::TokenExpired, "token_expired"),
    ]);
    enum_strings(&[
        (ApiCode::SessionInvalid, "session_invalid"),
        (ApiCode::RoleDenied, "role_denied"),
        (ApiCode::NotFound, "not_found"),
        (ApiCode::NotAvailable, "not_available"),
        (ApiCode::OffsetMismatch, "offset_mismatch"),
        (ApiCode::RateLimited, "rate_limited"),
        (ApiCode::BadRequest, "bad_request"),
        (ApiCode::ProtoUnsupported, "proto_unsupported"),
    ]);
}

#[test]
fn ws_and_config_enums() {
    enum_strings(&[
        (ChangeType::EventUpdated, "event_updated"),
        (ChangeType::EventExtended, "event_extended"),
        (ChangeType::ItemAvailable, "item_available"),
        (ChangeType::ItemDeleted, "item_deleted"),
        (ChangeType::AttendeeJoined, "attendee_joined"),
        (ChangeType::AttendeeRemoved, "attendee_removed"),
        (ChangeType::PostPublished, "post_published"),
        (ChangeType::PostDeleted, "post_deleted"),
        (ChangeType::CountsUpdated, "counts_updated"),
        (ChangeType::EventEnded, "event_ended"),
    ]);
    enum_strings(&[
        (ByeReason::Removed, "removed"),
        (ByeReason::EventEnded, "event_ended"),
        (ByeReason::SessionExpired, "session_expired"),
    ]);
    enum_strings(&[(NoticeKind::ExpiringSoon, "expiring_soon")]);
    enum_strings(&[(ReachabilityTestMode::Full, "full")]);
    enum_strings(&[(CompatibleCopyStrategy::OnDemand, "on_demand")]);
}

// ---------------------------------------------------------------- §5.1

#[test]
fn transport_strings() {
    roundtrip(&format!("\"{EVT}\""), &evt());
    roundtrip(
        "\"q6urq6urq6urq6urq6urqw\"",
        &JoinToken::from_bytes([0xab; 16]),
    );
    let mut d = [0u8; 32];
    for (i, b) in d.iter_mut().enumerate() {
        *b = i as u8;
    }
    roundtrip(
        "\"AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8\"",
        &Fingerprint::from_digest(d),
    );
}

// ---------------------------------------------------------------- §5.2

#[test]
fn error_body() {
    roundtrip(
        r#"{"error": {"code": "not_found", "message": "no such item", "retryable": false}}"#,
        &ErrorBody::api(ApiCode::NotFound, "no such item"),
    );
    roundtrip(
        r#"{"error": {"code": "rate_limited", "message": "rate limited", "retryable": true, "retry_after_ms": 1500}}"#,
        &ErrorBody::rate_limited(1500),
    );
    roundtrip(
        r#"{"error": {"code": "offset_mismatch", "message": "upload offset mismatch", "retryable": true, "received_bytes": 2097152}}"#,
        &ErrorBody::offset_mismatch(2_097_152),
    );
    roundtrip(
        r#"{"error": {"code": "wrong_password", "message": "wrong password", "retryable": false}}"#,
        &ErrorBody::join(JoinFailure::WrongPassword, "wrong password"),
    );
}

// ---------------------------------------------------------------- §5.3

#[test]
fn meta_response() {
    let mut meta = MetaResponse {
        event_id: evt(),
        name: "Rooftop".into(),
        host_name: "Maya".into(),
        starts_at: 1_763_190_000_000,
        ends_at: 1_763_204_400_000,
        expires_at: 1_763_290_800_000,
        access_type: AccessType::Password,
        kind: EventKind::Event,
        status: EventStatus::Active,
        browser_guests_allowed: true,
        proto: 1,
        counts: None,
    };
    roundtrip(
        r#"{"event_id": "evt_3f9a0c12b7de", "name": "Rooftop", "host_name": "Maya",
            "starts_at": 1763190000000, "ends_at": 1763204400000, "expires_at": 1763290800000,
            "access_type": "password", "kind": "event", "status": "active",
            "browser_guests_allowed": true, "proto": 1}"#,
        &meta,
    );
    meta.access_type = AccessType::Open;
    meta.counts = Some(MetaCounts {
        photos: 120,
        attendees: 14,
    });
    roundtrip(
        r#"{"event_id": "evt_3f9a0c12b7de", "name": "Rooftop", "host_name": "Maya",
            "starts_at": 1763190000000, "ends_at": 1763204400000, "expires_at": 1763290800000,
            "access_type": "open", "kind": "event", "status": "active",
            "browser_guests_allowed": true, "proto": 1, "counts": {"photos": 120, "attendees": 14}}"#,
        &meta,
    );
}

fn credential() -> Credential {
    format!("v1.{PAYLOAD_SEG}.{SIG_1}").parse().unwrap()
}

#[test]
fn join_request_full_and_minimal() {
    let full = JoinRequest {
        client: ClientInfo {
            kind: ClientKind::Ios,
            app_version: "1.0.0".into(),
        },
        credential: Some(format!("v1.{PAYLOAD_SEG}.{SIG_1}")),
        proof: Some(serde_json::json!({
            "timestamp": 1_763_200_000_000_i64,
            "signature": SIG_1,
        })),
        guest: None,
        join_token: Some(JoinToken::from_bytes([0xab; 16])),
        password: Some("hunter2".into()),
    };
    roundtrip(
        &format!(
            r#"{{"client": {{"kind": "ios", "app_version": "1.0.0"}},
                "credential": "v1.{PAYLOAD_SEG}.{SIG_1}",
                "proof": {{"timestamp": 1763200000000, "signature": "{SIG_1}"}},
                "join_token": "q6urq6urq6urq6urq6urqw", "password": "hunter2"}}"#
        ),
        &full,
    );
    let guest = JoinRequest {
        client: ClientInfo {
            kind: ClientKind::Web,
            app_version: "web".into(),
        },
        credential: None,
        proof: None,
        guest: Some(GuestInfo {
            guest_id: ATT.into(),
            display_name: "Ana".into(),
        }),
        join_token: None,
        password: None,
    };
    roundtrip(
        &format!(
            r#"{{"client": {{"kind": "web", "app_version": "web"}},
                "guest": {{"guest_id": "{ATT}", "display_name": "Ana"}}}}"#
        ),
        &guest,
    );
}

const CONFIG_JSON: &str = r#"{
    "browser_guests_available": true,
    "quick_session_enabled": true,
    "reachability_test_mode": "full",
    "compatible_copy_strategy": "on_demand",
    "desktop_download_all": true,
    "transfer_config": {"chunk_bytes": 1048576, "stall_timeout_ms": 120000, "transfer_retries": 3, "backoff_base_ms": 500},
    "outbox_config": {"max_attempts": 10, "max_age_ms": 86400000},
    "rate_limits": {"uploads_per_window": 30, "upload_window_ms": 300000}
}"#;

#[test]
fn join_response_and_session() {
    roundtrip(
        &format!(
            r#"{{"session_token": "tok", "session_expires_at": 1763243200000,
                "attendee": {{"id": "{ATT}", "role": "uploader", "is_host_device": false}},
                "config": {CONFIG_JSON}}}"#
        ),
        &JoinResponse {
            session_token: "tok".into(),
            session_expires_at: 1_763_243_200_000,
            attendee: JoinedAttendee {
                id: ATT.into(),
                role: Role::Uploader,
                is_host_device: false,
            },
            config: Config::default(),
        },
    );
    roundtrip(
        r#"{"session_token": "tok2", "session_expires_at": 1763286400000}"#,
        &SessionRefreshResponse {
            session_token: "tok2".into(),
            session_expires_at: 1_763_286_400_000,
        },
    );
}

#[test]
fn items_and_uploads() {
    roundtrip(
        &format!(
            r#"{{"submission_id": "{SUB}", "content_type": "image/heic", "size_bytes": 3145728,
                "sha256": "{SHA}", "original_name": "IMG_0042.HEIC"}}"#
        ),
        &CreateItemRequest {
            submission_id: SUB.into(),
            content_type: "image/heic".into(),
            size_bytes: 3_145_728,
            sha256: SHA.into(),
            original_name: "IMG_0042.HEIC".into(),
        },
    );
    roundtrip(
        &format!(
            r#"{{"submission_id": "{SUB}", "item_id": "{ITEM}", "state": "UPLOADING",
                "received_bytes": 0, "upload_token": "up", "chunk_bytes": 1048576}}"#
        ),
        &CreateItemResponse {
            submission_id: SUB.into(),
            item_id: ITEM.into(),
            state: ItemState::Uploading,
            reason_code: None,
            retryable: None,
            received_bytes: 0,
            upload_token: Some("up".into()),
            chunk_bytes: Some(1_048_576),
            duplicate_of: None,
        },
    );
    roundtrip(
        &format!(
            r#"{{"submission_id": "{SUB}", "item_id": "{ITEM}", "state": "FAILED",
                "reason_code": "duplicate", "retryable": false, "received_bytes": 0,
                "duplicate_of": "{POST_ID}"}}"#
        ),
        &CreateItemResponse {
            submission_id: SUB.into(),
            item_id: ITEM.into(),
            state: ItemState::Failed,
            reason_code: Some(ItemReason::Duplicate),
            retryable: Some(false),
            received_bytes: 0,
            upload_token: None,
            chunk_bytes: None,
            duplicate_of: Some(POST_ID.into()),
        },
    );
    roundtrip(
        r#"{"received_bytes": 2097152, "state": "UPLOADING"}"#,
        &UploadChunkResponse {
            received_bytes: 2_097_152,
            state: ItemState::Uploading,
        },
    );
    roundtrip(
        &format!(
            r#"{{"state": "FAILED", "reason_code": "stalled", "retryable": true,
                "received_bytes": 1048576, "item_id": "{ITEM}"}}"#
        ),
        &SubmissionStatus {
            state: ItemState::Failed,
            reason_code: Some(ItemReason::Stalled),
            retryable: true,
            received_bytes: 1_048_576,
            item_id: ITEM.into(),
        },
    );
    roundtrip(
        &format!(
            r#"{{"state": "AVAILABLE", "reason_code": null, "retryable": false,
                "received_bytes": 3145728, "item_id": "{ITEM}"}}"#
        ),
        &SubmissionStatus {
            state: ItemState::Available,
            reason_code: None,
            retryable: false,
            received_bytes: 3_145_728,
            item_id: ITEM.into(),
        },
    );
    roundtrip(
        r#"{"version": "compatible"}"#,
        &SavedRequest {
            version: DownloadVersion::Compatible,
        },
    );
    roundtrip(
        r#"{"text": "Cake at 9"}"#,
        &CreatePostRequest {
            text: "Cake at 9".into(),
        },
    );
}

// ---------------------------------------------------------------- §5.4 and §6

fn item() -> Item {
    Item {
        id: ITEM.into(),
        uploader_id: ATT.into(),
        content_type: "image/heic".into(),
        size_bytes: 3_145_728,
        sha256: SHA.into(),
        width: 4032,
        height: 3024,
        has_hdr_gain_map: true,
        compatible_same_as_original: false,
        original_name: "IMG_0042.HEIC".into(),
        available_at: 1_763_200_000_000,
    }
}

fn item_json() -> String {
    format!(
        r#"{{"id": "{ITEM}", "uploader_id": "{ATT}", "content_type": "image/heic",
            "size_bytes": 3145728, "sha256": "{SHA}", "width": 4032, "height": 3024,
            "has_hdr_gain_map": true, "compatible_same_as_original": false,
            "original_name": "IMG_0042.HEIC", "available_at": 1763200000000}}"#
    )
}

fn attendee() -> Attendee {
    Attendee {
        id: ATT.into(),
        display_name: "Leo".into(),
        role: Role::Uploader,
        verified: true,
        client_kind: ClientKind::Android,
        is_host_device: false,
    }
}

const ATTENDEE_JSON: &str = r#"{"id": "0b6f2c1d-3e4a-4b5c-9d6e-7f8091a2b3c4", "display_name": "Leo",
    "role": "uploader", "verified": true, "client_kind": "android", "is_host_device": false}"#;

fn post(kind: PostKind, author: Option<&str>) -> Post {
    Post {
        id: POST_ID.into(),
        kind,
        author_id: author.map(Into::into),
        text: "Cake at 9".into(),
        created_at: 1_763_200_100_000,
    }
}

fn event() -> Event {
    Event {
        id: evt(),
        name: "Rooftop".into(),
        description: "Bring a jacket".into(),
        links: vec!["https://example.com/playlist".into()],
        starts_at: 1_763_190_000_000,
        ends_at: 1_763_204_400_000,
        expires_at: 1_763_290_800_000,
        retention_hours: 24,
        max_ends_at: 1_763_795_000_000,
        extension_count: 1,
        access_type: AccessType::Invite,
        status: EventStatus::Active,
        kind: EventKind::Quick,
        host_display_name: "Maya".into(),
        browser_guests_allowed: false,
        storage_limit_bytes: 5_368_709_120,
        created_at: 1_763_180_000_000,
    }
}

const EVENT_JSON: &str = r#"{"id": "evt_3f9a0c12b7de", "name": "Rooftop", "description": "Bring a jacket",
    "links": ["https://example.com/playlist"], "starts_at": 1763190000000, "ends_at": 1763204400000,
    "expires_at": 1763290800000, "retention_hours": 24, "max_ends_at": 1763795000000,
    "extension_count": 1, "access_type": "invite", "status": "active", "kind": "quick",
    "host_display_name": "Maya", "browser_guests_allowed": false,
    "storage_limit_bytes": 5368709120, "created_at": 1763180000000}"#;

#[test]
fn records() {
    roundtrip(EVENT_JSON, &event());
    roundtrip(ATTENDEE_JSON, &attendee());
    roundtrip(&item_json(), &item());
    roundtrip(
        &format!(
            r#"{{"id": "{POST_ID}", "kind": "host", "author_id": "{ATT}", "text": "Cake at 9", "created_at": 1763200100000}}"#
        ),
        &post(PostKind::Host, Some(ATT)),
    );
    roundtrip(
        &format!(
            r#"{{"id": "{POST_ID}", "kind": "system", "author_id": null, "text": "Cake at 9", "created_at": 1763200100000}}"#
        ),
        &post(PostKind::System, None),
    );
    roundtrip(
        &format!(
            r#"{{"id": "{ITEM}", "download_count": 3, "hot_score": 2.5, "hot_score_updated_at": 1763200200000}}"#
        ),
        &ItemCounts {
            id: ITEM.into(),
            download_count: 3,
            hot_score: 2.5,
            hot_score_updated_at: 1_763_200_200_000,
        },
    );
    roundtrip(
        &format!(
            r#"{{"id": "{ITEM}", "uploader_id": "{ATT}", "content_type": "image/heic",
                "size_bytes": 3145728, "sha256": "{SHA}", "width": 4032, "height": 3024,
                "has_hdr_gain_map": true, "compatible_same_as_original": false,
                "original_name": "IMG_0042.HEIC", "available_at": 1763200000000,
                "download_count": 0, "hot_score": 0.0, "hot_score_updated_at": 1763200000000}}"#
        ),
        &SnapshotItem {
            item: item(),
            download_count: 0,
            hot_score: 0.0,
            hot_score_updated_at: 1_763_200_000_000,
        },
    );
}

#[test]
fn client_messages() {
    roundtrip(
        r#"{"t": "hello", "session_token": "tok", "version": 41}"#,
        &ClientMessage::Hello {
            session_token: "tok".into(),
            version: 41,
        },
    );
    roundtrip(
        r#"{"t": "resync", "version": 41}"#,
        &ClientMessage::Resync { version: 41 },
    );
    roundtrip(r#"{"t": "ping"}"#, &ClientMessage::Ping);
}

/// One `(type, payload literal, typed body)` per change type in the digest table.
fn change_cases() -> Vec<(&'static str, String, ChangeBody)> {
    vec![
        (
            "event_updated",
            r#"{"name": "Rooftop party", "ends_at": 1763208000000}"#.into(),
            ChangeBody::EventUpdated(EventPatch {
                name: Some("Rooftop party".into()),
                ends_at: Some(1_763_208_000_000),
                ..EventPatch::default()
            }),
        ),
        (
            "event_extended",
            r#"{"old_expires_at": 1763290800000, "new_expires_at": 1763377200000, "ends_at": 1763290800000, "retention_hours": 24}"#.into(),
            ChangeBody::EventExtended(EventExtended {
                old_expires_at: 1_763_290_800_000,
                new_expires_at: 1_763_377_200_000,
                ends_at: 1_763_290_800_000,
                retention_hours: 24,
            }),
        ),
        ("item_available", item_json(), ChangeBody::ItemAvailable(item())),
        (
            "item_deleted",
            format!(r#"{{"id": "{ITEM}"}}"#),
            ChangeBody::ItemDeleted(Tombstone { id: ITEM.into() }),
        ),
        (
            "attendee_joined",
            ATTENDEE_JSON.into(),
            ChangeBody::AttendeeJoined(attendee()),
        ),
        (
            "attendee_removed",
            format!(r#"{{"id": "{ATT}"}}"#),
            ChangeBody::AttendeeRemoved(Tombstone { id: ATT.into() }),
        ),
        (
            "post_published",
            format!(
                r#"{{"id": "{POST_ID}", "kind": "host", "author_id": "{ATT}", "text": "Cake at 9", "created_at": 1763200100000}}"#
            ),
            ChangeBody::PostPublished(post(PostKind::Host, Some(ATT))),
        ),
        (
            "post_deleted",
            format!(r#"{{"id": "{POST_ID}"}}"#),
            ChangeBody::PostDeleted(Tombstone { id: POST_ID.into() }),
        ),
        (
            "counts_updated",
            format!(
                r#"[{{"id": "{ITEM}", "download_count": 3, "hot_score": 2.5, "hot_score_updated_at": 1763200200000}}]"#
            ),
            ChangeBody::CountsUpdated(vec![ItemCounts {
                id: ITEM.into(),
                download_count: 3,
                hot_score: 2.5,
                hot_score_updated_at: 1_763_200_200_000,
            }]),
        ),
        ("event_ended", "{}".into(), ChangeBody::EventEnded(Empty {})),
    ]
}

#[test]
fn every_change_type_round_trips() {
    let cases = change_cases();
    assert_eq!(
        cases.len(),
        ChangeType::ALL.len(),
        "one case per change type"
    );
    for (i, (ty, payload, body)) in cases.into_iter().enumerate() {
        assert_eq!(body.change_type().as_str(), ty);
        let version = 100 + i as u64;
        let change = Change {
            version,
            body,
            ts: 1_763_200_300_000,
        };
        roundtrip(
            &format!(
                r#"{{"version": {version}, "type": "{ty}", "payload": {payload}, "ts": 1763200300000}}"#
            ),
            &change,
        );
        // Live form: the envelope fields sit beside "t".
        roundtrip(
            &format!(
                r#"{{"t": "change", "version": {version}, "type": "{ty}", "payload": {payload}, "ts": 1763200300000}}"#
            ),
            &ServerMessage::Change(change.clone()),
        );
        // Catch-up form.
        roundtrip(
            &format!(
                r#"{{"t": "changes", "changes": [{{"version": {version}, "type": "{ty}", "payload": {payload}, "ts": 1763200300000}}]}}"#
            ),
            &ServerMessage::Changes {
                changes: vec![change],
            },
        );
    }
}

#[test]
fn server_messages() {
    roundtrip(
        &format!(
            r#"{{"t": "snapshot", "version": 41, "event": {EVENT_JSON},
                "attendees": [{ATTENDEE_JSON}],
                "items": [{{"id": "{ITEM}", "uploader_id": "{ATT}", "content_type": "image/heic",
                    "size_bytes": 3145728, "sha256": "{SHA}", "width": 4032, "height": 3024,
                    "has_hdr_gain_map": true, "compatible_same_as_original": false,
                    "original_name": "IMG_0042.HEIC", "available_at": 1763200000000,
                    "download_count": 2, "hot_score": 1.25, "hot_score_updated_at": 1763200000000}}],
                "posts": [{{"id": "{POST_ID}", "kind": "system", "author_id": null, "text": "Cake at 9", "created_at": 1763200100000}}],
                "config": {CONFIG_JSON}}}"#
        ),
        &ServerMessage::Snapshot {
            version: 41,
            event: event(),
            attendees: vec![attendee()],
            items: vec![SnapshotItem {
                item: item(),
                download_count: 2,
                hot_score: 1.25,
                hot_score_updated_at: 1_763_200_000_000,
            }],
            posts: vec![post(PostKind::System, None)],
            config: Config::default(),
        },
    );
    roundtrip(
        &format!(
            r#"{{"t": "upload_state", "submission_id": "{SUB}", "state": "FAILED",
                "reason_code": "integrity", "retryable": true, "received_bytes": 0, "item_id": "{ITEM}"}}"#
        ),
        &ServerMessage::UploadState {
            submission_id: SUB.into(),
            state: ItemState::Failed,
            reason_code: Some(ItemReason::Integrity),
            retryable: true,
            received_bytes: 0,
            item_id: ITEM.into(),
        },
    );
    roundtrip(
        r#"{"t": "notice", "kind": "expiring_soon", "expires_at": 1763290800000}"#,
        &ServerMessage::Notice {
            kind: NoticeKind::ExpiringSoon,
            expires_at: Some(1_763_290_800_000),
        },
    );
    for (s, reason) in [
        ("removed", ByeReason::Removed),
        ("event_ended", ByeReason::EventEnded),
        ("session_expired", ByeReason::SessionExpired),
    ] {
        roundtrip(
            &format!(r#"{{"t": "bye", "reason": "{s}"}}"#),
            &ServerMessage::Bye { reason },
        );
    }
    roundtrip(r#"{"t": "pong"}"#, &ServerMessage::Pong);
}

// ---------------------------------------------------------------- §5.5

fn account() -> Account {
    Account {
        id: "2e3f4a5b-6c7d-4e8f-9a0b-1c2d3e4f5a6b".into(),
        display_name: "Leo".into(),
        email: "leo@example.com".into(),
        created_at: 1_763_100_000_000,
    }
}

const ACCOUNT_JSON: &str = r#"{"id": "2e3f4a5b-6c7d-4e8f-9a0b-1c2d3e4f5a6b", "display_name": "Leo",
    "email": "leo@example.com", "created_at": 1763100000000}"#;

#[test]
fn service_bodies() {
    let key = DevicePublicKey::from_bytes([9; 32]);
    roundtrip(
        r#"{"email": "leo@example.com"}"#,
        &AuthStartRequest {
            email: "leo@example.com".into(),
        },
    );
    roundtrip(
        &format!(
            r#"{{"email": "leo@example.com", "code": "123456", "display_name": "Leo", "device_pubkey": "{PUBKEY_9}"}}"#
        ),
        &AuthVerifyRequest {
            email: "leo@example.com".into(),
            code: "123456".into(),
            display_name: Some("Leo".into()),
            device_pubkey: key,
        },
    );
    roundtrip(
        &format!(
            r#"{{"email": "leo@example.com", "code": "123456", "device_pubkey": "{PUBKEY_9}"}}"#
        ),
        &AuthVerifyRequest {
            email: "leo@example.com".into(),
            code: "123456".into(),
            display_name: None,
            device_pubkey: key,
        },
    );
    roundtrip(
        &format!(
            r#"{{"account": {ACCOUNT_JSON}, "credential": "v1.{PAYLOAD_SEG}.{SIG_1}", "refresh_token": "rt"}}"#
        ),
        &AuthVerifyResponse {
            account: account(),
            credential: credential(),
            refresh_token: "rt".into(),
        },
    );
    roundtrip(
        &format!(r#"{{"refresh_token": "rt", "device_pubkey": "{PUBKEY_9}"}}"#),
        &CredentialRefreshRequest {
            refresh_token: "rt".into(),
            device_pubkey: key,
        },
    );
    roundtrip(
        &format!(r#"{{"credential": "v1.{PAYLOAD_SEG}.{SIG_1}"}}"#),
        &CredentialRefreshResponse {
            credential: credential(),
        },
    );
    roundtrip(
        r#"{"display_name": "Leo M"}"#,
        &AccountPatchRequest {
            display_name: "Leo M".into(),
        },
    );
    roundtrip(
        &format!(r#"{{"account": {ACCOUNT_JSON}, "credential": "v1.{PAYLOAD_SEG}.{SIG_1}"}}"#),
        &AccountPatchResponse {
            account: account(),
            credential: credential(),
        },
    );
    roundtrip(
        &format!(
            r#"{{"kid": "k1", "account_id": "2e3f4a5b-6c7d-4e8f-9a0b-1c2d3e4f5a6b", "display_name": "Leo",
                "role": "verified", "device_pubkey": "{PUBKEY_9}", "issued_at": 1763200000000, "expires_at": 1763804800000}}"#
        ),
        credential().payload(),
    );
    roundtrip(
        &format!(r#"{{"timestamp": 1763200000000, "signature": "{SIG_1}"}}"#),
        &JoinProof {
            timestamp: 1_763_200_000_000,
            signature: Signature::from_bytes([1; 64]),
        },
    );
}

// ---------------------------------------------------------------- §10, §11

#[test]
fn config_round_trips() {
    roundtrip(CONFIG_JSON, &Config::default());
    roundtrip(
        r#"{"chunk_bytes": 262144, "stall_timeout_ms": 60000, "transfer_retries": 5, "backoff_base_ms": 250}"#,
        &TransferConfig {
            chunk_bytes: 262_144,
            stall_timeout_ms: 60_000,
            transfer_retries: 5,
            backoff_base_ms: 250,
        },
    );
    roundtrip(
        r#"{"max_attempts": 4, "max_age_ms": 3600000}"#,
        &OutboxConfig {
            max_attempts: 4,
            max_age_ms: 3_600_000,
        },
    );
    roundtrip(
        r#"{"uploads_per_window": 60, "upload_window_ms": 600000}"#,
        &RateLimits {
            uploads_per_window: 60,
            upload_window_ms: 600_000,
        },
    );
}
