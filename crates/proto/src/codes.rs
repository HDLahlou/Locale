//! Error body and reason codes (digest §5.2).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

open_enum! {
    /// Item reason codes (`item.reason_code`, MVP §6, plus `duplicate` from DU-12).
    ///
    /// Open: an older client that receives a newer reason treats it as final
    /// (not retryable).
    pub enum ItemReason {
        /// Content type not accepted by the precheck.
        Type = "type",
        /// Larger than the maximum photo size.
        Size = "size",
        /// The host has no room for it.
        StorageFull = "storage_full",
        /// No progress for 10 minutes while pending.
        Abandoned = "abandoned",
        /// No chunk for the stall timeout while uploading.
        Stalled = "stalled",
        /// The uploader cancelled.
        Cancelled = "cancelled",
        /// The upload token was invalid.
        Unauthorized = "unauthorized",
        /// Server content validation denied it.
        InvalidImage = "invalid_image",
        /// Integrity retries exceeded.
        Integrity = "integrity",
        /// The same user already uploaded these bytes (DU-12); see `duplicate_of`.
        Duplicate = "duplicate",
    }
}

impl ItemReason {
    /// Whether the same submission may be resubmitted. Exactly `abandoned`,
    /// `stalled` and `integrity` are retryable; all others are final (MVP §6).
    pub const fn retryable(self) -> bool {
        matches!(
            self,
            ItemReason::Abandoned | ItemReason::Stalled | ItemReason::Integrity
        )
    }
}

closed_enum! {
    /// Join failure codes (PH §5 `join_failed`, plus DU). The first six come from
    /// the host in an [`ErrorBody`]; the last four are detected by the client.
    ///
    /// Closed, but it only ever travels as the string [`ErrorDetail::code`], so an
    /// unknown code never fails deserialisation of the body.
    pub enum JoinFailure {
        /// The event password was wrong.
        WrongPassword = "wrong_password",
        /// The join token is not this event's token.
        TokenInvalid = "token_invalid",
        /// The person is on the event's removed list.
        Removed = "removed",
        /// The event has ended.
        EventEnded = "event_ended",
        /// The host does not allow browser guests.
        BrowserNotAllowed = "browser_not_allowed",
        /// Too many joins or password attempts from this address.
        RateLimited = "rate_limited",
        /// Client only: the OS denied local network access.
        LocalNetworkDenied = "local_network_denied",
        /// Client only: the host's certificate does not match the pinned fingerprint.
        FingerprintMismatch = "fingerprint_mismatch",
        /// Client only: the host could not be reached.
        Unreachable = "unreachable",
        /// Client only: the stored join token is no longer accepted.
        TokenExpired = "token_expired",
    }
}

impl JoinFailure {
    /// Whether the client detects this failure itself (never sent by the host).
    pub const fn is_client_side(self) -> bool {
        matches!(
            self,
            JoinFailure::LocalNetworkDenied
                | JoinFailure::FingerprintMismatch
                | JoinFailure::Unreachable
                | JoinFailure::TokenExpired
        )
    }

    /// HTTP status the host answers `POST /join` with, or `None` for client-side codes.
    pub const fn http_status(self) -> Option<u16> {
        match self {
            JoinFailure::WrongPassword | JoinFailure::TokenInvalid => Some(401),
            JoinFailure::Removed | JoinFailure::BrowserNotAllowed => Some(403),
            JoinFailure::EventEnded => Some(410),
            JoinFailure::RateLimited => Some(429),
            JoinFailure::LocalNetworkDenied
            | JoinFailure::FingerprintMismatch
            | JoinFailure::Unreachable
            | JoinFailure::TokenExpired => None,
        }
    }

    /// The `retryable` flag sent with this code: the same request may succeed later.
    pub const fn retryable(self) -> bool {
        matches!(self, JoinFailure::RateLimited | JoinFailure::Unreachable)
    }
}

closed_enum! {
    /// General API error codes, each with a fixed HTTP status.
    ///
    /// Closed, but it only ever travels as the string [`ErrorDetail::code`].
    pub enum ApiCode {
        /// 401: the session token is missing, expired or revoked.
        SessionInvalid = "session_invalid",
        /// 403: the attendee's role does not allow this.
        RoleDenied = "role_denied",
        /// 404: no such item, submission, post or event.
        NotFound = "not_found",
        /// 409: the item exists but is not in a state that allows this.
        NotAvailable = "not_available",
        /// 409: the chunk's `Upload-Offset` is not the received byte count.
        OffsetMismatch = "offset_mismatch",
        /// 429: rate limit reached; see `retry_after_ms`.
        RateLimited = "rate_limited",
        /// 400: the request is malformed.
        BadRequest = "bad_request",
        /// 426: the `X-Locale-Proto` version is missing or not supported.
        ProtoUnsupported = "proto_unsupported",
    }
}

impl ApiCode {
    /// The HTTP status for this code.
    pub const fn http_status(self) -> u16 {
        match self {
            ApiCode::SessionInvalid => 401,
            ApiCode::RoleDenied => 403,
            ApiCode::NotFound => 404,
            ApiCode::NotAvailable | ApiCode::OffsetMismatch => 409,
            ApiCode::RateLimited => 429,
            ApiCode::BadRequest => 400,
            ApiCode::ProtoUnsupported => 426,
        }
    }

    /// The `retryable` flag sent with this code: the same request may succeed later
    /// (after `retry_after_ms`, or from `received_bytes` for an offset mismatch).
    pub const fn retryable(self) -> bool {
        matches!(self, ApiCode::RateLimited | ApiCode::OffsetMismatch)
    }
}

/// The error body of every failed host API or service request:
/// `{"error": {"code", "message", "retryable", "retry_after_ms"?}}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ErrorBody {
    /// The error.
    pub error: ErrorDetail,
}

/// The inside of an [`ErrorBody`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ErrorDetail {
    /// A code from [`ApiCode`] or [`JoinFailure`]. Kept as a string so a code this
    /// version does not know still parses.
    pub code: String,
    /// Human-readable text for logs. Not shown to people as is.
    pub message: String,
    /// Whether the same request may succeed later.
    pub retryable: bool,
    /// How long to wait before retrying, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,
    /// For `offset_mismatch`: the bytes the host has, where the upload resumes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub received_bytes: Option<u64>,
}

impl ErrorBody {
    /// An error body for a general API code.
    pub fn api(code: ApiCode, message: impl Into<String>) -> Self {
        Self::with(code.as_str(), message, code.retryable())
    }

    /// An error body for a join failure sent by the host.
    pub fn join(code: JoinFailure, message: impl Into<String>) -> Self {
        Self::with(code.as_str(), message, code.retryable())
    }

    /// The 409 `offset_mismatch` body carrying the host's received byte count.
    pub fn offset_mismatch(received_bytes: u64) -> Self {
        let mut body = Self::api(ApiCode::OffsetMismatch, "upload offset mismatch");
        body.error.received_bytes = Some(received_bytes);
        body
    }

    /// The 429 `rate_limited` body with a retry delay.
    pub fn rate_limited(retry_after_ms: u64) -> Self {
        let mut body = Self::api(ApiCode::RateLimited, "rate limited");
        body.error.retry_after_ms = Some(retry_after_ms);
        body
    }

    fn with(code: &str, message: impl Into<String>, retryable: bool) -> Self {
        ErrorBody {
            error: ErrorDetail {
                code: code.to_owned(),
                message: message.into(),
                retryable,
                retry_after_ms: None,
                received_bytes: None,
            },
        }
    }
}

impl ErrorDetail {
    /// The code as an [`ApiCode`], if it is one.
    pub fn api_code(&self) -> Option<ApiCode> {
        self.code.parse().ok()
    }

    /// The code as a [`JoinFailure`], if it is one.
    pub fn join_failure(&self) -> Option<JoinFailure> {
        self.code.parse().ok()
    }
}
