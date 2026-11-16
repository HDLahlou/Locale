mod capability {
    use std::net::Ipv4Addr;
    use std::path::PathBuf;

    use host_core::capability::test_support::{FakeAnswers, FakeProbe};
    use host_core::capability::*;

    const GB: u64 = 1_000_000_000;

    fn req(platform: Platform) -> CapabilityRequest {
        CapabilityRequest {
            allocation_bytes: 20 * GB,
            folder: PathBuf::from("/x"),
            port: 8443,
            platform,
        }
    }

    fn row(r: &CapabilityReport, k: CheckKind) -> CheckRow {
        r.rows.iter().find(|x| x.check == k).unwrap().clone()
    }

    fn json<T: serde::Serialize>(v: T) -> String {
        serde_json::to_string(&v).unwrap()
    }

    #[test]
    fn all_pass_in_order() {
        let r = run_capability_check(&FakeProbe::new(), &req(Platform::Windows));
        let kinds: Vec<_> = r.rows.iter().map(|x| x.check).collect();
        assert_eq!(
            kinds,
            [
                CheckKind::Network,
                CheckKind::Firewall,
                CheckKind::Space,
                CheckKind::Power
            ]
        );
        assert!(r
            .rows
            .iter()
            .all(|x| x.status == CheckStatus::Ok && x.reason.is_none() && x.fix_id.is_none()));
        assert!(r.can_start);
    }

    #[test]
    fn hotspot_on_passes_network() {
        let p = FakeProbe::new();
        p.set(|a| a.network = NetworkState::HotspotOn);
        assert!(run_capability_check(&p, &req(Platform::Windows)).can_start);
    }

    #[test]
    fn network_failure_fix_depends_on_platform() {
        let p = FakeProbe::new();
        p.set(|a| a.network = NetworkState::None);
        for (plat, fix) in [
            (Platform::Windows, FixId::TurnOnHotspot),
            (Platform::Macos, FixId::ShowNetworkInstructions),
        ] {
            let r = run_capability_check(&p, &req(plat));
            let n = row(&r, CheckKind::Network);
            assert_eq!(
                (n.status, n.reason, n.fix_id),
                (CheckStatus::Failed, Some(Reason::NoNetwork), Some(fix))
            );
            assert!(!r.can_start);
        }
    }

    #[test]
    fn firewall_blocked_fails() {
        let p = FakeProbe::new();
        p.set(|a| a.firewall = FirewallState::Blocked);
        let r = run_capability_check(&p, &req(Platform::Macos));
        let f = row(&r, CheckKind::Firewall);
        assert_eq!(
            (f.status, f.reason, f.fix_id),
            (
                CheckStatus::Failed,
                Some(Reason::FirewallBlocked),
                Some(FixId::AllowFirewall)
            )
        );
        assert!(!r.can_start);
    }

    #[test]
    fn firewall_unknown_warns_without_blocking() {
        let p = FakeProbe::new();
        p.set(|a| a.firewall = FirewallState::Unknown);
        let r = run_capability_check(&p, &req(Platform::Macos));
        let f = row(&r, CheckKind::Firewall);
        assert_eq!(f.status, CheckStatus::Warning);
        assert_eq!(f.reason, Some(Reason::FirewallUnknown));
        assert!(r.can_start);
    }

    #[test]
    fn space_boundary_and_bytes() {
        let p = FakeProbe::new();
        p.set(|a| a.free_bytes = 20 * GB);
        let r = run_capability_check(&p, &req(Platform::Windows));
        assert_eq!(row(&r, CheckKind::Space).status, CheckStatus::Ok);
        p.set(|a| a.free_bytes = 20 * GB - 1);
        let r = run_capability_check(&p, &req(Platform::Windows));
        let s = row(&r, CheckKind::Space);
        assert_eq!(s.status, CheckStatus::Failed);
        assert_eq!(s.reason, Some(Reason::LowSpace));
        assert_eq!(s.fix_id, Some(FixId::ChooseSmallerSize));
        assert_eq!(
            (s.free_bytes, s.required_bytes),
            (Some(20 * GB - 1), Some(20 * GB))
        );
        assert!(!r.can_start);
    }

    #[test]
    fn power_table() {
        let cases = [
            (PowerState::Desktop, true),
            (PowerState::PluggedIn, true),
            (PowerState::Battery { percent: 100 }, true),
            (PowerState::Battery { percent: 51 }, true),
            (PowerState::Battery { percent: 50 }, false),
            (PowerState::Battery { percent: 0 }, false),
        ];
        for (power, ok) in cases {
            let p = FakeProbe::new();
            p.set(|a| a.power = power);
            let r = run_capability_check(&p, &req(Platform::Windows));
            let w = row(&r, CheckKind::Power);
            assert_eq!(r.can_start, ok, "{power:?}");
            if !ok {
                assert_eq!(
                    (w.reason, w.fix_id),
                    (Some(Reason::LowPower), Some(FixId::PlugIn))
                );
            }
        }
    }

    #[test]
    fn multiple_failures_all_reported() {
        let p = FakeProbe::new();
        p.set(|a| {
            a.network = NetworkState::None;
            a.firewall = FirewallState::Blocked;
            a.free_bytes = 0;
            a.power = PowerState::Battery { percent: 10 };
        });
        let r = run_capability_check(&p, &req(Platform::Windows));
        let failed = r.rows.iter().filter(|x| x.status == CheckStatus::Failed);
        assert_eq!(failed.count(), 4);
        assert!(!r.can_start);
    }

    #[test]
    fn enums_have_stable_snake_case_forms() {
        assert_eq!(json(Reason::NoNetwork), "\"no_network\"");
        assert_eq!(json(Reason::FirewallBlocked), "\"firewall_blocked\"");
        assert_eq!(json(Reason::FirewallUnknown), "\"firewall_unknown\"");
        assert_eq!(json(Reason::LowSpace), "\"low_space\"");
        assert_eq!(json(Reason::LowPower), "\"low_power\"");
        assert_eq!(json(FixId::TurnOnHotspot), "\"turn_on_hotspot\"");
        assert_eq!(
            json(FixId::ShowNetworkInstructions),
            "\"show_network_instructions\""
        );
        assert_eq!(json(FixId::AllowFirewall), "\"allow_firewall\"");
        assert_eq!(json(FixId::ChooseSmallerSize), "\"choose_smaller_size\"");
        assert_eq!(json(FixId::PlugIn), "\"plug_in\"");
        assert_eq!(json(Reachability::Isolated), "\"isolated\"");
        assert_eq!(json(CheckStatus::Warning), "\"warning\"");
    }

    // Reachability.

    async fn reach(f: impl FnOnce(&mut FakeAnswers)) -> Reachability {
        let p = FakeProbe::new();
        p.set(f);
        run_reachability_test(&p).await
    }

    #[tokio::test(start_paused = true)]
    async fn reachability_branches() {
        use SelfConnect::*;
        assert_eq!(reach(|_| {}).await, Reachability::Ok);
        // Private, self-connect ok, no peer.
        assert_eq!(reach(|a| a.mdns_reply = false).await, Reachability::Ok);
        // Private, peer replied, self-connect not permitted.
        assert_eq!(
            reach(|a| a.self_connect = NotPermitted).await,
            Reachability::Ok
        );
        // Not permitted and no peer.
        let r = reach(|a| {
            a.self_connect = NotPermitted;
            a.mdns_reply = false;
        })
        .await;
        assert_eq!(r, Reachability::Unknown);
        // Self-connect failed, even with a peer.
        assert_eq!(
            reach(|a| a.self_connect = Failed).await,
            Reachability::Isolated
        );
        for class in [
            AddressClass::Public,
            AddressClass::CaptiveOrIsolated,
            AddressClass::NoAddress,
        ] {
            let r = reach(|a| a.address_class = class).await;
            assert_eq!(r, Reachability::Isolated);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn hanging_mdns_is_bounded_by_two_seconds() {
        let start = tokio::time::Instant::now();
        let r = reach(|a| {
            a.mdns_hangs = true;
            a.self_connect = SelfConnect::NotPermitted;
        })
        .await;
        assert_eq!(r, Reachability::Unknown);
        assert_eq!(start.elapsed(), MDNS_WAIT);
        // With a successful self-connection a hung mDNS still yields ok.
        assert_eq!(reach(|a| a.mdns_hangs = true).await, Reachability::Ok);
    }

    #[tokio::test(start_paused = true)]
    async fn quick_check_uses_five_gb_and_reachability() {
        let p = FakeProbe::new();
        p.set(|a| {
            a.free_bytes = 5 * GB;
            a.address_class = AddressClass::Public;
        });
        // Request asks for 50 GB; quick session overrides to 5 GB.
        let mut r = req(Platform::Windows);
        r.allocation_bytes = 50 * GB;
        let q = run_quick_check(&p, &r).await;
        assert!(q.capability.can_start);
        assert_eq!(q.reachability, Reachability::Isolated);
        p.set(|a| a.free_bytes = 5 * GB - 1);
        let q = run_quick_check(&p, &r).await;
        let space = row(&q.capability, CheckKind::Space);
        assert_eq!(space.required_bytes, Some(QUICK_ALLOCATION_BYTES));
        assert!(!q.capability.can_start);
    }

    // Real probe.

    #[test]
    fn classify_ranges() {
        let c = |a, b, c_, d| classify_ipv4(Ipv4Addr::new(a, b, c_, d));
        assert_eq!(c(10, 0, 0, 5), AddressClass::Private);
        assert_eq!(c(172, 16, 0, 1), AddressClass::Private);
        assert_eq!(c(192, 168, 1, 1), AddressClass::Private);
        assert_eq!(c(8, 8, 8, 8), AddressClass::Public);
        assert_eq!(c(169, 254, 1, 1), AddressClass::CaptiveOrIsolated);
        assert_eq!(c(100, 64, 0, 1), AddressClass::CaptiveOrIsolated);
        assert_eq!(c(100, 128, 0, 1), AddressClass::Public);
    }

    #[test]
    fn real_probe_free_space_and_port_binding() {
        let dir = tempfile::tempdir().unwrap();
        assert!(RealProbe.free_bytes(dir.path()) > 0);
        let l = std::net::TcpListener::bind(("0.0.0.0", 0)).unwrap();
        let port = l.local_addr().unwrap().port();
        assert!(!RealProbe::port_is_free(port));
        drop(l);
        assert!(RealProbe::port_is_free(port));
    }
}
