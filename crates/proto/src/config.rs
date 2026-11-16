//! Tunable configuration with compiled defaults and bounds, and the fixed
//! constants (digest §10, §11, DU-43).
//!
//! The host builds a [`Config`] at event start (fresh remote values, else cached,
//! else compiled; see [`Config::apply_remote`]), freezes it into the event and
//! sends it in `config` at join and in the snapshot. Values outside the compiled
//! bounds are ignored.
//!
//! Sizes use binary units (DU-43): 1 KB = 1,024 bytes, 1 MB = 1,048,576 bytes.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const KIB: u64 = 1024;
const MIB: u64 = 1024 * KIB;
const GIB: u64 = 1024 * MIB;
const SECOND_MS: u64 = 1000;
const MINUTE_MS: u64 = 60 * SECOND_MS;
const HOUR_MS: u64 = 60 * MINUTE_MS;
const DAY_MS: u64 = 24 * HOUR_MS;

/// A tunable numeric value: its key in the remote config, compiled default and
/// inclusive bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bound {
    /// Dotted key, `<group>.<field>`, as in the remote config and [`Config`] JSON.
    pub key: &'static str,
    /// Compiled default.
    pub default: u64,
    /// Smallest accepted value.
    pub min: u64,
    /// Largest accepted value.
    pub max: u64,
    /// Unit, for the contract document.
    pub unit: &'static str,
}

impl Bound {
    /// Whether `value` is within the bounds.
    pub const fn accepts(&self, value: u64) -> bool {
        value >= self.min && value <= self.max
    }

    fn pick(&self, value: u64) -> u64 {
        if self.accepts(value) {
            value
        } else {
            self.default
        }
    }
}

/// Upload chunk size: 1 MiB, bounds 256 KiB–4 MiB (§10, DU-43).
pub const CHUNK_BYTES: Bound = Bound {
    key: "transfer_config.chunk_bytes",
    default: MIB,
    min: 256 * KIB,
    max: 4 * MIB,
    unit: "bytes",
};
/// Stall timeout: no chunk for this long fails the upload as `stalled`.
pub const STALL_TIMEOUT_MS: Bound = Bound {
    key: "transfer_config.stall_timeout_ms",
    default: 120 * SECOND_MS,
    min: 30 * SECOND_MS,
    max: 10 * MINUTE_MS,
    unit: "ms",
};
/// Transfer retries per chunk request.
pub const TRANSFER_RETRIES: Bound = Bound {
    key: "transfer_config.transfer_retries",
    default: 3,
    min: 1,
    max: 10,
    unit: "count",
};
/// Base of the exponential backoff with jitter (C3).
pub const BACKOFF_BASE_MS: Bound = Bound {
    key: "transfer_config.backoff_base_ms",
    default: 500,
    min: 100,
    max: 10 * SECOND_MS,
    unit: "ms",
};
/// Outbox attempts before an entry expires.
pub const OUTBOX_MAX_ATTEMPTS: Bound = Bound {
    key: "outbox_config.max_attempts",
    default: 10,
    min: 1,
    max: 50,
    unit: "count",
};
/// Outbox age before an entry expires.
pub const OUTBOX_MAX_AGE_MS: Bound = Bound {
    key: "outbox_config.max_age_ms",
    default: DAY_MS,
    min: HOUR_MS,
    max: 7 * DAY_MS,
    unit: "ms",
};
/// Uploads allowed per user per window.
pub const UPLOADS_PER_WINDOW: Bound = Bound {
    key: "rate_limits.uploads_per_window",
    default: 30,
    min: 1,
    max: 1000,
    unit: "count",
};
/// Upload rate-limit window.
pub const UPLOAD_WINDOW_MS: Bound = Bound {
    key: "rate_limits.upload_window_ms",
    default: 5 * MINUTE_MS,
    min: MINUTE_MS,
    max: HOUR_MS,
    unit: "ms",
};

/// Every tunable numeric value, in contract order.
pub const TUNABLES: &[Bound] = &[
    CHUNK_BYTES,
    STALL_TIMEOUT_MS,
    TRANSFER_RETRIES,
    BACKOFF_BASE_MS,
    OUTBOX_MAX_ATTEMPTS,
    OUTBOX_MAX_AGE_MS,
    UPLOADS_PER_WINDOW,
    UPLOAD_WINDOW_MS,
];

open_enum! {
    /// Remote flag `reachability_test_mode` (PH §7). Open: an unknown value falls
    /// back to the compiled default.
    pub enum ReachabilityTestMode {
        /// Full reachability test.
        Full = "full",
    }
}

open_enum! {
    /// Remote flag `compatible_copy_strategy` (PH §7). Open: an unknown value falls
    /// back to the compiled default.
    pub enum CompatibleCopyStrategy {
        /// Make the compatible copy on first request.
        OnDemand = "on_demand",
    }
}

/// Transfer settings (remote key `transfer_config`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct TransferConfig {
    /// Upload chunk size in bytes.
    pub chunk_bytes: u64,
    /// Stall timeout in milliseconds.
    pub stall_timeout_ms: u64,
    /// Transfer retries.
    pub transfer_retries: u32,
    /// Backoff base in milliseconds.
    pub backoff_base_ms: u64,
}

/// Outbox settings (remote key `outbox_config`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct OutboxConfig {
    /// Attempts before an entry expires.
    pub max_attempts: u32,
    /// Age in milliseconds before an entry expires.
    pub max_age_ms: u64,
}

/// Rate limits (remote key `rate_limits`). Join and password limits are fixed;
/// see [`crate::api::JOINS_PER_MINUTE_PER_IP`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct RateLimits {
    /// Uploads per user per window.
    pub uploads_per_window: u32,
    /// Window length in milliseconds.
    pub upload_window_ms: u64,
}

/// The configuration the host freezes at event start and sends at join and in the
/// snapshot. Keys match the PH §7 flag and remote-config names.
///
/// Missing keys take the compiled default, so an older host's config still parses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Config {
    /// Flag: browser guests can be offered.
    pub browser_guests_available: bool,
    /// Flag: quick sessions are enabled.
    pub quick_session_enabled: bool,
    /// Remote config: reachability test mode.
    pub reachability_test_mode: ReachabilityTestMode,
    /// Remote config: when the compatible copy is made.
    pub compatible_copy_strategy: CompatibleCopyStrategy,
    /// Flag: the desktop app offers Download all.
    pub desktop_download_all: bool,
    /// Transfer settings.
    pub transfer_config: TransferConfig,
    /// Outbox settings.
    pub outbox_config: OutboxConfig,
    /// Rate limits.
    pub rate_limits: RateLimits,
}

impl Default for TransferConfig {
    fn default() -> Self {
        TransferConfig {
            chunk_bytes: CHUNK_BYTES.default,
            stall_timeout_ms: STALL_TIMEOUT_MS.default,
            transfer_retries: TRANSFER_RETRIES.default as u32,
            backoff_base_ms: BACKOFF_BASE_MS.default,
        }
    }
}

impl Default for OutboxConfig {
    fn default() -> Self {
        OutboxConfig {
            max_attempts: OUTBOX_MAX_ATTEMPTS.default as u32,
            max_age_ms: OUTBOX_MAX_AGE_MS.default,
        }
    }
}

impl Default for RateLimits {
    fn default() -> Self {
        RateLimits {
            uploads_per_window: UPLOADS_PER_WINDOW.default as u32,
            upload_window_ms: UPLOAD_WINDOW_MS.default,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            browser_guests_available: true,
            quick_session_enabled: true,
            reachability_test_mode: ReachabilityTestMode::Full,
            compatible_copy_strategy: CompatibleCopyStrategy::OnDemand,
            desktop_download_all: true,
            transfer_config: TransferConfig::default(),
            outbox_config: OutboxConfig::default(),
            rate_limits: RateLimits::default(),
        }
    }
}

fn pick_u32(bound: &Bound, value: u32) -> u32 {
    // Every u32 bound's default fits in u32.
    bound.pick(u64::from(value)) as u32
}

impl Config {
    /// Replaces every out-of-range numeric value and every unknown enum value with
    /// its compiled default. In-range values are kept.
    pub fn sanitize(self) -> Self {
        let t = self.transfer_config;
        let o = self.outbox_config;
        let r = self.rate_limits;
        let defaults = Config::default();
        Config {
            reachability_test_mode: match self.reachability_test_mode {
                ReachabilityTestMode::Unknown => defaults.reachability_test_mode,
                known => known,
            },
            compatible_copy_strategy: match self.compatible_copy_strategy {
                CompatibleCopyStrategy::Unknown => defaults.compatible_copy_strategy,
                known => known,
            },
            transfer_config: TransferConfig {
                chunk_bytes: CHUNK_BYTES.pick(t.chunk_bytes),
                stall_timeout_ms: STALL_TIMEOUT_MS.pick(t.stall_timeout_ms),
                transfer_retries: pick_u32(&TRANSFER_RETRIES, t.transfer_retries),
                backoff_base_ms: BACKOFF_BASE_MS.pick(t.backoff_base_ms),
            },
            outbox_config: OutboxConfig {
                max_attempts: pick_u32(&OUTBOX_MAX_ATTEMPTS, o.max_attempts),
                max_age_ms: OUTBOX_MAX_AGE_MS.pick(o.max_age_ms),
            },
            rate_limits: RateLimits {
                uploads_per_window: pick_u32(&UPLOADS_PER_WINDOW, r.uploads_per_window),
                upload_window_ms: UPLOAD_WINDOW_MS.pick(r.upload_window_ms),
            },
            ..self
        }
    }

    /// Applies remote or cached values on top of `self`. Each key is applied only
    /// when it is present, has the right JSON type, is within bounds and (for
    /// enums) is known; anything else is ignored and the current value kept.
    /// Unknown keys are ignored.
    pub fn apply_remote(&mut self, remote: &Value) {
        let Some(obj) = remote.as_object() else {
            return;
        };
        let flag = |key: &str, slot: &mut bool| {
            if let Some(v) = obj.get(key).and_then(Value::as_bool) {
                *slot = v;
            }
        };
        flag(
            "browser_guests_available",
            &mut self.browser_guests_available,
        );
        flag("quick_session_enabled", &mut self.quick_session_enabled);
        flag("desktop_download_all", &mut self.desktop_download_all);

        if let Some(mode) = obj.get("reachability_test_mode").and_then(Value::as_str) {
            match mode.parse() {
                Ok(ReachabilityTestMode::Unknown) | Err(_) => {}
                Ok(known) => self.reachability_test_mode = known,
            }
        }
        if let Some(strategy) = obj.get("compatible_copy_strategy").and_then(Value::as_str) {
            match strategy.parse() {
                Ok(CompatibleCopyStrategy::Unknown) | Err(_) => {}
                Ok(known) => self.compatible_copy_strategy = known,
            }
        }

        let num = |bound: &Bound| -> Option<u64> {
            let (group, field) = bound.key.split_once('.')?;
            let v = obj.get(group)?.get(field)?.as_u64()?;
            bound.accepts(v).then_some(v)
        };
        let t = &mut self.transfer_config;
        if let Some(v) = num(&CHUNK_BYTES) {
            t.chunk_bytes = v;
        }
        if let Some(v) = num(&STALL_TIMEOUT_MS) {
            t.stall_timeout_ms = v;
        }
        if let Some(v) = num(&TRANSFER_RETRIES) {
            t.transfer_retries = v as u32;
        }
        if let Some(v) = num(&BACKOFF_BASE_MS) {
            t.backoff_base_ms = v;
        }
        let o = &mut self.outbox_config;
        if let Some(v) = num(&OUTBOX_MAX_ATTEMPTS) {
            o.max_attempts = v as u32;
        }
        if let Some(v) = num(&OUTBOX_MAX_AGE_MS) {
            o.max_age_ms = v;
        }
        let r = &mut self.rate_limits;
        if let Some(v) = num(&UPLOADS_PER_WINDOW) {
            r.uploads_per_window = v as u32;
        }
        if let Some(v) = num(&UPLOAD_WINDOW_MS) {
            r.upload_window_ms = v;
        }
    }

    /// The compiled defaults with `remote` applied (see [`Config::apply_remote`]).
    pub fn from_remote(remote: &Value) -> Self {
        let mut config = Config::default();
        config.apply_remote(remote);
        config
    }
}

/// Maximum photo size: 50 MiB = 52,428,800 bytes (§10, DU-43).
pub const MAX_PHOTO_BYTES: u64 = 50 * MIB;
/// Default host storage per event: 20 GiB.
pub const DEFAULT_STORAGE_LIMIT_BYTES: u64 = 20 * GIB;
/// Host storage for a quick session: 5 GiB.
pub const QUICK_SESSION_STORAGE_LIMIT_BYTES: u64 = 5 * GIB;
/// Longest event life, including retention: 7 days.
pub const MAX_EVENT_LIFETIME_MS: u64 = 7 * DAY_MS;
/// A pending item with no progress for this long fails as `abandoned`.
pub const PENDING_TIMEOUT_MS: u64 = 10 * MINUTE_MS;
/// Failed items are cleaned up after this long.
pub const FAILED_CLEANUP_MS: u64 = HOUR_MS;
/// Sweeper interval.
pub const SWEEPER_INTERVAL_MS: u64 = 60 * SECOND_MS;
/// App session lifetime.
pub const SESSION_TTL_MS: u64 = 12 * HOUR_MS;
/// Browser guest session lifetime (DU-10).
pub const BROWSER_SESSION_TTL_MS: u64 = 2 * HOUR_MS;
/// Credential lifetime.
pub const CREDENTIAL_LIFETIME_MS: u64 = 7 * DAY_MS;
/// Refresh the credential when this much life is left.
pub const CREDENTIAL_REFRESH_REMAINING_MS: u64 = 2 * DAY_MS;
/// Download counts are batched into `counts_updated` at this interval.
pub const COUNT_BATCH_INTERVAL_MS: u64 = 30 * SECOND_MS;
/// Hot-score half-life (continuous decay, DU-18).
pub const HOT_SCORE_HALF_LIFE_MS: u64 = 15 * MINUTE_MS;
/// Longest edge of a preview, in pixels.
pub const PREVIEW_MAX_EDGE_PX: u32 = 1024;
/// JPEG quality of previews (DU-14).
pub const PREVIEW_JPEG_QUALITY: u8 = 80;
/// JPEG quality of compatible copies.
pub const COMPATIBLE_JPEG_QUALITY: u8 = 92;
/// Integrity mismatches allowed before the item fails as `integrity`
/// (the first two re-send from offset 0, DU-13).
pub const MAX_INTEGRITY_ATTEMPTS: u32 = 3;
/// Characters allowed in a post: at least 1, at most 500.
pub const POST_TEXT_MAX_CHARS: usize = 500;
/// Digits of an email sign-in code.
pub const LOGIN_CODE_DIGITS: u32 = 6;
/// Lifetime of an email sign-in code.
pub const LOGIN_CODE_TTL_MS: u64 = 10 * MINUTE_MS;

/// A fixed constant, for the contract document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fixed {
    /// Constant name in this module.
    pub name: &'static str,
    /// Value.
    pub value: u64,
    /// Unit.
    pub unit: &'static str,
}

/// Every fixed constant of this module, in contract order.
pub const FIXED: &[Fixed] = &[
    Fixed {
        name: "MAX_PHOTO_BYTES",
        value: MAX_PHOTO_BYTES,
        unit: "bytes",
    },
    Fixed {
        name: "DEFAULT_STORAGE_LIMIT_BYTES",
        value: DEFAULT_STORAGE_LIMIT_BYTES,
        unit: "bytes",
    },
    Fixed {
        name: "QUICK_SESSION_STORAGE_LIMIT_BYTES",
        value: QUICK_SESSION_STORAGE_LIMIT_BYTES,
        unit: "bytes",
    },
    Fixed {
        name: "MAX_EVENT_LIFETIME_MS",
        value: MAX_EVENT_LIFETIME_MS,
        unit: "ms",
    },
    Fixed {
        name: "PENDING_TIMEOUT_MS",
        value: PENDING_TIMEOUT_MS,
        unit: "ms",
    },
    Fixed {
        name: "FAILED_CLEANUP_MS",
        value: FAILED_CLEANUP_MS,
        unit: "ms",
    },
    Fixed {
        name: "SWEEPER_INTERVAL_MS",
        value: SWEEPER_INTERVAL_MS,
        unit: "ms",
    },
    Fixed {
        name: "SESSION_TTL_MS",
        value: SESSION_TTL_MS,
        unit: "ms",
    },
    Fixed {
        name: "BROWSER_SESSION_TTL_MS",
        value: BROWSER_SESSION_TTL_MS,
        unit: "ms",
    },
    Fixed {
        name: "CREDENTIAL_LIFETIME_MS",
        value: CREDENTIAL_LIFETIME_MS,
        unit: "ms",
    },
    Fixed {
        name: "CREDENTIAL_REFRESH_REMAINING_MS",
        value: CREDENTIAL_REFRESH_REMAINING_MS,
        unit: "ms",
    },
    Fixed {
        name: "COUNT_BATCH_INTERVAL_MS",
        value: COUNT_BATCH_INTERVAL_MS,
        unit: "ms",
    },
    Fixed {
        name: "HOT_SCORE_HALF_LIFE_MS",
        value: HOT_SCORE_HALF_LIFE_MS,
        unit: "ms",
    },
    Fixed {
        name: "PREVIEW_MAX_EDGE_PX",
        value: PREVIEW_MAX_EDGE_PX as u64,
        unit: "px",
    },
    Fixed {
        name: "PREVIEW_JPEG_QUALITY",
        value: PREVIEW_JPEG_QUALITY as u64,
        unit: "quality",
    },
    Fixed {
        name: "COMPATIBLE_JPEG_QUALITY",
        value: COMPATIBLE_JPEG_QUALITY as u64,
        unit: "quality",
    },
    Fixed {
        name: "MAX_INTEGRITY_ATTEMPTS",
        value: MAX_INTEGRITY_ATTEMPTS as u64,
        unit: "count",
    },
    Fixed {
        name: "POST_TEXT_MAX_CHARS",
        value: POST_TEXT_MAX_CHARS as u64,
        unit: "chars",
    },
    Fixed {
        name: "LOGIN_CODE_DIGITS",
        value: LOGIN_CODE_DIGITS as u64,
        unit: "digits",
    },
    Fixed {
        name: "LOGIN_CODE_TTL_MS",
        value: LOGIN_CODE_TTL_MS,
        unit: "ms",
    },
];

/// Whether a post text has 1 to [`POST_TEXT_MAX_CHARS`] characters (Unicode scalar values).
pub fn post_text_valid(text: &str) -> bool {
    let n = text.chars().count();
    (1..=POST_TEXT_MAX_CHARS).contains(&n)
}
