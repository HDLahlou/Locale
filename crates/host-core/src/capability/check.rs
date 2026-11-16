use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::probe::{FirewallState, NetworkState, OsProbe, PowerState};

/// Battery must be strictly above this percentage (exactly 50 fails).
pub const BATTERY_MIN_PERCENT: u8 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Windows,
    Macos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckKind {
    Network,
    Firewall,
    Space,
    Power,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Ok,
    Failed,
    /// Reported but does not block.
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    NoNetwork,
    FirewallBlocked,
    FirewallUnknown,
    LowSpace,
    LowPower,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixId {
    TurnOnHotspot,
    ShowNetworkInstructions,
    AllowFirewall,
    ChooseSmallerSize,
    PlugIn,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckRow {
    pub check: CheckKind,
    pub status: CheckStatus,
    pub reason: Option<Reason>,
    pub fix_id: Option<FixId>,
    /// Set on the space row when it fails.
    pub free_bytes: Option<u64>,
    /// Set on the space row when it fails.
    pub required_bytes: Option<u64>,
}

impl CheckRow {
    fn new(check: CheckKind, status: CheckStatus) -> Self {
        Self {
            check,
            status,
            reason: None,
            fix_id: None,
            free_bytes: None,
            required_bytes: None,
        }
    }
    fn ok(check: CheckKind) -> Self {
        Self::new(check, CheckStatus::Ok)
    }
    fn bad(check: CheckKind, status: CheckStatus, reason: Reason, fix: FixId) -> Self {
        Self {
            reason: Some(reason),
            fix_id: Some(fix),
            ..Self::new(check, status)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityRequest {
    pub allocation_bytes: u64,
    pub folder: PathBuf,
    pub port: u16,
    /// Decides the network fix; passed in so both are testable anywhere.
    pub platform: Platform,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityReport {
    /// Network, firewall, space, power, in that order.
    pub rows: Vec<CheckRow>,
    pub can_start: bool,
}

/// Run the four checks of spec 4.2 step 4.
pub fn run_capability_check<P: OsProbe>(probe: &P, req: &CapabilityRequest) -> CapabilityReport {
    let network = match probe.network() {
        NetworkState::Connected(_) | NetworkState::HotspotOn => CheckRow::ok(CheckKind::Network),
        NetworkState::None => {
            let fix = match req.platform {
                Platform::Windows => FixId::TurnOnHotspot,
                Platform::Macos => FixId::ShowNetworkInstructions,
            };
            CheckRow::bad(
                CheckKind::Network,
                CheckStatus::Failed,
                Reason::NoNetwork,
                fix,
            )
        }
    };
    let firewall = match probe.firewall(req.port) {
        FirewallState::Allowed => CheckRow::ok(CheckKind::Firewall),
        FirewallState::Blocked => CheckRow::bad(
            CheckKind::Firewall,
            CheckStatus::Failed,
            Reason::FirewallBlocked,
            FixId::AllowFirewall,
        ),
        FirewallState::Unknown => CheckRow::bad(
            CheckKind::Firewall,
            CheckStatus::Warning,
            Reason::FirewallUnknown,
            FixId::AllowFirewall,
        ),
    };
    let free = probe.free_bytes(&req.folder);
    let space = if free >= req.allocation_bytes {
        CheckRow::ok(CheckKind::Space)
    } else {
        CheckRow {
            free_bytes: Some(free),
            required_bytes: Some(req.allocation_bytes),
            ..CheckRow::bad(
                CheckKind::Space,
                CheckStatus::Failed,
                Reason::LowSpace,
                FixId::ChooseSmallerSize,
            )
        }
    };
    let power = match probe.power() {
        PowerState::Battery { percent } if percent <= BATTERY_MIN_PERCENT => CheckRow::bad(
            CheckKind::Power,
            CheckStatus::Failed,
            Reason::LowPower,
            FixId::PlugIn,
        ),
        _ => CheckRow::ok(CheckKind::Power),
    };
    let rows = vec![network, firewall, space, power];
    let can_start = rows.iter().all(|r| r.status != CheckStatus::Failed);
    CapabilityReport { rows, can_start }
}
