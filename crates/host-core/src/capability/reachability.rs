use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::check::{run_capability_check, CapabilityReport, CapabilityRequest};
use super::probe::{AddressClass, OsProbe, SelfConnect};

/// How long to listen for an mDNS peer reply (spec 8.6).
pub const MDNS_WAIT: Duration = Duration::from_secs(2);
/// Bound on the whole reachability test.
pub const TOTAL_TIMEOUT: Duration = Duration::from_secs(5);
/// Quick-session storage allocation: 5 GB (spec 4.10).
pub const QUICK_ALLOCATION_BYTES: u64 = 5_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reachability {
    Ok,
    Isolated,
    Unknown,
}

async fn decide<P: OsProbe>(probe: &P) -> Reachability {
    if probe.address_class() != AddressClass::Private {
        return Reachability::Isolated;
    }
    let self_connect = probe.connect_self().await;
    if self_connect == SelfConnect::Failed {
        return Reachability::Isolated;
    }
    let peer = tokio::time::timeout(MDNS_WAIT, probe.mdns_peer_reply(MDNS_WAIT))
        .await
        .unwrap_or(false);
    if peer || self_connect == SelfConnect::Connected {
        Reachability::Ok
    } else {
        Reachability::Unknown
    }
}

/// Reachability test for client isolation (spec 8.6, DU-38).
pub async fn run_reachability_test<P: OsProbe>(probe: &P) -> Reachability {
    tokio::time::timeout(TOTAL_TIMEOUT, decide(probe))
        .await
        .unwrap_or(Reachability::Unknown)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuickCheckResult {
    pub capability: CapabilityReport,
    pub reachability: Reachability,
}

/// Fast check for the quick session (spec 4.10 step 2). Ignores
/// `req.allocation_bytes` and uses the 5 GB quick-session default.
pub async fn run_quick_check<P: OsProbe>(probe: &P, req: &CapabilityRequest) -> QuickCheckResult {
    let req = CapabilityRequest {
        allocation_bytes: QUICK_ALLOCATION_BYTES,
        ..req.clone()
    };
    QuickCheckResult {
        capability: run_capability_check(probe, &req),
        reachability: run_reachability_test(probe).await,
    }
}
