//! Browsing for nearby events over mDNS (digest §5.6, DU-32).
//!
//! A plain library with no OS-specific code: on phones the app holds the Local
//! Network permission and the Android multicast lock (the `locale_net`
//! plugin) while a [`Browse`] is alive. Backed by `mdns-sd`, pure Rust.

use std::collections::HashMap;
use std::fmt;
use std::net::IpAddr;
use std::time::Duration;

use mdns_sd::{ServiceDaemon, ServiceEvent};
use proto::mdns::{TxtRecord, SERVICE_TYPE};
use proto::transport::EventId;

/// How long to wait for the daemon to confirm shutdown on drop.
const DAEMON_WAIT: Duration = Duration::from_secs(2);

/// A nearby event: its parsed TXT record and where to reach the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NearbyService {
    /// The parsed TXT record.
    pub txt: TxtRecord,
    /// Resolved addresses, sorted.
    pub addresses: Vec<IpAddr>,
    /// Port of the host listener.
    pub port: u16,
}

/// What a [`Browse`] reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NearbyEvent {
    /// A new event appeared.
    Found(NearbyService),
    /// A known event changed its TXT content, addresses or port.
    Updated(NearbyService),
    /// A known event went away (goodbye or expiry).
    Lost(EventId),
}

/// Why browsing could not start.
#[derive(Debug)]
pub struct BrowseError(String);

impl fmt::Display for BrowseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "mDNS browsing failed: {}", self.0)
    }
}

impl std::error::Error for BrowseError {}

/// A resolved record as the daemon reports it, before parsing.
struct Resolved {
    fullname: String,
    pairs: Vec<(String, String)>,
    addresses: Vec<IpAddr>,
    port: u16,
}

/// Turns raw observations into found / updated / lost events and counts the
/// records it ignores. Pure: no network.
#[derive(Default)]
struct Tracker {
    known: HashMap<String, NearbyService>,
    ignored: usize,
}

impl Tracker {
    fn resolved(&mut self, r: Resolved) -> Option<NearbyEvent> {
        let txt = match TxtRecord::from_pairs(r.pairs) {
            Ok(txt) => txt,
            Err(_) => {
                self.ignored += 1;
                // A record that was valid and turned invalid is gone for the list.
                return self
                    .known
                    .remove(&r.fullname)
                    .map(|old| NearbyEvent::Lost(old.txt.id));
            }
        };
        let mut addresses = r.addresses;
        addresses.sort();
        let svc = NearbyService {
            txt,
            addresses,
            port: r.port,
        };
        match self.known.insert(r.fullname, svc.clone()) {
            None => Some(NearbyEvent::Found(svc)),
            Some(old) if old == svc => None,
            Some(_) => Some(NearbyEvent::Updated(svc)),
        }
    }

    fn removed(&mut self, fullname: &str) -> Option<NearbyEvent> {
        self.known
            .remove(fullname)
            .map(|old| NearbyEvent::Lost(old.txt.id))
    }

    fn observe(&mut self, event: ServiceEvent) -> Option<NearbyEvent> {
        match event {
            ServiceEvent::ServiceResolved(info) => self.resolved(Resolved {
                fullname: info.get_fullname().to_owned(),
                pairs: info
                    .get_properties()
                    .iter()
                    .map(|p| (p.key().to_owned(), p.val_str().to_owned()))
                    .collect(),
                addresses: info
                    .get_addresses()
                    .iter()
                    .map(|a| a.to_ip_addr())
                    .collect(),
                port: info.get_port(),
            }),
            ServiceEvent::ServiceRemoved(_, fullname) => self.removed(&fullname),
            _ => None,
        }
    }
}

/// A running browse. Dropping it stops the search and frees its sockets.
pub struct Browse {
    daemon: Option<ServiceDaemon>,
    rx: mdns_sd::Receiver<ServiceEvent>,
    tracker: Tracker,
}

/// Starts browsing for `_locale._tcp.local.` services.
pub fn browse() -> Result<Browse, BrowseError> {
    let err = |e: mdns_sd::Error| BrowseError(e.to_string());
    let daemon = ServiceDaemon::new().map_err(err)?;
    let rx = daemon.browse(SERVICE_TYPE).map_err(err)?;
    Ok(Browse {
        daemon: Some(daemon),
        rx,
        tracker: Tracker::default(),
    })
}

impl Browse {
    /// The next event, or `None` once browsing has ended.
    pub async fn next(&mut self) -> Option<NearbyEvent> {
        loop {
            let raw = self.rx.recv_async().await.ok()?;
            if let Some(event) = self.tracker.observe(raw) {
                return Some(event);
            }
        }
    }

    /// Blocking variant of [`Browse::next`]: waits up to `timeout`.
    pub fn next_blocking(&mut self, timeout: Duration) -> Option<NearbyEvent> {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            let raw = self.rx.recv_timeout(left).ok()?;
            if let Some(event) = self.tracker.observe(raw) {
                return Some(event);
            }
        }
    }

    /// How many records were ignored because their TXT failed to parse.
    pub fn ignored_count(&self) -> usize {
        self.tracker.ignored
    }
}

impl Drop for Browse {
    fn drop(&mut self) {
        if let Some(daemon) = self.daemon.take() {
            let _ = daemon.stop_browse(SERVICE_TYPE);
            if let Ok(done) = daemon.shutdown() {
                let _ = done.recv_timeout(DAEMON_WAIT);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proto::model::AccessType;

    fn txt(name: &str) -> TxtRecord {
        TxtRecord {
            v: proto::PROTO_VERSION,
            id: EventId::from_random_bytes([9, 8, 7, 6, 5, 4]),
            name: name.to_owned(),
            host: "Host".to_owned(),
            access: AccessType::Password,
            st: 10,
            en: 20,
            fp: "ABCDEF012345".to_owned(),
        }
    }

    fn resolved(full: &str, t: &TxtRecord, port: u16) -> Resolved {
        Resolved {
            fullname: full.to_owned(),
            pairs: t
                .to_pairs()
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v))
                .collect(),
            addresses: vec!["192.168.1.5".parse().unwrap()],
            port,
        }
    }

    #[test]
    fn found_updated_lost() {
        let mut tr = Tracker::default();
        let t = txt("A");
        let Some(NearbyEvent::Found(s)) = tr.resolved(resolved("x", &t, 1)) else {
            panic!("expected found");
        };
        assert_eq!(s.txt, t);
        assert_eq!(s.port, 1);
        // Identical re-resolution is not an event.
        assert_eq!(tr.resolved(resolved("x", &t, 1)), None);
        // Changed port and changed TXT are updates.
        assert!(matches!(
            tr.resolved(resolved("x", &t, 2)),
            Some(NearbyEvent::Updated(_))
        ));
        let t2 = txt("B");
        assert!(matches!(
            tr.resolved(resolved("x", &t2, 2)),
            Some(NearbyEvent::Updated(s)) if s.txt.name == "B"
        ));
        assert_eq!(tr.removed("x"), Some(NearbyEvent::Lost(t.id.clone())));
        assert_eq!(tr.removed("x"), None);
    }

    #[test]
    fn malformed_records_are_ignored_and_counted() {
        let mut tr = Tracker::default();
        let mut r = resolved("bad", &txt("A"), 1);
        r.pairs.retain(|(k, _)| k != "fp");
        assert_eq!(tr.resolved(r), None);
        let mut r = resolved("bad2", &txt("A"), 1);
        r.pairs[0].1 = "not a number".to_owned();
        assert_eq!(tr.resolved(r), None);
        assert_eq!(tr.ignored, 2);
        assert_eq!(tr.removed("bad"), None);
    }

    #[test]
    fn valid_record_turning_invalid_is_lost() {
        let mut tr = Tracker::default();
        let t = txt("A");
        tr.resolved(resolved("x", &t, 1));
        let mut r = resolved("x", &t, 1);
        r.pairs.clear();
        assert_eq!(tr.resolved(r), Some(NearbyEvent::Lost(t.id)));
        assert_eq!(tr.ignored, 1);
    }

    #[test]
    fn addresses_are_sorted() {
        let mut tr = Tracker::default();
        let mut r = resolved("x", &txt("A"), 1);
        r.addresses = vec!["10.0.0.9".parse().unwrap(), "10.0.0.2".parse().unwrap()];
        let Some(NearbyEvent::Found(s)) = tr.resolved(r) else {
            panic!()
        };
        assert_eq!(s.addresses[0].to_string(), "10.0.0.2");
    }
}
