//! Renders the JSON Schema files and `CONTRACT.md` from the types and tables in
//! this crate. Nothing here touches the file system; the `proto-gen` binary
//! writes the output and a test compares it with the committed files.

use std::fmt::Write as _;

use schemars::{schema_for, JsonSchema};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::api::{self, headers, ENDPOINTS};
use crate::codes::{ApiCode, ErrorBody, ItemReason, JoinFailure};
use crate::config::{self, Config, FIXED, TUNABLES};
use crate::mdns::{self, TxtRecord};
use crate::model::{
    AccessType, Attendee, ClientKind, DownloadVersion, Event, EventKind, EventPatch, EventStatus,
    Item, ItemCounts, ItemState, Post, PostKind, Role, SessionScope, SnapshotItem,
};
use crate::service::{self, CredentialPayload, CredentialRole, DevicePublicKey, SERVICE_ENDPOINTS};
use crate::state::{Actor, TRANSITIONS};
use crate::transport::{self, EventId, Fingerprint, JoinDeepLink, JoinToken, JoinUrl};
use crate::ws::{
    ByeReason, Change, ChangeBody, ChangeType, ClientMessage, Empty, EventExtended, NoticeKind,
    ServerMessage, Tombstone,
};

/// The command that regenerates every generated file.
pub const REGENERATE_COMMAND: &str = "cargo run -p proto --bin proto-gen";

/// Directory of the schema files, relative to the crate root.
pub const SCHEMA_DIR: &str = "schema";

/// Path of the contract document, relative to the crate root.
pub const CONTRACT_PATH: &str = "CONTRACT.md";

/// Sorts object keys recursively, so output does not depend on whether
/// `serde_json`'s `preserve_order` feature is unified into the build.
fn canonical(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<(String, Value)> = map.into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            let mut out = Map::new();
            for (k, v) in entries {
                out.insert(k, canonical(v));
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.into_iter().map(canonical).collect()),
        other => other,
    }
}

fn pretty(value: Value) -> String {
    let mut s = serde_json::to_string_pretty(&canonical(value)).unwrap_or_default();
    s.push('\n');
    s
}

fn compact<T: Serialize>(value: &T) -> String {
    serde_json::to_string(&canonical(
        serde_json::to_value(value).unwrap_or(Value::Null),
    ))
    .unwrap_or_default()
}

fn schema<T: JsonSchema>(name: &str) -> (String, String) {
    let s = schema_for!(T);
    (format!("{SCHEMA_DIR}/{name}.json"), pretty(s.to_value()))
}

/// The JSON Schema files: `(path relative to the crate root, contents)`, one per
/// root type, sorted by path.
pub fn schema_files() -> Vec<(String, String)> {
    let mut files = vec![
        // §5.2
        schema::<ErrorBody>("error_body"),
        // §5.3
        schema::<api::MetaResponse>("meta_response"),
        schema::<api::JoinRequest>("join_request"),
        schema::<api::JoinResponse>("join_response"),
        schema::<api::SessionRefreshResponse>("session_refresh_response"),
        schema::<api::CreateItemRequest>("create_item_request"),
        schema::<api::CreateItemResponse>("create_item_response"),
        schema::<api::UploadChunkResponse>("upload_chunk_response"),
        schema::<api::SubmissionStatus>("submission_status"),
        schema::<api::SavedRequest>("saved_request"),
        schema::<api::CreatePostRequest>("create_post_request"),
        schema::<Post>("post"),
        // §5.4
        schema::<ClientMessage>("client_message"),
        schema::<ServerMessage>("server_message"),
        schema::<Change>("change"),
        // §5.5
        schema::<service::AuthStartRequest>("auth_start_request"),
        schema::<service::AuthVerifyRequest>("auth_verify_request"),
        schema::<service::AuthVerifyResponse>("auth_verify_response"),
        schema::<service::CredentialRefreshRequest>("credential_refresh_request"),
        schema::<service::CredentialRefreshResponse>("credential_refresh_response"),
        schema::<service::AccountPatchRequest>("account_patch_request"),
        schema::<service::AccountPatchResponse>("account_patch_response"),
        schema::<CredentialPayload>("credential_payload"),
        // §10, §11
        schema::<Config>("config"),
    ];
    files.sort();
    files
}

/// Every generated file: the schema files and `CONTRACT.md`.
pub fn generated_files() -> Vec<(String, String)> {
    let mut files = schema_files();
    files.push((CONTRACT_PATH.to_owned(), contract_markdown()));
    files
}

// ---------------------------------------------------------------------------
// Examples, built from the types so the document cannot drift from them.

fn ex_event_id() -> EventId {
    EventId::from_random_bytes([0x3f, 0x9a, 0x0c, 0x12, 0xb7, 0xde])
}

fn ex_fingerprint() -> Fingerprint {
    let mut d = [0u8; 32];
    d[..6].copy_from_slice(&[0x4f, 0x2a, 0x9c, 0x1e, 0x77, 0xb0]);
    for (i, b) in d.iter_mut().enumerate().skip(6) {
        *b = i as u8;
    }
    Fingerprint::from_digest(d)
}

fn ex_join_url() -> JoinUrl {
    JoinUrl {
        host: [192, 168, 1, 20].into(),
        port: transport::DEFAULT_PORT,
        event_id: ex_event_id(),
        join_token: JoinToken::from_bytes([7; 16]),
        fingerprint: ex_fingerprint(),
    }
}

fn ex_item() -> Item {
    Item {
        id: "7d1e5f0a-2b3c-4d5e-8f90-a1b2c3d4e5f6".into(),
        uploader_id: "0b6f2c1d-3e4a-4b5c-9d6e-7f8091a2b3c4".into(),
        content_type: "image/heic".into(),
        size_bytes: 3_145_728,
        sha256: "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08".into(),
        width: 4032,
        height: 3024,
        has_hdr_gain_map: true,
        compatible_same_as_original: false,
        original_name: "IMG_0042.HEIC".into(),
        available_at: 1_763_200_000_000,
    }
}

fn ex_attendee() -> Attendee {
    Attendee {
        id: "0b6f2c1d-3e4a-4b5c-9d6e-7f8091a2b3c4".into(),
        display_name: "Leo".into(),
        role: Role::Uploader,
        verified: true,
        client_kind: ClientKind::Ios,
        is_host_device: false,
    }
}

fn ex_post() -> Post {
    Post {
        id: "5a4b3c2d-1e0f-4a9b-8c7d-6e5f4a3b2c1d".into(),
        kind: PostKind::Host,
        author_id: Some("1c2d3e4f-5a6b-4c7d-8e9f-0a1b2c3d4e5f".into()),
        text: "Cake at 9".into(),
        created_at: 1_763_200_100_000,
    }
}

fn ex_event() -> Event {
    Event {
        id: ex_event_id(),
        name: "Rooftop".into(),
        description: String::new(),
        links: vec![],
        starts_at: 1_763_190_000_000,
        ends_at: 1_763_204_400_000,
        expires_at: 1_763_290_800_000,
        retention_hours: 24,
        max_ends_at: 1_763_795_000_000,
        extension_count: 0,
        access_type: AccessType::Open,
        status: EventStatus::Active,
        kind: EventKind::Event,
        host_display_name: "Maya".into(),
        browser_guests_allowed: true,
        storage_limit_bytes: config::DEFAULT_STORAGE_LIMIT_BYTES,
        created_at: 1_763_180_000_000,
    }
}

fn ex_changes() -> Vec<ChangeBody> {
    let id = |s: &str| Tombstone { id: s.into() };
    vec![
        ChangeBody::EventUpdated(EventPatch {
            name: Some("Rooftop party".into()),
            ..EventPatch::default()
        }),
        ChangeBody::EventExtended(EventExtended {
            old_expires_at: 1_763_290_800_000,
            new_expires_at: 1_763_377_200_000,
            ends_at: 1_763_290_800_000,
            retention_hours: 24,
        }),
        ChangeBody::ItemAvailable(ex_item()),
        ChangeBody::ItemDeleted(id("7d1e5f0a-2b3c-4d5e-8f90-a1b2c3d4e5f6")),
        ChangeBody::AttendeeJoined(ex_attendee()),
        ChangeBody::AttendeeRemoved(id("0b6f2c1d-3e4a-4b5c-9d6e-7f8091a2b3c4")),
        ChangeBody::PostPublished(ex_post()),
        ChangeBody::PostDeleted(id("5a4b3c2d-1e0f-4a9b-8c7d-6e5f4a3b2c1d")),
        ChangeBody::CountsUpdated(vec![ItemCounts {
            id: "7d1e5f0a-2b3c-4d5e-8f90-a1b2c3d4e5f6".into(),
            download_count: 3,
            hot_score: 2.5,
            hot_score_updated_at: 1_763_200_200_000,
        }]),
        ChangeBody::EventEnded(Empty {}),
    ]
}

fn ex_server_messages() -> Vec<ServerMessage> {
    let change = Change {
        version: 42,
        body: ChangeBody::ItemDeleted(Tombstone {
            id: "7d1e5f0a-2b3c-4d5e-8f90-a1b2c3d4e5f6".into(),
        }),
        ts: 1_763_200_300_000,
    };
    vec![
        ServerMessage::Snapshot {
            version: 41,
            event: ex_event(),
            attendees: vec![ex_attendee()],
            items: vec![SnapshotItem {
                item: ex_item(),
                download_count: 0,
                hot_score: 0.0,
                hot_score_updated_at: 1_763_200_000_000,
            }],
            posts: vec![ex_post()],
            config: Config::default(),
        },
        ServerMessage::Changes {
            changes: vec![change.clone()],
        },
        ServerMessage::Change(change),
        ServerMessage::UploadState {
            submission_id: "c0ffee00-1234-4abc-8def-0123456789ab".into(),
            state: ItemState::Available,
            reason_code: None,
            retryable: false,
            received_bytes: 3_145_728,
            item_id: "7d1e5f0a-2b3c-4d5e-8f90-a1b2c3d4e5f6".into(),
        },
        ServerMessage::Notice {
            kind: NoticeKind::ExpiringSoon,
            expires_at: Some(1_763_290_800_000),
        },
        ServerMessage::Bye {
            reason: ByeReason::EventEnded,
        },
        ServerMessage::Pong,
    ]
}

fn ex_client_messages() -> Vec<ClientMessage> {
    vec![
        ClientMessage::Hello {
            session_token: "s3ss10n-t0k3n".into(),
            version: 41,
        },
        ClientMessage::Resync { version: 41 },
        ClientMessage::Ping,
    ]
}

// ---------------------------------------------------------------------------
// Markdown.

fn yes_no(b: bool) -> &'static str {
    if b {
        "yes"
    } else {
        "no"
    }
}

fn codes<T: Copy>(all: &[T], f: impl Fn(T) -> &'static str) -> String {
    all.iter()
        .map(|v| format!("`{}`", f(*v)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn enum_row(out: &mut String, name: &str, open: bool, values: String, note: &str) {
    let kind = if open { "open" } else { "closed" };
    let _ = writeln!(out, "| `{name}` | {kind} | {values} | {note} |");
}

/// Renders `CONTRACT.md`.
pub fn contract_markdown() -> String {
    let mut o = String::new();
    let w = &mut o;
    macro_rules! line {
        ($($arg:tt)*) => { let _ = writeln!(w, $($arg)*); };
    }

    line!("# Locale interface contract");
    line!();
    line!("Generated from `crates/proto` by `{REGENERATE_COMMAND}`. Do not edit by hand; a test fails when this file is stale.");
    line!("The code is the contract (digest §5–§7, §10–§11). Changes go through the orchestrator.");
    line!();
    line!("Conventions: timestamps are Unix milliseconds, UTC; IDs are UUIDv4 strings unless stated; JSON uses snake_case; sizes are bytes (binary units); hashes are lowercase hex SHA-256. Base64url is always without padding.");
    line!();

    // §5.1
    line!("## 1. Transport, join URL, fingerprint (§5.1)");
    line!();
    line!("| Item | Value |");
    line!("|---|---|");
    line!(
        "| Protocol version | `{}` (header `{}`) |",
        crate::PROTO_VERSION,
        headers::X_LOCALE_PROTO
    );
    line!(
        "| Default port | `{}` (next free port if taken) |",
        transport::DEFAULT_PORT
    );
    line!(
        "| Port sniffing | first byte `0x{:02x}` → TLS, otherwise plain HTTP |",
        transport::TLS_FIRST_BYTE
    );
    line!(
        "| Event ID | `{}` + {} lowercase hex digits, e.g. `{}` |",
        transport::EVENT_ID_PREFIX,
        transport::EVENT_ID_HEX_LEN,
        ex_event_id()
    );
    line!(
        "| Join token | {} random bytes, base64url (22 chars) |",
        transport::JOIN_TOKEN_BYTES
    );
    line!(
        "| Fingerprint | SHA-256 of the certificate DER ({} bytes), base64url (43 chars) |",
        transport::FINGERPRINT_BYTES
    );
    line!(
        "| Host code | first {} hex digits of the fingerprint, uppercase, grouped: `{}` |",
        transport::HOST_CODE_HEX_LEN,
        ex_fingerprint().host_code()
    );
    line!();
    line!("Join URL: `http://<host-ip>:<port>/e/<event-id>#t=<join-token>&fp=<cert-fingerprint>`. IPv6 hosts are bracketed. Example:");
    line!();
    line!("```text\n{}\n```", ex_join_url());
    line!();
    line!("Deep link (DU-11): `{}<url-encoded join URL>`; everything except ASCII letters, digits and `-._~` is percent-encoded. Example:", transport::DEEP_LINK_PREFIX);
    line!();
    line!(
        "```text\n{}\n```",
        JoinDeepLink {
            join_url: ex_join_url()
        }
    );
    line!();

    // §5.2
    line!("## 2. Errors and reason codes (§5.2)");
    line!();
    line!("Error body (`ErrorBody`): `{{\"error\": {{\"code\", \"message\", \"retryable\", \"retry_after_ms\"?, \"received_bytes\"?}}}}`. `received_bytes` is set only for `offset_mismatch`. Example:");
    line!();
    line!(
        "```json\n{}\n```",
        compact(&ErrorBody::offset_mismatch(1_048_576))
    );
    line!();
    line!("### Item reason codes (`ItemReason`, open)");
    line!();
    line!("| Code | Retryable |");
    line!("|---|---|");
    for r in ItemReason::ALL {
        line!("| `{}` | {} |", r.as_str(), yes_no(r.retryable()));
    }
    line!();
    line!("### Join failures (`JoinFailure`)");
    line!();
    line!("| Code | Sent by | HTTP status | Retryable |");
    line!("|---|---|---|---|");
    for f in JoinFailure::ALL {
        let (by, status) = match f.http_status() {
            Some(s) => ("host", s.to_string()),
            None => ("client only", "—".to_owned()),
        };
        line!(
            "| `{}` | {by} | {status} | {} |",
            f.as_str(),
            yes_no(f.retryable())
        );
    }
    line!();
    line!("### General API codes (`ApiCode`)");
    line!();
    line!("| Code | HTTP status | Retryable |");
    line!("|---|---|---|");
    for c in ApiCode::ALL {
        line!(
            "| `{}` | {} | {} |",
            c.as_str(),
            c.http_status(),
            yes_no(c.retryable())
        );
    }
    line!();

    // §5.3
    line!("## 3. Host API (§5.3)");
    line!();
    line!("Auth: `{}: {}<session_token>`; the guest page also gets the token as cookie `{}` scoped to `/e/{{event_id}}`. Every request carries `{}: {}`.", headers::AUTHORIZATION, headers::BEARER_PREFIX, api::SESSION_COOKIE, headers::X_LOCALE_PROTO, crate::PROTO_VERSION);
    line!("Bodies are JSON; their schemas are in `schema/`. Host-device-only actions are not on the network API.");
    line!();
    line!("| Method | Path | Who | Request | Response |");
    line!("|---|---|---|---|---|");
    for e in ENDPOINTS {
        line!(
            "| `{}` | `{}` | {} | {} | {} |",
            e.method,
            e.path,
            e.who,
            e.request,
            e.response
        );
    }
    line!();
    line!(
        "Headers: {}.",
        headers::ALL
            .iter()
            .map(|h| format!("`{h}`"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    line!(
        "A download counts only with `?{}` or `POST …/saved` (DU-19).",
        api::SAVE_QUERY
    );
    line!();
    line!("Rate limits: uploads {} per {} ms per user (tunable), joins {} per minute per IP, password attempts {} per minute per IP.", api::UPLOADS_PER_WINDOW, api::UPLOAD_WINDOW_MS, api::JOINS_PER_MINUTE_PER_IP, api::PASSWORD_ATTEMPTS_PER_MINUTE_PER_IP);
    line!();

    // §5.4
    line!("## 4. Index sync messages (§5.4)");
    line!();
    line!("WebSocket JSON text frames tagged by `t`. `hello` must arrive within {} ms. Both message enums are open: an unknown `t` parses to `Unknown` and is ignored.", crate::ws::HELLO_TIMEOUT_MS);
    line!();
    line!("Client → host (`ClientMessage`):");
    line!();
    line!("```json");
    for m in ex_client_messages() {
        line!("{}", compact(&m));
    }
    line!("```");
    line!();
    line!("Host → client (`ServerMessage`):");
    line!();
    line!("```json");
    for m in ex_server_messages() {
        line!("{}", compact(&m));
    }
    line!("```");
    line!();
    line!(
        "Bye reasons (open): {}. Notice kinds (open): {}.",
        codes(ByeReason::ALL, ByeReason::as_str),
        codes(NoticeKind::ALL, NoticeKind::as_str)
    );
    line!();
    line!("### Change types (`ChangeType`, open)");
    line!();
    line!("Envelope `{{version, type, payload, ts}}`. An unknown `type` is kept as `ChangeBody::Unknown` with its raw payload.");
    line!();
    line!("| `type` | Payload example |");
    line!("|---|---|");
    for body in ex_changes() {
        line!(
            "| `{}` | `{}` |",
            body.type_str(),
            compact(&body.payload_json())
        );
    }
    line!();

    // §5.5
    line!("## 5. App service API (§5.5)");
    line!();
    line!("| Method | Path | Request | Response |");
    line!("|---|---|---|---|");
    for e in SERVICE_ENDPOINTS {
        line!(
            "| `{}` | `{}` | {} | {} |",
            e.method,
            e.path,
            e.request,
            e.response
        );
    }
    line!();
    line!(
        "Account-scoped requests authenticate with `Authorization: {}`.",
        service::SERVICE_AUTH_SCHEME
    );
    line!();
    line!("**Credential:** `{}.<base64url(payload JSON)>.<base64url(Ed25519 signature)>`. The signature covers the ASCII bytes `{}.<base64url(payload JSON)>`. The payload JSON is compact, fields in this order:", service::CREDENTIAL_VERSION, service::CREDENTIAL_VERSION);
    line!();
    let payload = CredentialPayload {
        kid: "k1".into(),
        account_id: "2e3f4a5b-6c7d-4e8f-9a0b-1c2d3e4f5a6b".into(),
        display_name: "Leo".into(),
        role: CredentialRole::Verified,
        device_pubkey: DevicePublicKey::from_bytes([9; 32]),
        issued_at: 1_763_200_000_000,
        expires_at: 1_763_200_000_000 + config::CREDENTIAL_LIFETIME_MS as i64,
    };
    line!(
        "```json\n{}\n```",
        serde_json::to_string(&payload).unwrap_or_default()
    );
    line!();
    line!("**Proof of possession:** `proof` in `POST /join` is `{{\"timestamp\", \"signature\"}}`; the device key signs these {} bytes:", service::PROOF_MESSAGE_LEN);
    line!();
    line!("| Part | Bytes | Encoding |");
    line!("|---|---|---|");
    line!(
        "| `\"{}\"` | {} | ASCII |",
        String::from_utf8_lossy(service::PROOF_DOMAIN),
        service::PROOF_DOMAIN.len()
    );
    line!("| event_id | 16 | ASCII of `evt_` + 12 hex |");
    line!("| cert_fingerprint | 32 | raw SHA-256 digest |");
    line!("| timestamp | 8 | Unix ms, big-endian signed 64-bit |");
    line!();
    line!(
        "Accepted within ±{} ms of the host clock.",
        service::PROOF_WINDOW_MS
    );
    line!();

    // §5.6
    line!("## 6. mDNS (§5.6)");
    line!();
    line!("Service type `{}`, instance name = event ID. Advertised for `open` and `password` events only. TXT keys: {}. Example:", mdns::SERVICE_TYPE, mdns::keys::ALL.iter().map(|k| format!("`{k}`")).collect::<Vec<_>>().join(", "));
    line!();
    let txt = TxtRecord {
        v: crate::PROTO_VERSION,
        id: ex_event_id(),
        name: "Rooftop".into(),
        host: "Maya".into(),
        access: AccessType::Open,
        st: 1_763_190_000_000,
        en: 1_763_204_400_000,
        fp: ex_fingerprint().host_code_hex(),
    };
    line!("```text");
    for (k, v) in txt.to_pairs() {
        line!("{k}={v}");
    }
    line!("```");
    line!();

    // §6
    line!("## 7. Enums (§6)");
    line!();
    line!("Open enums map unknown values to `Unknown` (receive-only, never sent). Closed enums reject unknown values.");
    line!();
    line!("| Enum | Kind | Values | Why |");
    line!("|---|---|---|---|");
    enum_row(
        w,
        "AccessType",
        AccessType::OPEN,
        codes(AccessType::ALL, AccessType::as_str),
        "access control",
    );
    enum_row(
        w,
        "EventStatus",
        EventStatus::OPEN,
        codes(EventStatus::ALL, EventStatus::as_str),
        "",
    );
    enum_row(
        w,
        "EventKind",
        EventKind::OPEN,
        codes(EventKind::ALL, EventKind::as_str),
        "",
    );
    enum_row(
        w,
        "Role",
        Role::OPEN,
        codes(Role::ALL, Role::as_str),
        "permissions",
    );
    enum_row(
        w,
        "ClientKind",
        ClientKind::OPEN,
        codes(ClientKind::ALL, ClientKind::as_str),
        "",
    );
    enum_row(
        w,
        "SessionScope",
        SessionScope::OPEN,
        codes(SessionScope::ALL, SessionScope::as_str),
        "permissions",
    );
    enum_row(
        w,
        "ItemState",
        ItemState::OPEN,
        codes(ItemState::ALL, ItemState::as_str),
        "state machine",
    );
    enum_row(
        w,
        "PostKind",
        PostKind::OPEN,
        codes(PostKind::ALL, PostKind::as_str),
        "",
    );
    enum_row(
        w,
        "DownloadVersion",
        DownloadVersion::OPEN,
        codes(DownloadVersion::ALL, DownloadVersion::as_str),
        "",
    );
    enum_row(
        w,
        "CredentialRole",
        CredentialRole::OPEN,
        codes(CredentialRole::ALL, CredentialRole::as_str),
        "",
    );
    enum_row(
        w,
        "ItemReason",
        ItemReason::OPEN,
        codes(ItemReason::ALL, ItemReason::as_str),
        "unknown = final, not retryable",
    );
    enum_row(
        w,
        "JoinFailure",
        JoinFailure::OPEN,
        codes(JoinFailure::ALL, JoinFailure::as_str),
        "travels as the string `error.code`",
    );
    enum_row(
        w,
        "ApiCode",
        ApiCode::OPEN,
        codes(ApiCode::ALL, ApiCode::as_str),
        "travels as the string `error.code`",
    );
    enum_row(
        w,
        "ChangeType",
        ChangeType::OPEN,
        codes(ChangeType::ALL, ChangeType::as_str),
        "unknown changes are skipped",
    );
    enum_row(
        w,
        "ClientMessage.t",
        true,
        "`hello`, `resync`, `ping`".into(),
        "unknown frames are ignored",
    );
    enum_row(
        w,
        "ServerMessage.t",
        true,
        "`snapshot`, `changes`, `change`, `upload_state`, `notice`, `bye`, `pong`".into(),
        "unknown frames are ignored",
    );
    enum_row(
        w,
        "ByeReason",
        ByeReason::OPEN,
        codes(ByeReason::ALL, ByeReason::as_str),
        "socket closes anyway",
    );
    enum_row(
        w,
        "NoticeKind",
        NoticeKind::OPEN,
        codes(NoticeKind::ALL, NoticeKind::as_str),
        "informational",
    );
    enum_row(
        w,
        "ReachabilityTestMode",
        config::ReachabilityTestMode::OPEN,
        codes(
            config::ReachabilityTestMode::ALL,
            config::ReachabilityTestMode::as_str,
        ),
        "unknown → compiled default",
    );
    enum_row(
        w,
        "CompatibleCopyStrategy",
        config::CompatibleCopyStrategy::OPEN,
        codes(
            config::CompatibleCopyStrategy::ALL,
            config::CompatibleCopyStrategy::as_str,
        ),
        "unknown → compiled default",
    );
    line!();

    // §7
    line!("## 8. Item state machine (§7)");
    line!();
    line!(
        "Any transition not in this table is illegal. `DELETED` is a tombstone in the change log."
    );
    line!();
    line!("| From | To | Trigger | Who | Reason codes | Retryable |");
    line!("|---|---|---|---|---|---|");
    for t in TRANSITIONS {
        let from = t.from.map_or("(none)", ItemState::as_str);
        let reasons = if t.reasons.is_empty() {
            String::new()
        } else {
            codes(t.reasons, ItemReason::as_str)
        };
        let retry = match t.retryable {
            Some(true) => "Yes",
            Some(false) => "No",
            None => "",
        };
        let who = match t.actor {
            Actor::Host => "Host",
            Actor::Sweeper => "Sweeper",
            Actor::HostOnRequest => "Host on request",
        };
        line!(
            "| {from} | {} | {} | {who} | {reasons} | {retry} |",
            t.to.as_str(),
            t.trigger
        );
    }
    line!();

    // §10, §11
    line!("## 9. Configuration (§10, §11)");
    line!();
    line!("`Config` is frozen at event start and sent at join and in the snapshot. Remote values outside the bounds, of the wrong type, or unknown enum values are ignored. Compiled defaults:");
    line!();
    line!(
        "```json\n{}\n```",
        pretty(serde_json::to_value(Config::default()).unwrap_or(Value::Null)).trim_end()
    );
    line!();
    line!("| Key | Default | Min | Max | Unit |");
    line!("|---|---|---|---|---|");
    for b in TUNABLES {
        line!(
            "| `{}` | {} | {} | {} | {} |",
            b.key,
            b.default,
            b.min,
            b.max,
            b.unit
        );
    }
    line!();
    line!("Fixed constants (`proto::config`):");
    line!();
    line!("| Constant | Value | Unit |");
    line!("|---|---|---|");
    for f in FIXED {
        line!("| `{}` | {} | {} |", f.name, f.value, f.unit);
    }
    line!();
    o
}
