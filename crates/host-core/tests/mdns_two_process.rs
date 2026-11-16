//! Real-network mDNS tests: advertising in one process, browsing in another.
//!
//! The child process is this same test binary, re-run on `mdns_child_entry` with
//! `LOCALE_MDNS_CHILD` set. Every run uses a fresh random event ID, so parallel
//! or repeated runs never see each other's records.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, UdpSocket};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use client_core::nearby::{browse, NearbyEvent};
use host_core::mdns::{fit_txt, Advertiser};
use proto::mdns::TxtRecord;
use proto::model::AccessType;
use proto::transport::EventId;

const FOUND_WITHIN: Duration = Duration::from_secs(5);
const LOST_WITHIN: Duration = Duration::from_secs(10);

fn random_bytes() -> [u8; 6] {
    let mut h = RandomState::new().build_hasher();
    h.write_u32(std::process::id());
    h.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    );
    let n = h.finish().to_le_bytes();
    [n[0], n[1], n[2], n[3], n[4], n[5]]
}

fn record(name: &str, host: &str) -> TxtRecord {
    TxtRecord {
        v: proto::PROTO_VERSION,
        id: EventId::from_random_bytes(random_bytes()),
        name: name.to_owned(),
        host: host.to_owned(),
        access: AccessType::Password,
        st: 1_700_000_000_000,
        en: 1_700_086_400_000,
        fp: "0123456789AB".to_owned(),
    }
}

/// Fails plainly (does not skip) when the machine cannot do multicast.
fn require_multicast() {
    let probe = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))
        .and_then(|s| s.join_multicast_v4(&Ipv4Addr::new(224, 0, 0, 251), &Ipv4Addr::UNSPECIFIED));
    if let Err(e) = probe {
        panic!("this machine has no usable multicast interface (cannot join 224.0.0.251): {e}");
    }
}

fn name_for(case: usize) -> (String, String) {
    if case == 1 {
        ("\u{e9}".repeat(125), "x".repeat(250))
    } else {
        (
            "Maya\u{2019}s 30\u{e8}me \u{1F389} \u{5370}\u{5237}".to_owned(),
            "H\u{e9}l\u{e8}ne".to_owned(),
        )
    }
}

// ---- child side ----

/// Not a real test: the body of the child process. Returns at once unless
/// `LOCALE_MDNS_CHILD` is set.
#[test]
fn mdns_child_entry() {
    let Ok(role) = std::env::var("LOCALE_MDNS_CHILD") else {
        return;
    };
    let say = |line: String| {
        println!("\nCHILD:{line}");
        std::io::stdout().flush().unwrap();
    };
    match role.as_str() {
        "advertise" => {
            let case: usize = std::env::var("LOCALE_MDNS_CASE").unwrap().parse().unwrap();
            let (name, host) = name_for(case);
            let mut txt = record(&name, &host);
            txt.id = EventId::parse(&std::env::var("LOCALE_MDNS_ID").unwrap()).unwrap();
            let adv = Advertiser::start(txt, 41234).unwrap();
            say("READY".to_owned());
            // Run until the parent closes our stdin, then withdraw.
            let mut line = String::new();
            while std::io::stdin().read_line(&mut line).unwrap_or(0) > 0 {}
            adv.stop();
            say("STOPPED".to_owned());
        }
        "browse" => {
            let want = std::env::var("LOCALE_MDNS_ID").unwrap();
            let mut b = browse().unwrap();
            say("READY".to_owned());
            let deadline = Instant::now() + Duration::from_secs(30);
            while Instant::now() < deadline {
                match b.next_blocking(Duration::from_millis(200)) {
                    Some(NearbyEvent::Found(s)) if s.txt.id.as_str() == want => {
                        say(format!("FOUND {} {:?}", s.port, s.txt));
                    }
                    Some(NearbyEvent::Lost(id)) if id.as_str() == want => {
                        say("LOST".to_owned());
                        return;
                    }
                    _ => {}
                }
            }
        }
        other => panic!("unknown role {other}"),
    }
}

struct ChildProc {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
}

impl ChildProc {
    fn spawn(role: &str, id: &EventId, case: usize) -> ChildProc {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "mdns_child_entry", "--nocapture", "--test-threads=1"])
            .env("LOCALE_MDNS_CHILD", role)
            .env("LOCALE_MDNS_ID", id.as_str())
            .env("LOCALE_MDNS_CASE", case.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("spawn child");
        let stdin = child.stdin.take();
        let out = child.stdout.take().unwrap();
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                if let Some(rest) = line.strip_prefix("CHILD:") {
                    let _ = tx.send(rest.to_owned());
                }
            }
        });
        ChildProc {
            child,
            stdin,
            lines,
        }
    }

    fn expect(&self, prefix: &str, within: Duration) -> String {
        let deadline = Instant::now() + within;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.lines.recv_timeout(left) {
                Ok(l) if l.starts_with(prefix) => return l,
                Ok(_) => {}
                Err(_) => panic!("child did not report {prefix:?} within {within:?}"),
            }
        }
    }

    fn close_stdin(&mut self) {
        self.stdin = None;
    }
}

impl Drop for ChildProc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// ---- parent side ----

fn child_advertises(case: usize) {
    require_multicast();
    let (name, host) = name_for(case);
    let advertised = fit_txt(record(&name, &host));
    assert_eq!(advertised.name, name, "test record must fit the TXT limit");
    let mut child = ChildProc::spawn("advertise", &advertised.id, case);
    child.expect("READY", Duration::from_secs(10));

    let mut b = browse().unwrap();
    let deadline = Instant::now() + FOUND_WITHIN;
    let found = loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match b.next_blocking(left) {
            Some(NearbyEvent::Found(s)) if s.txt.id == advertised.id => break s,
            Some(_) => {}
            None => panic!(
                "advertised record not found within {FOUND_WITHIN:?}; \
                 check that this machine has a multicast-capable interface"
            ),
        }
    };
    assert_eq!(found.port, 41234);
    assert!(!found.addresses.is_empty());
    let mut expected = advertised.clone();
    expected.name = name;
    assert_eq!(found.txt, expected);
    assert_eq!(b.ignored_count(), 0);

    child.close_stdin();
    let deadline = Instant::now() + LOST_WITHIN;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match b.next_blocking(left) {
            Some(NearbyEvent::Lost(id)) if id == advertised.id => break,
            Some(_) => {}
            None => panic!("no Lost event within {LOST_WITHIN:?} after the child stopped"),
        }
    }
}

#[test]
fn mdns_child_advertises_non_ascii_name() {
    child_advertises(0);
}

#[test]
fn mdns_child_advertises_name_at_txt_limit() {
    child_advertises(1);
}

#[test]
fn mdns_parent_advertises_child_browses() {
    require_multicast();
    let (name, host) = name_for(0);
    let txt = record(&name, &host);
    let mut child = ChildProc::spawn("browse", &txt.id, 0);
    child.expect("READY", Duration::from_secs(10));
    let adv = Advertiser::start(txt.clone(), 41234).unwrap();

    let line = child.expect("FOUND", FOUND_WITHIN);
    assert_eq!(line, format!("FOUND 41234 {txt:?}"));
    adv.stop();
    child.expect("LOST", LOST_WITHIN);
    child.close_stdin();
}

#[test]
fn mdns_parent_advertises_name_at_txt_limit_child_browses() {
    require_multicast();
    let (name, host) = name_for(1);
    let txt = record(&name, &host);
    assert_eq!(fit_txt(txt.clone()), txt);
    let child = ChildProc::spawn("browse", &txt.id, 1);
    child.expect("READY", Duration::from_secs(10));
    let _adv = Advertiser::start(txt.clone(), 41234).unwrap();
    let line = child.expect("FOUND", FOUND_WITHIN);
    assert_eq!(line, format!("FOUND 41234 {txt:?}"));
}

#[test]
fn mdns_readvertise_replaces_the_old_record() {
    require_multicast();
    let txt = record("First name", "Host");
    let mut adv = Advertiser::start(txt.clone(), 41234).unwrap();
    let mut b = browse().unwrap();
    let deadline = Instant::now() + FOUND_WITHIN;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match b.next_blocking(left) {
            Some(NearbyEvent::Found(s)) if s.txt.id == txt.id => {
                assert_eq!(s.txt.name, "First name");
                break;
            }
            Some(_) => {}
            None => panic!("record not found within {FOUND_WITHIN:?}"),
        }
    }
    let mut changed = txt.clone();
    changed.name = "Second name".to_owned();
    adv.readvertise(changed, 41235).unwrap();
    let deadline = Instant::now() + FOUND_WITHIN;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match b.next_blocking(left) {
            Some(NearbyEvent::Found(s) | NearbyEvent::Updated(s)) if s.txt.id == txt.id => {
                if s.txt.name == "Second name" {
                    assert_eq!(s.port, 41235);
                    break;
                }
            }
            Some(_) => {}
            None => panic!("replacement record not seen within {FOUND_WITHIN:?}"),
        }
    }
}
