//! The item state machine as data (digest §7, MVP §6, DU-13).
//!
//! [`TRANSITIONS`] holds exactly the digest's rows. Any other transition is
//! illegal and must be rejected. `DELETED` is a tombstone in the change log; the
//! item row is removed.

use crate::codes::ItemReason;
use crate::model::ItemState;

closed_enum! {
    /// Who makes a transition.
    pub enum Actor {
        /// The host.
        Host = "host",
        /// The sweeper (timeouts).
        Sweeper = "sweeper",
        /// The host, at the uploader's request.
        HostOnRequest = "host_on_request",
    }
}

/// One row of the state table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transition {
    /// From state; `None` for a new item.
    pub from: Option<ItemState>,
    /// To state.
    pub to: ItemState,
    /// What triggers it.
    pub trigger: &'static str,
    /// Who makes it.
    pub actor: Actor,
    /// Reason codes set on this row (empty when none).
    pub reasons: &'static [ItemReason],
    /// The table's Retryable column: `Some(true)` Yes, `Some(false)` No, `None` blank.
    pub retryable: Option<bool>,
}

use crate::model::ItemState::{Available, Deleted, Failed, Pending, Uploading, Validating};

/// The state table, in digest order (14 rows).
pub const TRANSITIONS: &[Transition] = &[
    Transition {
        from: None,
        to: Pending,
        trigger: "Submit passes User Validation",
        actor: Actor::Host,
        reasons: &[],
        retryable: None,
    },
    Transition {
        from: Some(Pending),
        to: Uploading,
        trigger: "Basic Content Precheck passes, storage available",
        actor: Actor::Host,
        reasons: &[],
        retryable: None,
    },
    Transition {
        from: Some(Pending),
        to: Failed,
        trigger: "Precheck denied",
        actor: Actor::Host,
        reasons: &[ItemReason::Type, ItemReason::Size, ItemReason::StorageFull],
        retryable: Some(false),
    },
    Transition {
        from: Some(Pending),
        to: Failed,
        trigger: "No progress for 10 min",
        actor: Actor::Sweeper,
        reasons: &[ItemReason::Abandoned],
        retryable: Some(true),
    },
    Transition {
        from: Some(Uploading),
        to: Validating,
        trigger: "Last chunk received",
        actor: Actor::Host,
        reasons: &[],
        retryable: None,
    },
    Transition {
        from: Some(Uploading),
        to: Failed,
        trigger: "No chunk for 2 min",
        actor: Actor::Sweeper,
        reasons: &[ItemReason::Stalled],
        retryable: Some(true),
    },
    Transition {
        from: Some(Uploading),
        to: Failed,
        trigger: "Uploader cancels",
        actor: Actor::HostOnRequest,
        reasons: &[ItemReason::Cancelled],
        retryable: Some(false),
    },
    Transition {
        from: Some(Uploading),
        to: Failed,
        trigger: "Token invalid",
        actor: Actor::Host,
        reasons: &[ItemReason::Unauthorized],
        retryable: Some(false),
    },
    Transition {
        from: Some(Failed),
        to: Uploading,
        trigger: "Same submission ID resubmitted, reason retryable",
        actor: Actor::Host,
        reasons: &[],
        retryable: None,
    },
    Transition {
        from: Some(Validating),
        to: Available,
        trigger: "Integrity and Server Content Validation pass",
        actor: Actor::Host,
        reasons: &[],
        retryable: None,
    },
    Transition {
        from: Some(Validating),
        to: Failed,
        trigger: "Validation denied",
        actor: Actor::Host,
        reasons: &[ItemReason::InvalidImage],
        retryable: Some(false),
    },
    Transition {
        from: Some(Validating),
        to: Failed,
        trigger: "Integrity retries exceeded",
        actor: Actor::Host,
        reasons: &[ItemReason::Integrity],
        retryable: Some(true),
    },
    Transition {
        from: Some(Validating),
        to: Uploading,
        trigger: "Integrity mismatch, fewer than 3 so far: bytes discarded, offset 0 (DU-13)",
        actor: Actor::Host,
        reasons: &[],
        retryable: None,
    },
    Transition {
        from: Some(Available),
        to: Deleted,
        trigger: "Uploader or host deletes, or event expires",
        actor: Actor::Host,
        reasons: &[],
        retryable: None,
    },
];

/// Whether the table has a row from `from` (`None` = new item) to `to`.
pub fn is_legal(from: Option<ItemState>, to: ItemState) -> bool {
    TRANSITIONS.iter().any(|t| t.from == from && t.to == to)
}

/// The row matching `from`, `to` and the reason code, if any. A row with reason
/// codes matches only one of them; a row without matches only `None`.
pub fn find(
    from: Option<ItemState>,
    to: ItemState,
    reason: Option<ItemReason>,
) -> Option<&'static Transition> {
    TRANSITIONS.iter().find(|t| {
        t.from == from
            && t.to == to
            && match reason {
                Some(r) => t.reasons.contains(&r),
                None => t.reasons.is_empty(),
            }
    })
}
