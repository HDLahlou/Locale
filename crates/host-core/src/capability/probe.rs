use std::net::Ipv4Addr;
use std::path::Path;
use std::time::Duration;

/// State of the host's network connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkState {
    /// Connected with a private address.
    Connected(Ipv4Addr),
    /// The host's own hotspot is on.
    HotspotOn,
    /// No usable network.
    None,
}

/// Whether the OS firewall allows inbound connections on a port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirewallState {
    Allowed,
    Blocked,
    Unknown,
}

/// Power state of the machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerState {
    Desktop,
    PluggedIn,
    Battery { percent: u8 },
}

/// Class of the host's LAN address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressClass {
    /// Private and not on a known captive or isolated range.
    Private,
    /// Not a private address.
    Public,
    /// Private-looking but on a known captive or isolated range.
    CaptiveOrIsolated,
    /// No address at all.
    NoAddress,
}

/// Result of connecting to the host's own LAN address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfConnect {
    Connected,
    Failed,
    /// The OS does not allow the attempt.
    NotPermitted,
}

/// Everything the capability check and reachability test ask of the OS.
pub trait OsProbe {
    fn network(&self) -> NetworkState;
    fn firewall(&self, port: u16) -> FirewallState;
    fn free_bytes(&self, path: &Path) -> u64;
    fn power(&self) -> PowerState;
    fn address_class(&self) -> AddressClass;
    /// Send an mDNS query and report whether any peer replied within `wait`.
    fn mdns_peer_reply(&self, wait: Duration) -> impl std::future::Future<Output = bool> + Send;
    /// Connect to the host's own LAN address.
    fn connect_self(&self) -> impl std::future::Future<Output = SelfConnect> + Send;
}
