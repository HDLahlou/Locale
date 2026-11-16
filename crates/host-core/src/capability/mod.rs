//! Host capability check and reachability test (spec 4.2 step 4, 4.10 step 2, 8.6).
//!
//! Everything that touches the operating system goes through [`OsProbe`]. The
//! decision logic here is pure; nothing opens a firewall, changes a network or
//! toggles a hotspot.

mod check;
mod probe;
mod reachability;
mod real;
#[cfg(any(test, feature = "test-util"))]
pub mod test_support;

pub use check::{
    run_capability_check, CapabilityReport, CapabilityRequest, CheckKind, CheckRow, CheckStatus,
    FixId, Platform, Reason, BATTERY_MIN_PERCENT,
};
pub use probe::{AddressClass, FirewallState, NetworkState, OsProbe, PowerState, SelfConnect};
pub use reachability::{
    run_quick_check, run_reachability_test, QuickCheckResult, Reachability, MDNS_WAIT,
    QUICK_ALLOCATION_BYTES, TOTAL_TIMEOUT,
};
pub use real::{classify_ipv4, RealProbe};
