//! Data model enums and the records that cross the wire (digest §6).
//!
//! Database-only columns (password hash, file paths, attempt counters, the join
//! token, storage path, port, config JSON) are not part of this crate.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::transport::EventId;

/// A UUIDv4 string identifying an attendee.
pub type AttendeeId = String;
/// A UUIDv4 string identifying an item.
pub type ItemId = String;
/// A UUIDv4 string chosen by the uploader for one submission (idempotency key and
/// correlation ID).
pub type SubmissionId = String;
/// A UUIDv4 string identifying a post.
pub type PostId = String;
/// A UUIDv4 string identifying an account on the app service.
pub type AccountId = String;
/// A UUIDv4 string a browser guest generates and keeps.
pub type GuestId = String;
/// Lowercase hex SHA-256.
pub type Sha256Hex = String;
/// Unix milliseconds, UTC.
pub type Millis = i64;

closed_enum! {
    /// How people get into an event. Closed: guessing would weaken access control.
    pub enum AccessType {
        /// Anyone on the network.
        Open = "open",
        /// Only with the join token from the QR code or link.
        Invite = "invite",
        /// Only with the password.
        Password = "password",
    }
}

closed_enum! {
    /// Event status. Closed.
    pub enum EventStatus {
        /// Running.
        Active = "active",
        /// The host app is closed; the event resumes on launch.
        Paused = "paused",
        /// Ended by the host or by expiry.
        Ended = "ended",
    }
}

closed_enum! {
    /// Event kind. Closed.
    pub enum EventKind {
        /// A planned event.
        Event = "event",
        /// A quick session with tighter limits.
        Quick = "quick",
    }
}

closed_enum! {
    /// Attendee role. Closed: it decides permissions.
    pub enum Role {
        /// The host: everything an uploader can do, plus posts and deleting any item.
        Host = "host",
        /// Can upload and view.
        Uploader = "uploader",
        /// Can view and download only.
        Viewer = "viewer",
    }
}

closed_enum! {
    /// The kind of client an attendee joined from (four values everywhere, C8). Closed.
    pub enum ClientKind {
        /// iPhone app.
        Ios = "ios",
        /// Android app.
        Android = "android",
        /// Desktop app.
        Desktop = "desktop",
        /// Guest web page.
        Web = "web",
    }
}

closed_enum! {
    /// What a session token allows. Closed.
    pub enum SessionScope {
        /// Every action the attendee's role allows.
        Full = "full",
        /// Read-only.
        Read = "read",
    }
}

closed_enum! {
    /// Item state (MVP §6). Wire values are uppercase, as the digest writes them.
    /// Closed: the state machine depends on it.
    pub enum ItemState {
        /// Submitted, waiting for the precheck.
        Pending = "PENDING",
        /// Receiving chunks.
        Uploading = "UPLOADING",
        /// All bytes received; integrity and content validation running.
        Validating = "VALIDATING",
        /// Visible to the event.
        Available = "AVAILABLE",
        /// Failed with a reason code.
        Failed = "FAILED",
        /// Deleted; a tombstone in the change log.
        Deleted = "DELETED",
    }
}

closed_enum! {
    /// Post kind. Closed.
    pub enum PostKind {
        /// Written by the host.
        Host = "host",
        /// Written by the app (for example "Event extended").
        System = "system",
    }
}

closed_enum! {
    /// Which file of an item a download or save refers to. Closed.
    pub enum DownloadVersion {
        /// The original file (after the location strip).
        Original = "original",
        /// The compatible JPEG copy.
        Compatible = "compatible",
    }
}

/// An event as attendees see it: the snapshot's `event`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Event {
    /// Event ID.
    pub id: EventId,
    /// Event name.
    pub name: String,
    /// Description; empty when none.
    #[serde(default)]
    pub description: String,
    /// Links the host added.
    #[serde(default)]
    pub links: Vec<String>,
    /// Start time.
    pub starts_at: i64,
    /// End time.
    pub ends_at: i64,
    /// When the event and its photos are deleted (end plus retention).
    pub expires_at: i64,
    /// Hours photos stay available after the end.
    pub retention_hours: u32,
    /// Latest end the host may extend to.
    pub max_ends_at: i64,
    /// How many times the event was extended.
    pub extension_count: u32,
    /// Access type.
    pub access_type: AccessType,
    /// Status.
    pub status: EventStatus,
    /// Kind.
    pub kind: EventKind,
    /// The host's display name.
    pub host_display_name: String,
    /// Whether guests may join from a browser.
    pub browser_guests_allowed: bool,
    /// Storage limit for the event.
    pub storage_limit_bytes: u64,
    /// Creation time.
    pub created_at: i64,
}

/// The payload of `event_updated`: only the fields that changed are present.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EventPatch {
    /// New name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// New description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// New links.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub links: Option<Vec<String>>,
    /// New start time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starts_at: Option<i64>,
    /// New end time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ends_at: Option<i64>,
    /// New expiry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    /// New retention.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retention_hours: Option<u32>,
    /// New latest end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_ends_at: Option<i64>,
    /// New extension count.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extension_count: Option<u32>,
    /// New status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<EventStatus>,
    /// New kind (a quick session converted to an event).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<EventKind>,
    /// New host display name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_display_name: Option<String>,
    /// New browser-guest setting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser_guests_allowed: Option<bool>,
    /// New storage limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage_limit_bytes: Option<u64>,
}

/// An attendee: the `attendee_joined` payload and the snapshot's `attendees` entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Attendee {
    /// Attendee ID.
    pub id: AttendeeId,
    /// Display name.
    pub display_name: String,
    /// Role.
    pub role: Role,
    /// Joined with a valid account credential.
    pub verified: bool,
    /// Client kind.
    pub client_kind: ClientKind,
    /// The host's own device.
    pub is_host_device: bool,
}

/// An available item: the `item_available` payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Item {
    /// Item ID.
    pub id: ItemId,
    /// Uploader's attendee ID.
    pub uploader_id: AttendeeId,
    /// MIME type of the original.
    pub content_type: String,
    /// Size of the original.
    pub size_bytes: u64,
    /// SHA-256 of the original, lowercase hex.
    pub sha256: Sha256Hex,
    /// Width in pixels, orientation applied.
    pub width: u32,
    /// Height in pixels, orientation applied.
    pub height: u32,
    /// The original carries an HDR gain map.
    pub has_hdr_gain_map: bool,
    /// The compatible copy is the original (JPEG and PNG).
    pub compatible_same_as_original: bool,
    /// The uploader's file name.
    pub original_name: String,
    /// When it became available (the photo time, DU-21).
    pub available_at: i64,
}

/// Download counts and hot score of one item: an entry of `counts_updated`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ItemCounts {
    /// Item ID.
    pub id: ItemId,
    /// Number of downloads (explicit saves, DU-19).
    pub download_count: u64,
    /// Hot score at `hot_score_updated_at`; decays continuously (DU-18).
    pub hot_score: f64,
    /// When `hot_score` was computed.
    pub hot_score_updated_at: i64,
}

/// An item in the snapshot: the `item_available` fields plus its current counts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SnapshotItem {
    /// The item.
    #[serde(flatten)]
    pub item: Item,
    /// Number of downloads.
    pub download_count: u64,
    /// Hot score at `hot_score_updated_at`.
    pub hot_score: f64,
    /// When `hot_score` was computed.
    pub hot_score_updated_at: i64,
}

/// A post: the `post_published` payload, the snapshot's `posts` entries and the
/// `POST /posts` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Post {
    /// Post ID.
    pub id: PostId,
    /// Kind.
    pub kind: PostKind,
    /// Author's attendee ID; `null` for system posts.
    pub author_id: Option<AttendeeId>,
    /// Text, 1–500 characters.
    pub text: String,
    /// Creation time.
    pub created_at: i64,
}
