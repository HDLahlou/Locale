//! Host API (digest §5.3): routes, headers, request and response bodies, rate limits.
//!
//! Every route is under `/e/{event_id}`. Auth is `Authorization: Bearer
//! <session_token>`; the guest page also gets the token as a cookie scoped to
//! `/e/{event_id}`. Every request carries `X-Locale-Proto: 1`.
//!
//! Host-device-only actions (edit, extend, shorten, remove attendee, end, convert
//! quick session) are not on the network API; the desktop shell calls host-core
//! functions in process (MVP §5.11).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::codes::ItemReason;
use crate::config::Config;
use crate::model::{
    AccessType, AttendeeId, ClientKind, DownloadVersion, EventKind, EventStatus, GuestId, ItemId,
    ItemState, Role, Sha256Hex, SubmissionId,
};
use crate::service::JoinProof;
use crate::transport::{EventId, JoinToken};

/// Header names.
pub mod headers {
    /// Protocol version header; value [`crate::PROTO_VERSION`] on every request.
    pub const X_LOCALE_PROTO: &str = "X-Locale-Proto";
    /// Bearer session token.
    pub const AUTHORIZATION: &str = "Authorization";
    /// Prefix of the `Authorization` value.
    pub const BEARER_PREFIX: &str = "Bearer ";
    /// Upload token from `POST /items`, sent on every chunk.
    pub const UPLOAD_TOKEN: &str = "Upload-Token";
    /// Byte offset of a chunk (`PUT`), or the received byte count (`HEAD` response).
    pub const UPLOAD_OFFSET: &str = "Upload-Offset";
    /// SHA-256 (lowercase hex) of the file being downloaded.
    pub const X_LOCALE_SHA256: &str = "X-Locale-Sha256";
    /// Join token presented to `/meta` so invite events return `counts`.
    pub const X_JOIN_TOKEN: &str = "X-Join-Token";
    /// Download file name header.
    pub const CONTENT_DISPOSITION: &str = "Content-Disposition";
    /// Byte-range header on downloads.
    pub const RANGE: &str = "Range";
    /// All header names this contract defines, for the contract document.
    pub const ALL: &[&str] = &[
        X_LOCALE_PROTO,
        AUTHORIZATION,
        UPLOAD_TOKEN,
        UPLOAD_OFFSET,
        X_LOCALE_SHA256,
        X_JOIN_TOKEN,
        CONTENT_DISPOSITION,
        RANGE,
    ];
}

/// Name of the session cookie the guest page receives, scoped to `/e/{event_id}`.
pub const SESSION_COOKIE: &str = "locale_session";

/// Query string that marks a download for counting (DU-19).
pub const SAVE_QUERY: &str = "save=1";

/// Uploads allowed per user per window (compiled default; tunable in
/// [`crate::config::RateLimits`]).
pub const UPLOADS_PER_WINDOW: u32 = crate::config::UPLOADS_PER_WINDOW.default as u32;
/// Upload rate-limit window (compiled default; tunable).
pub const UPLOAD_WINDOW_MS: u64 = crate::config::UPLOAD_WINDOW_MS.default;
/// Joins allowed per minute per IP address.
pub const JOINS_PER_MINUTE_PER_IP: u32 = 10;
/// Password attempts allowed per minute per IP address.
pub const PASSWORD_ATTEMPTS_PER_MINUTE_PER_IP: u32 = 5;

/// Route templates and builders. Templates use `{name}` placeholders.
pub mod routes {
    use crate::model::{ItemId, PostId, SubmissionId};
    use crate::transport::EventId;

    /// Event base path; `GET` here serves the guest page.
    pub const EVENT_BASE: &str = "/e/{event_id}";
    /// `GET`: event metadata.
    pub const META: &str = "/e/{event_id}/meta";
    /// `POST`: join.
    pub const JOIN: &str = "/e/{event_id}/join";
    /// `POST`: refresh the session.
    pub const SESSION_REFRESH: &str = "/e/{event_id}/session/refresh";
    /// `GET`: WebSocket upgrade for index sync.
    pub const WS: &str = "/e/{event_id}/ws";
    /// `POST`: submit an item.
    pub const ITEMS: &str = "/e/{event_id}/items";
    /// `HEAD`, `PUT`, `DELETE`: one upload.
    pub const UPLOAD: &str = "/e/{event_id}/uploads/{submission_id}";
    /// `GET`: submission status.
    pub const SUBMISSION: &str = "/e/{event_id}/submissions/{submission_id}";
    /// `GET`: preview JPEG.
    pub const ITEM_PREVIEW: &str = "/e/{event_id}/items/{id}/preview";
    /// `GET`: original file.
    pub const ITEM_ORIGINAL: &str = "/e/{event_id}/items/{id}/original";
    /// `GET`: compatible JPEG.
    pub const ITEM_COMPATIBLE: &str = "/e/{event_id}/items/{id}/compatible";
    /// `POST`: report a save made from cache.
    pub const ITEM_SAVED: &str = "/e/{event_id}/items/{id}/saved";
    /// `DELETE`: delete an item.
    pub const ITEM: &str = "/e/{event_id}/items/{id}";
    /// `POST`: publish a post.
    pub const POSTS: &str = "/e/{event_id}/posts";
    /// `DELETE`: delete a post.
    pub const POST: &str = "/e/{event_id}/posts/{id}";

    /// `/e/{event_id}`.
    pub fn event_base(e: &EventId) -> String {
        format!("/e/{e}")
    }
    /// `/e/{event_id}/meta`.
    pub fn meta(e: &EventId) -> String {
        format!("/e/{e}/meta")
    }
    /// `/e/{event_id}/join`.
    pub fn join(e: &EventId) -> String {
        format!("/e/{e}/join")
    }
    /// `/e/{event_id}/session/refresh`.
    pub fn session_refresh(e: &EventId) -> String {
        format!("/e/{e}/session/refresh")
    }
    /// `/e/{event_id}/ws`.
    pub fn ws(e: &EventId) -> String {
        format!("/e/{e}/ws")
    }
    /// `/e/{event_id}/items`.
    pub fn items(e: &EventId) -> String {
        format!("/e/{e}/items")
    }
    /// `/e/{event_id}/uploads/{submission_id}`.
    pub fn upload(e: &EventId, submission_id: &SubmissionId) -> String {
        format!("/e/{e}/uploads/{submission_id}")
    }
    /// `/e/{event_id}/submissions/{submission_id}`.
    pub fn submission(e: &EventId, submission_id: &SubmissionId) -> String {
        format!("/e/{e}/submissions/{submission_id}")
    }
    /// `/e/{event_id}/items/{id}/preview`.
    pub fn item_preview(e: &EventId, id: &ItemId) -> String {
        format!("/e/{e}/items/{id}/preview")
    }
    /// `/e/{event_id}/items/{id}/original`, with `?save=1` when `save` is true.
    pub fn item_original(e: &EventId, id: &ItemId, save: bool) -> String {
        with_save(format!("/e/{e}/items/{id}/original"), save)
    }
    /// `/e/{event_id}/items/{id}/compatible`, with `?save=1` when `save` is true.
    pub fn item_compatible(e: &EventId, id: &ItemId, save: bool) -> String {
        with_save(format!("/e/{e}/items/{id}/compatible"), save)
    }
    /// `/e/{event_id}/items/{id}/saved`.
    pub fn item_saved(e: &EventId, id: &ItemId) -> String {
        format!("/e/{e}/items/{id}/saved")
    }
    /// `/e/{event_id}/items/{id}`.
    pub fn item(e: &EventId, id: &ItemId) -> String {
        format!("/e/{e}/items/{id}")
    }
    /// `/e/{event_id}/posts`.
    pub fn posts(e: &EventId) -> String {
        format!("/e/{e}/posts")
    }
    /// `/e/{event_id}/posts/{id}`.
    pub fn post(e: &EventId, id: &PostId) -> String {
        format!("/e/{e}/posts/{id}")
    }

    fn with_save(path: String, save: bool) -> String {
        if save {
            format!("{path}?{}", super::SAVE_QUERY)
        } else {
            path
        }
    }
}

/// One row of the host API table, for the contract document and tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Endpoint {
    /// HTTP method.
    pub method: &'static str,
    /// Route template.
    pub path: &'static str,
    /// Who may call it.
    pub who: &'static str,
    /// Request: body type or headers.
    pub request: &'static str,
    /// Success status and response: body type, headers or file.
    pub response: &'static str,
}

/// The host API table (§5.3), in digest order.
pub const ENDPOINTS: &[Endpoint] = &[
    Endpoint {
        method: "GET",
        path: routes::EVENT_BASE,
        who: "Anyone (plain HTTP)",
        request: "—",
        response: "200 guest web page (HTML)",
    },
    Endpoint {
        method: "GET",
        path: routes::META,
        who: "Anyone",
        request: "optional header X-Join-Token",
        response: "200 MetaResponse",
    },
    Endpoint {
        method: "POST",
        path: routes::JOIN,
        who: "Anyone",
        request: "JoinRequest",
        response: "200 JoinResponse",
    },
    Endpoint {
        method: "POST",
        path: routes::SESSION_REFRESH,
        who: "Attendee",
        request: "no body",
        response: "200 SessionRefreshResponse",
    },
    Endpoint {
        method: "GET",
        path: routes::WS,
        who: "Attendee",
        request: "WebSocket upgrade",
        response: "101; then ClientMessage / ServerMessage frames",
    },
    Endpoint {
        method: "POST",
        path: routes::ITEMS,
        who: "uploader, host",
        request: "CreateItemRequest (idempotent on submission_id)",
        response: "200 CreateItemResponse",
    },
    Endpoint {
        method: "HEAD",
        path: routes::UPLOAD,
        who: "Uploader",
        request: "—",
        response: "200, header Upload-Offset",
    },
    Endpoint {
        method: "PUT",
        path: routes::UPLOAD,
        who: "Uploader",
        request: "headers Upload-Token, Upload-Offset; body = one chunk",
        response: "200 UploadChunkResponse; wrong offset → 409 offset_mismatch with received_bytes",
    },
    Endpoint {
        method: "DELETE",
        path: routes::UPLOAD,
        who: "Uploader",
        request: "—",
        response: "204; item → FAILED cancelled",
    },
    Endpoint {
        method: "GET",
        path: routes::SUBMISSION,
        who: "Uploader",
        request: "—",
        response: "200 SubmissionStatus",
    },
    Endpoint {
        method: "GET",
        path: routes::ITEM_PREVIEW,
        who: "Attendee",
        request: "—",
        response: "200 image/jpeg",
    },
    Endpoint {
        method: "GET",
        path: routes::ITEM_ORIGINAL,
        who: "Attendee",
        request: "optional Range; ?save=1 counts a download",
        response: "200/206 file; headers X-Locale-Sha256, Content-Disposition (original_name)",
    },
    Endpoint {
        method: "GET",
        path: routes::ITEM_COMPATIBLE,
        who: "Attendee",
        request: "optional Range; ?save=1 counts a download",
        response:
            "200/206 JPEG; headers X-Locale-Sha256, Content-Disposition (original_name, .jpg)",
    },
    Endpoint {
        method: "POST",
        path: routes::ITEM_SAVED,
        who: "Attendee (apps)",
        request: "SavedRequest",
        response: "204",
    },
    Endpoint {
        method: "DELETE",
        path: routes::ITEM,
        who: "Uploader of it, host",
        request: "—",
        response: "204",
    },
    Endpoint {
        method: "POST",
        path: routes::POSTS,
        who: "host role",
        request: "CreatePostRequest (text 1–500 chars)",
        response: "200 Post",
    },
    Endpoint {
        method: "DELETE",
        path: routes::POST,
        who: "host role",
        request: "—",
        response: "204",
    },
];

/// Photo and attendee counts in `/meta`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MetaCounts {
    /// Available photos.
    pub photos: u64,
    /// Attendees.
    pub attendees: u64,
}

/// `GET /meta` response.
///
/// `counts` is present only for open events, or invite events when
/// `X-Join-Token` is valid; never for password events (D17).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MetaResponse {
    /// Event ID.
    pub event_id: EventId,
    /// Event name.
    pub name: String,
    /// The host's display name.
    pub host_name: String,
    /// Start time.
    pub starts_at: i64,
    /// End time.
    pub ends_at: i64,
    /// Expiry.
    pub expires_at: i64,
    /// Access type.
    pub access_type: AccessType,
    /// Kind.
    pub kind: EventKind,
    /// Status.
    pub status: EventStatus,
    /// Whether browser guests may join.
    pub browser_guests_allowed: bool,
    /// The host's protocol version.
    pub proto: u32,
    /// Counts, when allowed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counts: Option<MetaCounts>,
}

/// The joining client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ClientInfo {
    /// Client kind.
    pub kind: ClientKind,
    /// App version string.
    pub app_version: String,
}

/// A browser or signed-out guest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GuestInfo {
    /// Guest ID the client generated and keeps.
    pub guest_id: GuestId,
    /// Display name.
    pub display_name: String,
}

/// `POST /join` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct JoinRequest {
    /// The client.
    pub client: ClientInfo,
    /// Account credential text (`v1.<payload>.<signature>`), for verified
    /// attendees. Kept raw so a malformed credential never fails the request: the
    /// host treats it as a guest (`credential::evaluate_identity`, §4.3 step 4(a)).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential: Option<String>,
    /// Proof of possession of the credential's device key, a `JoinProof`
    /// (`{timestamp, signature}`). Kept as raw JSON so a malformed proof never
    /// fails the request: the host treats it as a guest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<JoinProof>")]
    pub proof: Option<serde_json::Value>,
    /// Guest identity, when joining without a credential.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guest: Option<GuestInfo>,
    /// Join token from the QR code or link.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub join_token: Option<JoinToken>,
    /// Event password.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

/// The attendee record returned by `POST /join`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct JoinedAttendee {
    /// Attendee ID.
    pub id: AttendeeId,
    /// Role.
    pub role: Role,
    /// The host's own device.
    pub is_host_device: bool,
}

/// `POST /join` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct JoinResponse {
    /// Session token.
    pub session_token: String,
    /// Session expiry.
    pub session_expires_at: i64,
    /// The attendee.
    pub attendee: JoinedAttendee,
    /// The event's frozen configuration.
    pub config: Config,
}

/// `POST /session/refresh` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SessionRefreshResponse {
    /// New session token.
    pub session_token: String,
    /// New expiry.
    pub session_expires_at: i64,
}

/// `POST /items` request. Idempotent on `submission_id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CreateItemRequest {
    /// Submission ID chosen by the uploader.
    pub submission_id: SubmissionId,
    /// MIME type.
    pub content_type: String,
    /// File size.
    pub size_bytes: u64,
    /// SHA-256 of the file, lowercase hex.
    pub sha256: Sha256Hex,
    /// File name.
    pub original_name: String,
}

/// `POST /items` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CreateItemResponse {
    /// Submission ID.
    pub submission_id: SubmissionId,
    /// Item ID.
    pub item_id: ItemId,
    /// Current state.
    pub state: ItemState,
    /// Reason, when `FAILED`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<ItemReason>,
    /// Whether the reason is retryable, when `FAILED`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retryable: Option<bool>,
    /// Bytes the host already has (resume offset).
    pub received_bytes: u64,
    /// Upload token for the chunk requests, when uploading.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upload_token: Option<String>,
    /// Chunk size to use, when uploading.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunk_bytes: Option<u64>,
    /// The existing item, for `duplicate` (DU-12).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duplicate_of: Option<ItemId>,
}

/// `PUT /uploads/{submission_id}` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UploadChunkResponse {
    /// Bytes received so far.
    pub received_bytes: u64,
    /// Current state.
    pub state: ItemState,
}

/// `GET /submissions/{submission_id}` response. `reason_code` is `null` unless `FAILED`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SubmissionStatus {
    /// Current state.
    pub state: ItemState,
    /// Reason, or `null`.
    pub reason_code: Option<ItemReason>,
    /// Whether the reason is retryable (`false` when there is none).
    pub retryable: bool,
    /// Bytes received so far.
    pub received_bytes: u64,
    /// Item ID.
    pub item_id: ItemId,
}

/// `POST /items/{id}/saved` request: a save made from the local cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SavedRequest {
    /// Which file was saved.
    pub version: DownloadVersion,
}

/// `POST /posts` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CreatePostRequest {
    /// Text, 1–500 characters.
    pub text: String,
}
