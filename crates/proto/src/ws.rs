//! Index sync messages: WebSocket JSON text frames tagged by `t` (digest §5.4).
//!
//! Both message enums are open: a frame with a `t` this version does not know
//! parses to `Unknown` and is ignored. A change with a `type` this version does
//! not know parses to [`ChangeBody::Unknown`], keeping its raw payload, so the
//! client can still advance its version.

use std::borrow::Cow;

use schemars::{json_schema, JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::codes::ItemReason;
use crate::config::Config;
use crate::model::{
    Attendee, Event, EventPatch, Item, ItemCounts, ItemId, ItemState, Post, SnapshotItem,
    SubmissionId,
};

/// The client must send `hello` within this long of the socket opening.
pub const HELLO_TIMEOUT_MS: u64 = 5000;

/// Client → host messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum ClientMessage {
    /// First frame, within 5 s: authenticates the socket and states the client's
    /// last applied version (0 for none).
    Hello {
        /// Session token.
        session_token: String,
        /// Last applied change version.
        version: u64,
    },
    /// Asks for the changes after `version` again (or a snapshot).
    Resync {
        /// Last applied change version.
        version: u64,
    },
    /// Keep-alive; the host answers `pong`.
    Ping,
    /// A message type this version does not know. Receive-only; never sent.
    #[serde(other)]
    #[schemars(skip)]
    Unknown,
}

open_enum! {
    /// Why the host closes the socket.
    pub enum ByeReason {
        /// The attendee was removed.
        Removed = "removed",
        /// The event ended.
        EventEnded = "event_ended",
        /// The session expired.
        SessionExpired = "session_expired",
    }
}

open_enum! {
    /// Kind of a `notice` message.
    pub enum NoticeKind {
        /// The event expires soon; see `expires_at`.
        ExpiringSoon = "expiring_soon",
    }
}

/// Host → client messages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum ServerMessage {
    /// The full index at `version`. `items` are `AVAILABLE` items only.
    Snapshot {
        /// Version of the newest change included.
        version: u64,
        /// The event.
        event: Event,
        /// Attendees not on the removed list.
        attendees: Vec<Attendee>,
        /// Available items with their counts.
        items: Vec<SnapshotItem>,
        /// Posts not deleted.
        posts: Vec<Post>,
        /// The event's frozen configuration.
        config: Config,
    },
    /// Catch-up: the changes after the client's version, oldest first.
    Changes {
        /// Changes in version order.
        changes: Vec<Change>,
    },
    /// One live change.
    Change(Change),
    /// The state of one of the client's own uploads. Sent to the uploader only;
    /// not versioned. This is the completion ack of 4.4 step 18.
    UploadState {
        /// Submission ID.
        submission_id: SubmissionId,
        /// Current state.
        state: ItemState,
        /// Reason, or `null`.
        reason_code: Option<ItemReason>,
        /// Whether the reason is retryable (`false` when there is none).
        retryable: bool,
        /// Bytes received so far.
        received_bytes: u64,
        /// Item ID.
        item_id: ItemId,
    },
    /// An informational notice. Not versioned.
    Notice {
        /// Kind.
        kind: NoticeKind,
        /// For `expiring_soon`: the expiry.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expires_at: Option<i64>,
    },
    /// The host is closing the socket.
    Bye {
        /// Why.
        reason: ByeReason,
    },
    /// Answer to `ping`.
    Pong,
    /// A message type this version does not know. Receive-only; never sent.
    #[serde(other)]
    #[schemars(skip)]
    Unknown,
}

open_enum! {
    /// Change types (MVP §5.6 step 3, plus counts from §5.12). Also the
    /// `change_log.change_type` column.
    pub enum ChangeType {
        /// Changed event fields.
        EventUpdated = "event_updated",
        /// The expiry moved (extend or shorten).
        EventExtended = "event_extended",
        /// An item became available.
        ItemAvailable = "item_available",
        /// An item was deleted (tombstone).
        ItemDeleted = "item_deleted",
        /// An attendee joined.
        AttendeeJoined = "attendee_joined",
        /// An attendee was removed.
        AttendeeRemoved = "attendee_removed",
        /// A post was published.
        PostPublished = "post_published",
        /// A post was deleted.
        PostDeleted = "post_deleted",
        /// Download counts and hot scores changed.
        CountsUpdated = "counts_updated",
        /// The event ended; the socket closes after this.
        EventEnded = "event_ended",
    }
}

/// `event_extended` payload (also used for shortening).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EventExtended {
    /// Previous expiry.
    pub old_expires_at: i64,
    /// New expiry.
    pub new_expires_at: i64,
    /// End time.
    pub ends_at: i64,
    /// Retention in hours.
    pub retention_hours: u32,
}

/// Payload of a deletion or removal: `{id}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Tombstone {
    /// ID of the deleted item, removed attendee or deleted post.
    pub id: String,
}

/// The empty `{}` payload of `event_ended`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Empty {}

/// The typed payload of a change, selected by its `type`.
#[derive(Debug, Clone, PartialEq)]
pub enum ChangeBody {
    /// `event_updated`.
    EventUpdated(EventPatch),
    /// `event_extended`.
    EventExtended(EventExtended),
    /// `item_available`.
    ItemAvailable(Item),
    /// `item_deleted`.
    ItemDeleted(Tombstone),
    /// `attendee_joined`.
    AttendeeJoined(Attendee),
    /// `attendee_removed`.
    AttendeeRemoved(Tombstone),
    /// `post_published`.
    PostPublished(Post),
    /// `post_deleted`.
    PostDeleted(Tombstone),
    /// `counts_updated`.
    CountsUpdated(Vec<ItemCounts>),
    /// `event_ended`.
    EventEnded(Empty),
    /// A change type this version does not know, kept as received.
    Unknown {
        /// The `type` string.
        change_type: String,
        /// The raw payload.
        payload: Value,
    },
}

impl ChangeBody {
    /// The change type.
    pub fn change_type(&self) -> ChangeType {
        match self {
            ChangeBody::EventUpdated(_) => ChangeType::EventUpdated,
            ChangeBody::EventExtended(_) => ChangeType::EventExtended,
            ChangeBody::ItemAvailable(_) => ChangeType::ItemAvailable,
            ChangeBody::ItemDeleted(_) => ChangeType::ItemDeleted,
            ChangeBody::AttendeeJoined(_) => ChangeType::AttendeeJoined,
            ChangeBody::AttendeeRemoved(_) => ChangeType::AttendeeRemoved,
            ChangeBody::PostPublished(_) => ChangeType::PostPublished,
            ChangeBody::PostDeleted(_) => ChangeType::PostDeleted,
            ChangeBody::CountsUpdated(_) => ChangeType::CountsUpdated,
            ChangeBody::EventEnded(_) => ChangeType::EventEnded,
            ChangeBody::Unknown { .. } => ChangeType::Unknown,
        }
    }

    /// The `type` string on the wire.
    pub fn type_str(&self) -> &str {
        match self {
            ChangeBody::Unknown { change_type, .. } => change_type,
            known => known.change_type().as_str(),
        }
    }

    /// The payload as JSON (the `change_log.payload` column).
    pub fn payload_json(&self) -> Value {
        // Serialising these plain structs to a Value cannot fail.
        let v = match self {
            ChangeBody::EventUpdated(p) => serde_json::to_value(p),
            ChangeBody::EventExtended(p) => serde_json::to_value(p),
            ChangeBody::ItemAvailable(p) => serde_json::to_value(p),
            ChangeBody::ItemDeleted(p)
            | ChangeBody::AttendeeRemoved(p)
            | ChangeBody::PostDeleted(p) => serde_json::to_value(p),
            ChangeBody::AttendeeJoined(p) => serde_json::to_value(p),
            ChangeBody::PostPublished(p) => serde_json::to_value(p),
            ChangeBody::CountsUpdated(p) => serde_json::to_value(p),
            ChangeBody::EventEnded(p) => serde_json::to_value(p),
            ChangeBody::Unknown { payload, .. } => Ok(payload.clone()),
        };
        v.unwrap_or(Value::Null)
    }

    /// Builds the typed body from a `type` string and its payload. An unknown
    /// type gives [`ChangeBody::Unknown`]; a known type with a malformed payload
    /// is an error.
    pub fn from_parts(change_type: &str, payload: Value) -> Result<Self, serde_json::Error> {
        use serde_json::from_value as p;
        let ty: ChangeType = change_type.parse().unwrap_or(ChangeType::Unknown);
        Ok(match ty {
            ChangeType::EventUpdated => ChangeBody::EventUpdated(p(payload)?),
            ChangeType::EventExtended => ChangeBody::EventExtended(p(payload)?),
            ChangeType::ItemAvailable => ChangeBody::ItemAvailable(p(payload)?),
            ChangeType::ItemDeleted => ChangeBody::ItemDeleted(p(payload)?),
            ChangeType::AttendeeJoined => ChangeBody::AttendeeJoined(p(payload)?),
            ChangeType::AttendeeRemoved => ChangeBody::AttendeeRemoved(p(payload)?),
            ChangeType::PostPublished => ChangeBody::PostPublished(p(payload)?),
            ChangeType::PostDeleted => ChangeBody::PostDeleted(p(payload)?),
            ChangeType::CountsUpdated => ChangeBody::CountsUpdated(p(payload)?),
            ChangeType::EventEnded => ChangeBody::EventEnded(p(payload)?),
            ChangeType::Unknown => ChangeBody::Unknown {
                change_type: change_type.to_owned(),
                payload,
            },
        })
    }
}

/// The change envelope `{version, type, payload, ts}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RawChange", into = "RawChange")]
pub struct Change {
    /// Change-log version (strictly increasing per event).
    pub version: u64,
    /// Typed payload; its variant gives `type`.
    pub body: ChangeBody,
    /// When the change was made.
    pub ts: i64,
}

/// The envelope as it appears on the wire.
#[derive(Serialize, Deserialize)]
struct RawChange {
    version: u64,
    #[serde(rename = "type")]
    change_type: String,
    payload: Value,
    ts: i64,
}

impl TryFrom<RawChange> for Change {
    type Error = serde_json::Error;
    fn try_from(raw: RawChange) -> Result<Self, Self::Error> {
        Ok(Change {
            version: raw.version,
            body: ChangeBody::from_parts(&raw.change_type, raw.payload)?,
            ts: raw.ts,
        })
    }
}

impl From<Change> for RawChange {
    fn from(c: Change) -> Self {
        RawChange {
            version: c.version,
            change_type: c.body.type_str().to_owned(),
            payload: c.body.payload_json(),
            ts: c.ts,
        }
    }
}

fn change_variant(ty: ChangeType, payload: Schema) -> Value {
    serde_json::json!({
        "type": "object",
        "required": ["version", "type", "payload", "ts"],
        "properties": {
            "version": {"type": "integer", "format": "uint64", "minimum": 0},
            "type": {"type": "string", "const": ty.as_str()},
            "payload": payload,
            "ts": {"type": "integer", "format": "int64"},
        },
    })
}

impl JsonSchema for Change {
    fn schema_name() -> Cow<'static, str> {
        "Change".into()
    }

    fn json_schema(g: &mut SchemaGenerator) -> Schema {
        let variants = vec![
            change_variant(ChangeType::EventUpdated, g.subschema_for::<EventPatch>()),
            change_variant(
                ChangeType::EventExtended,
                g.subschema_for::<EventExtended>(),
            ),
            change_variant(ChangeType::ItemAvailable, g.subschema_for::<Item>()),
            change_variant(ChangeType::ItemDeleted, g.subschema_for::<Tombstone>()),
            change_variant(ChangeType::AttendeeJoined, g.subschema_for::<Attendee>()),
            change_variant(ChangeType::AttendeeRemoved, g.subschema_for::<Tombstone>()),
            change_variant(ChangeType::PostPublished, g.subschema_for::<Post>()),
            change_variant(ChangeType::PostDeleted, g.subschema_for::<Tombstone>()),
            change_variant(
                ChangeType::CountsUpdated,
                g.subschema_for::<Vec<ItemCounts>>(),
            ),
            change_variant(ChangeType::EventEnded, g.subschema_for::<Empty>()),
        ];
        json_schema!({
            "description": "Change envelope {version, type, payload, ts}. Receivers keep changes with an unknown type as Unknown.",
            "oneOf": variants,
        })
    }
}
