//! Test support: a probe with settable answers, reusable by desktop and
//! harness tests (enable the `test-util` feature).

use std::net::Ipv4Addr;
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use super::probe::{AddressClass, FirewallState, NetworkState, OsProbe, PowerState, SelfConnect};

#[derive(Debug, Clone)]
pub struct FakeAnswers {
    pub network: NetworkState,
    pub firewall: FirewallState,
    pub free_bytes: u64,
    pub power: PowerState,
    pub address_class: AddressClass,
    pub mdns_reply: bool,
    /// When true the mDNS probe never answers (to exercise the timeout).
    pub mdns_hangs: bool,
    pub self_connect: SelfConnect,
}

/// Fake probe. Starts with everything passing.
#[derive(Debug)]
pub struct FakeProbe {
    answers: Mutex<FakeAnswers>,
}

impl Default for FakeProbe {
    fn default() -> Self {
        Self {
            answers: Mutex::new(FakeAnswers {
                network: NetworkState::Connected(Ipv4Addr::new(192, 168, 1, 20)),
                firewall: FirewallState::Allowed,
                free_bytes: u64::MAX,
                power: PowerState::PluggedIn,
                address_class: AddressClass::Private,
                mdns_reply: true,
                mdns_hangs: false,
                self_connect: SelfConnect::Connected,
            }),
        }
    }
}

impl FakeProbe {
    pub fn new() -> Self {
        Self::default()
    }

    /// Change any answers through a closure.
    pub fn set(&self, f: impl FnOnce(&mut FakeAnswers)) -> &Self {
        f(&mut self.answers.lock().unwrap());
        self
    }

    fn get(&self) -> FakeAnswers {
        self.answers.lock().unwrap().clone()
    }
}

impl OsProbe for FakeProbe {
    fn network(&self) -> NetworkState {
        self.get().network
    }
    fn firewall(&self, _port: u16) -> FirewallState {
        self.get().firewall
    }
    fn free_bytes(&self, _path: &Path) -> u64 {
        self.get().free_bytes
    }
    fn power(&self) -> PowerState {
        self.get().power
    }
    fn address_class(&self) -> AddressClass {
        self.get().address_class
    }
    async fn mdns_peer_reply(&self, _wait: Duration) -> bool {
        let a = self.get();
        if a.mdns_hangs {
            std::future::pending::<()>().await;
        }
        a.mdns_reply
    }
    async fn connect_self(&self) -> SelfConnect {
        self.get().self_connect
    }
}
