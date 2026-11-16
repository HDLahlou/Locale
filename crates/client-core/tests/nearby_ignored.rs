//! Real-network check that a malformed record is never surfaced and is counted.
//! Registers raw services with `mdns-sd` (no host-core), one valid and one
//! with a TXT that fails `TxtRecord::from_pairs`.

use std::net::{Ipv4Addr, UdpSocket};
use std::time::{Duration, Instant};

use client_core::nearby::{browse, NearbyEvent};
use mdns_sd::{ServiceDaemon, ServiceInfo};
use proto::mdns::{TxtRecord, SERVICE_TYPE};
use proto::model::AccessType;
use proto::transport::EventId;

fn random_id(salt: u8) -> EventId {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .to_le_bytes();
    let p = std::process::id().to_le_bytes();
    EventId::from_random_bytes([t[0], t[1], t[2], p[0], p[1], salt])
}

#[test]
fn nearby_malformed_record_is_ignored_counted_and_never_surfaced() {
    let probe = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))
        .and_then(|s| s.join_multicast_v4(&Ipv4Addr::new(224, 0, 0, 251), &Ipv4Addr::UNSPECIFIED));
    if let Err(e) = probe {
        panic!("this machine has no usable multicast interface (cannot join 224.0.0.251): {e}");
    }
    let good = TxtRecord {
        v: proto::PROTO_VERSION,
        id: random_id(1),
        name: "Good".to_owned(),
        host: "Host".to_owned(),
        access: AccessType::Open,
        st: 1,
        en: 2,
        fp: "0123456789AB".to_owned(),
    };
    let bad_id = random_id(2);
    let daemon = ServiceDaemon::new().unwrap();
    // Missing every key but `id`: fails from_pairs.
    let bad = ServiceInfo::new(
        SERVICE_TYPE,
        bad_id.as_str(),
        "bad-host.local.",
        "",
        41000,
        [("id", bad_id.as_str())].as_slice(),
    )
    .unwrap()
    .enable_addr_auto();
    let pairs: Vec<(&str, String)> = good.to_pairs();
    let good_info = ServiceInfo::new(
        SERVICE_TYPE,
        good.id.as_str(),
        "good-host.local.",
        "",
        41001,
        pairs.as_slice(),
    )
    .unwrap()
    .enable_addr_auto();
    daemon.register(bad).unwrap();
    daemon.register(good_info).unwrap();

    let mut b = browse().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match b.next_blocking(left) {
            Some(NearbyEvent::Found(s)) => {
                assert_ne!(s.txt.id, bad_id, "malformed record was surfaced");
                if s.txt.id == good.id {
                    assert_eq!(s.txt, good);
                    assert_eq!(s.port, 41001);
                    break;
                }
            }
            Some(_) => {}
            None => panic!("valid record not found within 5 s"),
        }
    }
    // The bad record resolves at about the same time; give it a moment.
    let deadline = Instant::now() + Duration::from_secs(5);
    while b.ignored_count() == 0 && Instant::now() < deadline {
        if let Some(NearbyEvent::Found(s) | NearbyEvent::Updated(s)) =
            b.next_blocking(Duration::from_millis(100))
        {
            assert_ne!(s.txt.id, bad_id, "malformed record was surfaced");
        }
    }
    assert!(b.ignored_count() >= 1, "malformed record was not counted");
    let _ = daemon.shutdown();
}
