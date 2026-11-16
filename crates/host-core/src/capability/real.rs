use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, UdpSocket};
use std::path::Path;
use std::time::Duration;

use super::probe::{AddressClass, FirewallState, NetworkState, OsProbe, PowerState, SelfConnect};

/// Classify an IPv4 address: private ranges are `Private`, except link-local
/// (169.254/16) and carrier-grade NAT (100.64/10), which are treated as
/// captive or isolated. Everything else is `Public`.
pub fn classify_ipv4(ip: Ipv4Addr) -> AddressClass {
    let o = ip.octets();
    if ip.is_link_local() || (o[0] == 100 && (64..128).contains(&o[1])) {
        AddressClass::CaptiveOrIsolated
    } else if ip.is_private() {
        AddressClass::Private
    } else {
        AddressClass::Public
    }
}

/// Real probe for what std and existing dependencies answer portably.
/// Firewall, hotspot, power and mDNS are left to M3-11: firewall is `Unknown`,
/// power is `Desktop`, mDNS reports no reply.
#[derive(Debug, Default, Clone, Copy)]
pub struct RealProbe;

impl RealProbe {
    /// Non-loopback IPv4 address of the default route, found by "connecting" a
    /// UDP socket (no packet is sent).
    fn lan_ipv4() -> Option<Ipv4Addr> {
        let s = UdpSocket::bind("0.0.0.0:0").ok()?;
        s.connect("192.0.2.1:9").ok()?;
        match s.local_addr().ok()?.ip() {
            IpAddr::V4(ip) if !ip.is_loopback() && !ip.is_unspecified() => Some(ip),
            _ => None,
        }
    }

    /// True when the port can be bound on all interfaces.
    pub fn port_is_free(port: u16) -> bool {
        TcpListener::bind(("0.0.0.0", port)).is_ok()
    }
}

impl OsProbe for RealProbe {
    fn network(&self) -> NetworkState {
        match Self::lan_ipv4() {
            Some(ip) if ip.is_private() => NetworkState::Connected(ip),
            _ => NetworkState::None,
        }
    }

    fn firewall(&self, _port: u16) -> FirewallState {
        FirewallState::Unknown
    }

    fn free_bytes(&self, path: &Path) -> u64 {
        fs4::available_space(path).unwrap_or(0)
    }

    fn power(&self) -> PowerState {
        PowerState::Desktop
    }

    fn address_class(&self) -> AddressClass {
        Self::lan_ipv4().map_or(AddressClass::NoAddress, classify_ipv4)
    }

    async fn mdns_peer_reply(&self, _wait: Duration) -> bool {
        false
    }

    async fn connect_self(&self) -> SelfConnect {
        let Some(ip) = Self::lan_ipv4() else {
            return SelfConnect::Failed;
        };
        let Ok(listener) = tokio::net::TcpListener::bind(SocketAddr::new(ip.into(), 0)).await
        else {
            return SelfConnect::NotPermitted;
        };
        let Ok(addr) = listener.local_addr() else {
            return SelfConnect::NotPermitted;
        };
        match tokio::net::TcpStream::connect(addr).await {
            Ok(_) => SelfConnect::Connected,
            Err(_) => SelfConnect::Failed,
        }
    }
}
