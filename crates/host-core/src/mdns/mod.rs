//! mDNS advertising of an event (digest §5.6, DU-32).
//!
//! The service type, TXT keys and TXT parsing come from `proto::mdns`. The
//! caller (task M1-8) decides which events are advertised and passes a
//! [`TxtRecord`] and the listener port. Backed by `mdns-sd`, a pure-Rust
//! implementation with no OS-specific code.

use std::fmt;
use std::time::Duration;

use mdns_sd::{ServiceDaemon, ServiceInfo};
use proto::mdns::{keys, TxtRecord, SERVICE_TYPE};

/// Longest DNS-SD TXT entry (`key=value`) in bytes (RFC 6763 §6.1: one length byte).
pub const TXT_ENTRY_MAX: usize = 255;

/// How long to wait for the daemon to confirm a goodbye or a shutdown.
const DAEMON_WAIT: Duration = Duration::from_secs(2);

/// Why advertising failed.
#[derive(Debug)]
pub struct MdnsError(String);

impl fmt::Display for MdnsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "mDNS advertising failed: {}", self.0)
    }
}

impl std::error::Error for MdnsError {}

fn err(e: impl fmt::Display) -> MdnsError {
    MdnsError(e.to_string())
}

/// The longest prefix of `s` that is at most `max_bytes` long and ends on a
/// character boundary.
pub fn truncate_to_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Returns `txt` with `name` and `host` shortened so that every TXT entry
/// (`key=value`) fits in [`TXT_ENTRY_MAX`] bytes. Each value is cut to
/// `255 - len(key) - 1` bytes, backing up to a UTF-8 character boundary so a
/// multi-byte character is dropped whole, never split. Other fields are short
/// by construction.
pub fn fit_txt(mut txt: TxtRecord) -> TxtRecord {
    let cap = |key: &str| TXT_ENTRY_MAX - key.len() - 1;
    let name = truncate_to_boundary(&txt.name, cap(keys::NAME)).len();
    txt.name.truncate(name);
    let host = truncate_to_boundary(&txt.host, cap(keys::HOST)).len();
    txt.host.truncate(host);
    txt
}

/// A live advertisement. The record is withdrawn (goodbye sent) on
/// [`Advertiser::stop`] or drop.
pub struct Advertiser {
    daemon: Option<ServiceDaemon>,
    fullname: String,
    txt: TxtRecord,
    port: u16,
}

impl Advertiser {
    /// Registers the service: instance name = event ID, TXT from
    /// [`TxtRecord::to_pairs`] (after [`fit_txt`]), addresses of all usable
    /// interfaces, kept current as they change.
    pub fn start(txt: TxtRecord, port: u16) -> Result<Self, MdnsError> {
        let daemon = ServiceDaemon::new().map_err(err)?;
        let mut adv = Advertiser {
            daemon: Some(daemon),
            fullname: String::new(),
            txt: fit_txt(txt),
            port,
        };
        adv.register()?;
        Ok(adv)
    }

    fn info(txt: &TxtRecord, port: u16) -> Result<ServiceInfo, MdnsError> {
        let pairs: Vec<(&str, String)> = txt.to_pairs();
        let host = format!(
            "locale-{}.local.",
            txt.instance_name().trim_start_matches("evt_")
        );
        Ok(ServiceInfo::new(
            SERVICE_TYPE,
            txt.instance_name(),
            &host,
            "",
            port,
            pairs.as_slice(),
        )
        .map_err(err)?
        .enable_addr_auto())
    }

    fn register(&mut self) -> Result<(), MdnsError> {
        let info = Self::info(&self.txt, self.port)?;
        self.fullname = info.get_fullname().to_owned();
        self.daemon
            .as_ref()
            .ok_or_else(|| err("stopped"))?
            .register(info)
            .map_err(err)
    }

    fn withdraw(&self) {
        if let Some(daemon) = &self.daemon {
            if let Ok(done) = daemon.unregister(&self.fullname) {
                let _ = done.recv_timeout(DAEMON_WAIT);
            }
        }
    }

    /// Replaces the advertised record after a change of TXT content or port
    /// (a changed address is tracked automatically). The old record is
    /// withdrawn first, so a changed event ID leaves no stale instance behind.
    pub fn readvertise(&mut self, txt: TxtRecord, port: u16) -> Result<(), MdnsError> {
        self.withdraw();
        self.txt = fit_txt(txt);
        self.port = port;
        self.register()
    }

    /// The record as advertised (after truncation).
    pub fn advertised(&self) -> &TxtRecord {
        &self.txt
    }

    /// The advertised port.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Withdraws the record and frees the sockets.
    pub fn stop(self) {}

    fn shutdown(&mut self) {
        if self.daemon.is_some() {
            self.withdraw();
            if let Some(daemon) = self.daemon.take() {
                if let Ok(done) = daemon.shutdown() {
                    let _ = done.recv_timeout(DAEMON_WAIT);
                }
            }
        }
    }
}

impl Drop for Advertiser {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proto::model::AccessType;
    use proto::transport::EventId;

    fn record(name: &str, host: &str) -> TxtRecord {
        TxtRecord {
            v: proto::PROTO_VERSION,
            id: EventId::from_random_bytes([1, 2, 3, 4, 5, 6]),
            name: name.to_owned(),
            host: host.to_owned(),
            access: AccessType::Open,
            st: 1_700_000_000_000,
            en: 1_700_086_400_000,
            fp: "0123456789AB".to_owned(),
        }
    }

    #[test]
    fn pairs_round_trip_through_proto() {
        let txt = record("Maya's 30th \u{1F389}", "Hakim");
        let pairs = txt.to_pairs();
        assert_eq!(pairs.len(), keys::ALL.len());
        assert_eq!(TxtRecord::from_pairs(pairs).unwrap(), txt);
    }

    #[test]
    fn short_values_are_unchanged() {
        let txt = record("Beach day", "Sam");
        assert_eq!(fit_txt(txt.clone()), txt);
    }

    #[test]
    fn long_name_is_cut_on_a_character_boundary() {
        // "name=" takes 5 bytes, leaving 250. Four-byte characters: 62 fit (248 bytes), not 63.
        let txt = fit_txt(record(&"\u{1F389}".repeat(100), "h"));
        assert_eq!(txt.name, "\u{1F389}".repeat(62));
        assert!(format!("name={}", txt.name).len() <= TXT_ENTRY_MAX);
    }

    #[test]
    fn name_at_the_limit_is_kept_exactly() {
        let name = "a".repeat(250);
        assert_eq!(fit_txt(record(&name, "h")).name, name);
        let over = "a".repeat(251);
        assert_eq!(fit_txt(record(&over, "h")).name.len(), 250);
    }

    #[test]
    fn long_host_is_cut_to_its_own_budget() {
        // "host=" is 5 bytes; three-byte characters: 83 fit (249 bytes).
        let txt = fit_txt(record("n", &"\u{20AC}".repeat(200)));
        assert_eq!(txt.host.chars().count(), 83);
        assert!(format!("host={}", txt.host).len() <= TXT_ENTRY_MAX);
    }

    #[test]
    fn malformed_pairs_are_rejected_by_proto() {
        let mut pairs = record("n", "h").to_pairs();
        pairs.retain(|(k, _)| *k != keys::FP);
        assert!(TxtRecord::from_pairs(pairs).is_err());
        let mut pairs = record("n", "h").to_pairs();
        pairs[4].1 = "invite".to_owned();
        assert!(TxtRecord::from_pairs(pairs).is_err());
    }
}
