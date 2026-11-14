# Locale MVP: work breakdown

Nov 13, 2026 · Milestones follow MVP §11 (M0a–M5), with a setup milestone S added in front. 92 tasks. All open questions were resolved on Nov 13, 2026 (`open-questions.md`); gates now name the setup-checklist items they need. Phone work is one Flutter app (`docs/Locale Design Tradeoffs.md`, TR-1); IDs M1-29, M1-30 and M2-17 were retired when the separate iOS and Android tasks merged. Section references ("§4.4", "DU-13") point into `spec-digest.md`; screen IDs ("P-05", "section D") point into `screen-inventory.md`.

## How to read a task row

- **Digest:** the digest sections the implementer needs. Every implementer also reads digest §12–§13.
- **Accept:** the acceptance criteria. A task is done only when all hold.
- **Verify:** the check a verifier agent runs. Command names are fixed by task S1; until then they are proposals: `just check` (format, lint, unit tests for the whole workspace), `cargo test -p <crate> <filter>`, `pnpm -C apps/<app> test` (component tests), `pnpm -C apps/<app> e2e` (Playwright), `flutter test` (Dart unit, widget and golden tests, headless, no simulator), `just mobile-it ios|android <suite>` (Flutter integration tests on the iOS simulator or Android emulator), `just plugin-test ios|android` (native unit tests inside our Flutter plugins), `just harness <scenario>` (the multi-client harness from M1-16).
- **Size:** S = under half a day of agent work and one file group; M = one module with tests; L = several modules, split further if it fails verification twice.
- **Model:** H = Haiku 5.5 low, S = Sonnet 5.5 medium, O = Opus 5.5 high. The verifier is always Sonnet in a fresh context.
- **Gate:** "—" means no person is needed. Anything else is a human gate and names what the person does.

**The harness.** Most verification leans on one tool built in M1-16: a Rust test harness that starts a real host core on loopback and drives N simulated clients built from the real client core, with a controllable clock and fault injection (drop the connection, delay, corrupt bytes, kill and restart the host). It makes multi-device behaviour testable on one machine. It cannot test real radios, OS permission prompts, firewalls, hotspots or phone backgrounding; those are the human gates.

**Dependency summary.** S1 → everything. S3 (interface contract, approved) → all M1 and M2 tasks. S6 (CI) → every Windows verification. M0a de-risks M2's image and transfer work but only M0a-10 (the device check) is a gate. M0b is independent and feeds only M5. Within M1 and M2, host-core, client-core and the three UI surfaces (desktop, guest web, Flutter phone app) run in parallel once S3 is approved.

---

## S. Setup (agents, one approval gate)

| ID | Goal | Digest | Accept | Verify | Deps | Size | Model | Gate |
|---|---|---|---|---|---|---|---|---|
| S1 | Repository skeleton: Cargo workspace, app folders, `justfile`, `CLAUDE.md` (stack, commands, conventions), ledger file | §3, DU-7 | Empty workspace builds; `just check` passes; every command named above exists (may be a stub that says "no tests") | `just check` | — | M | S | — |
| S2 | Toolchain probe script: Rust targets, Node, Flutter SDK and `flutter_rust_bridge` codegen, Xcode + iOS simulator, Android SDK/NDK + emulator, Tauri prerequisites, libheif | §3 | Script prints a table of present/missing tools and exits non-zero when a tool needed for M1 is missing | `just doctor` | S1 | S | H | **Person installs what is missing** |
| S3 | `crates/proto`: wire types, error and reason codes, change types, credential format, config struct, generated JSON Schema, and a contract document rendered from the code | §5, §6, §7 | Every endpoint, message and code in digest §5 has a type; round-trip serialisation tests; schema files generated | `cargo test -p proto` | S1 | M | O | — (contract approved Nov 13, 2026, Q2; later changes go through the orchestrator) |
| S4 | Test fixture set: script-made JPEG, PNG and HEIC (via `sips`), truncated and wrong-magic files, a 50 MB JPEG, files with GPS EXIF and XMP; manifest with hashes | §4.4, §8.5 | Manifest lists each file, format, size, hash, whether it has location data | `just fixtures-verify` | S1 | S | H | **Person adds real device photos:** iPhone HEIC with HDR gain map, Live Photo still, Android Ultra HDR JPEG, Android HEIC (needed by M0a-5, M2-3) |
| S5 | Design tokens and shared UI kit for web surfaces (buttons, pills, tabs, status labels, dialog, skeleton), light and dark; a Dart theme file generated from the same source | §9, DU-39 | Tokens match digest §9 values; contrast ≥ 4.5:1 for text pairs; component tests render both themes | `pnpm -C apps/desktop test ui-kit` | S1 | M | S | — |
| S6 | GitHub Actions CI: macOS and Windows jobs for the Rust workspace and image worker, guest-web tests, Flutter `flutter test`; triggered on `ci/*` branches and pull requests | §3; TR-12 | A pushed `ci/*` branch runs all jobs; a deliberately failing Windows-only test turns the run red | Push a `ci/smoke` branch and read the run result with `gh run view` | S1, S2 | S | S | **Person enables Actions and grants push rights for `ci/*` (setup items 5–6)** |

---

## M0a. Host core spike

Exit (MVP §11): SC1 and SC4 met in test apps; HEIC round trip byte-identical.

| ID | Goal | Digest | Accept | Verify | Deps | Size | Model | Gate |
|---|---|---|---|---|---|---|---|---|
| M0a-1 | Single-port listener: TLS (per-event self-signed certificate) and plain HTTP by first-byte sniffing; fingerprint and host code | §5.1, §8.1 | A pinned client connects; a client with a different pin fails; plain HTTP GET works on the same port | `cargo test -p host-core listener` | S1 | M | O | — |
| M0a-2 | Resumable chunk transfer (spike of §5.3 upload endpoints) and a client that resumes from the host's offset | §4.4 steps 11–12, §5.3 | 50 MB file uploads on loopback; killing the connection at 3 random points still ends with an identical hash; no chunk is re-sent beyond the last partial one | `cargo test -p harness spike_resume` | M0a-1 | M | S | — |
| M0a-3 | `ImageCodec` on macOS (ImageIO): sniff, decode, facts, preview, compatible JPEG, in a worker process | §3, §4.4 step 15, §4.5 step 4, §8.4 | HEIC, JPEG, PNG fixtures decode; preview is 1024 px on the long edge; compatible copy is full resolution with the colour profile kept; worker killed on a 10 s timeout | `cargo test -p image-worker` (macOS) | S4 | L | O | — |
| M0a-4 | `ImageCodec` on Windows: bundled libheif for HEIC, WIC for JPEG and PNG | same | Same tests pass on Windows without the HEVC codec installed | `cargo test -p image-worker` on Windows | M0a-3 | L | O | Runs on the GitHub Actions Windows runner (TR-12); needs setup items 5–6 |
| M0a-5 | Lossless location strip for JPEG, PNG, HEIC | §8.5, DU-20, DU-37 | Output has no GPS EXIF, XMP or IPTC location; pixels decode identical; orientation, ICC and gain map kept; a file with no location data comes out byte-identical | `cargo test -p image-worker strip` | S4 | L | O | Real-device fixtures from S4 |
| M0a-6 | `flutter_rust_bridge` bindings for a minimal client core (pin, upload, download) and a Flutter test app that uploads a picked photo | §3, §4.4, §5.7; TR-1 | Bindings generate and build for iOS simulator and Android emulator; the app uploads a file to a loopback harness host and the hash matches | `flutter test` (bridge tests on the build machine), `just mobile-it ios spike`, `just mobile-it android spike` | M0a-2, M0a-5 | L | S | — |
| M0a-7 | `locale_media` plugin spike: iOS `PHPickerViewController` with `preferredAssetRepresentationMode = .current`; Android system photo picker; both return a file path to Dart, never decoded bytes | §4.4 step 1; TR-1 | A HEIC in the simulator library arrives on the host with the same hash as the library file after the strip; the Android picker returns HEIC and Ultra HDR JPEG unchanged (emulator, API 28 and current) | `just plugin-test ios`, `just plugin-test android`, `just mobile-it ios picker` | M0a-6 | M | S | — |
| M0a-8 | mDNS advertise and browse on macOS and Windows | §5.6 | Two processes on one machine discover each other; TXT fields round-trip | `cargo test -p host-core mdns` | S1 | S | S | Windows result from the Actions runner (TR-12) |
| M0a-9 | Check `posthog-rs`: capture, flags, disk persistence; write findings | §11 rule 7 | A short findings note in the ledger: persists or not, flag evaluation offline | Verifier reads the note against the crate docs | S1 | S | H | — |
| M0a-10 | Exit check on real devices: iPhone and Android upload to a Mac and a Windows host over Wi-Fi; time a 12 MP transfer; compare hashes both ways | §1 SC1, SC4 | Results table filled in | Runbook `docs/agentic/runbooks/m0a.md` written by an agent | M0a-1…7 | S | H writes runbook | **Person runs it: two phones, Wi-Fi, Apple signing for device builds** |

---

## M0b. Phone host spike (parallel, does not block M1)

| ID | Goal | Digest | Accept | Verify | Deps | Size | Model | Gate |
|---|---|---|---|---|---|---|---|---|
| M0b-1 | Flutter test app embedding the host core, iOS: foreground hosting screen with keep-awake | §3, D10; TR-2 | Builds; in the simulator the app serves `/meta` to a harness client on the same machine | `just mobile-it ios hostspike` | M0a-1, M0a-6 | M | S | — |
| M0b-2 | Same app, Android: a native foreground service in a `locale_host_service` spike plugin owns the host core (TR-2) | same | On an emulator, the host keeps serving after the Flutter activity is destroyed, and the reopened UI reattaches to the running core | `just mobile-it android hostspike` | M0a-1, M0a-6 | M | S | — |
| M0b-3 | Measurement runbook and go / no-go template: 2-hour hosting, foreground and background behaviour, battery per hour, Local Network prompts | MVP §11, §12 | Runbook and template exist | Verifier checks the runbook covers each MVP §11 M0b measure | M0b-1, M0b-2 | S | H | **Person runs it on real phones and writes the go / no-go** |

---

## M1. Accounts and events

Exit (MVP §11): two phones and a browser join a laptop-hosted event and see the same page update live.

### App service and credentials

| ID | Goal | Digest | Accept | Verify | Deps | Size | Model | Gate |
|---|---|---|---|---|---|---|---|---|
| M1-1 | App service: accounts, sign-in codes (pluggable sender, console sender for development), credential issue and refresh, display-name change, host-link email | §4.1, §5.5, §6 | All five endpoints work; codes expire in 10 min and lock after 5 wrong tries; no endpoint accepts event or photo data (SC7) | `cargo test -p service` | S3 | M | S | Production deploy needs setup items 8–10 (domain, Postmark, Fly, signing key); development and tests do not |
| M1-2 | Credential library in `proto`: sign (service), verify offline (host), proof of possession, key list by `kid` | §5.5, §8.3 | Valid passes; expired, wrong key, tampered payload, stale or wrong-event proof all fail and are treated as guest | `cargo test -p proto credential` | S3 | M | O | — |

### Host core

| ID | Goal | Digest | Accept | Verify | Deps | Size | Model | Gate |
|---|---|---|---|---|---|---|---|---|
| M1-3 | SQLite schema and migrations for the event index and the host app database; event folder layout | §6, DU-34, DU-35 | Schema matches §6; migration from empty passes; index on `item(state, updated_at)` exists | `cargo test -p host-core schema` | S3 | M | S | — |
| M1-4 | Create event and quick session; `event_template`; one-active-event rule; QR URL and QR image | §4.2, §4.10 steps 3, §4.11, §5.1 | Defaults per §4.10 table; second create refused while one is active or paused; URL parses back to id, token, fingerprint | `cargo test -p host-core create` | M1-3, M0a-1 | M | S | — |
| M1-5 | Capability check and reachability test (network, port, space, power) behind an OS trait with fakes | §4.2 step 4, §4.10 step 2, §8.6 | Each failure yields its reason and fix id; reachability returns `ok`, `isolated` or `unknown` | `cargo test -p host-core capability` (fakes) | S1 | M | S | Real firewall and isolation behaviour is checked in M3-11 |
| M1-6 | `/meta` and `/join`: User Validation, Access Validation, roster, roles, sessions, refresh, re-join, removed list, rate limits | §4.3, §5.2, §5.3, §8.2 | Table-driven tests for every access type × identity × token/password case; password event never returns counts; guest joins as `viewer`; host account gets `host` | `cargo test -p host-core join` | M1-2, M1-4 | L | O | — |
| M1-7 | Change log and sync server over WebSocket: hello, snapshot, catch-up, live push, bye | §4.6, §5.4 | Client at version 0 gets a snapshot; at version n gets changes n+1…; every write commits state and change-log row in one transaction | `cargo test -p host-core sync` | M1-3, M1-6 | L | O | — |
| M1-8 | mDNS advertising with TXT fields for open and password events only | §5.6 | Invite events and quick sessions are never advertised | `cargo test -p host-core mdns_advert` | M0a-8, M1-4 | S | S | — |
| M1-9 | Posts: create, delete, system posts | §4.6, §5.3 | Only `host` role may post; change pushed; 500-character limit | `cargo test -p host-core posts` | M1-7 | S | S | — |
| M1-10 | Host runtime: start, stop, sleep prevention, resume after restart, IP-change handling, host panel counters | §4.2 step 10, §4.7 paused, §5.1, DU-17 | After a kill and restart the same QR code works and clients resync; counters match the index | `just harness host_restart` | M1-7 | M | S | — |

### Client core

| ID | Goal | Digest | Accept | Verify | Deps | Size | Model | Gate |
|---|---|---|---|---|---|---|---|---|
| M1-11 | Join client: URL parse, pinned TLS, trust on first use for Nearby, session refresh, silent re-join | §4.3, §8.1 | Wrong fingerprint is a hard failure; expired session re-joins and keeps the roster entry; removed client does not re-join | `just harness join_matrix` | M1-6 | M | O | — |
| M1-12 | Local index store and sync client, one store per event | §4.6, §6 | Applies only `local + 1`; drops duplicates; gap triggers resync; tombstones remove cached files; property test: any interleaving of drops and reconnects converges to the host's state | `cargo test -p client-core sync` | M1-7 | L | O | — |
| M1-13 | Multi-event registry and Your events list; mDNS browse | §4.11, §5.6 | Two joined events keep separate indexes; Nearby lists only open and password events | `cargo test -p client-core registry` | M1-12, M0a-8 | M | S | — |
| M1-14 | Account client: sign-in, credential storage and refresh at 2 days left, guest profile, device key | §4.1 | Refresh fires only when online and inside the window; expired offline → guest permissions | `cargo test -p client-core account` (fake clock) | M1-1, M1-2 | M | S | — |
| M1-15 | `flutter_rust_bridge` surface for the client core (`crates/mobile-bridge`): API of digest §5.7, observables as Dart streams | §5.7; TR-1 | Generated Dart package builds; Dart tests on the build machine join a loopback host, stream the index and enqueue an upload | `flutter test` in `packages/locale_core` | M1-11…14 | M | S | — |
| M1-16 | Multi-client harness: N simulated clients, fake clock, fault injection, scenario files | intro above | Scenarios `join_matrix`, `live_update_50` (50 clients see a post within 2 s on loopback), `host_restart` exist and pass | `just harness all` | M1-7, M1-12 | L | O | — |

### Telemetry

| ID | Goal | Digest | Accept | Verify | Deps | Size | Model | Gate |
|---|---|---|---|---|---|---|---|---|
| M1-17 | Telemetry schema, facade, allowlist filter, bucketing, ID hashing, consent, offline queue | §11 | A test feeds every forbidden property and proves none leaves the facade; unknown events dropped; nothing sent without consent | `cargo test -p proto telemetry` | S3, M0a-9 | M | S | PostHog key: setup item 13 (not needed for tests) |
| M1-18 | Flags and remote config: compiled defaults, bounds, freeze into the event record, deliver in `config` | §10, §11 | Out-of-range values ignored; an event keeps its frozen values after the host's flags change | `cargo test -p host-core config` | M1-4, M1-17 | S | S | — |

### Desktop app (Tauri)

| ID | Goal | Digest; screens | Accept | Verify | Deps | Size | Model | Gate |
|---|---|---|---|---|---|---|---|---|
| M1-19 | Shell, navigation, sign-in dialog, account menu, telemetry consent | §4.1, §9; D-01, section D | Works signed out as a viewer; sign-in against the local service | `pnpm -C apps/desktop e2e auth` | S5, M1-14 | M | S | — |
| M1-20 | Host page: quick session card, full form, capability checks, "You're hosting" state, quick session check page | §4.2, §4.10; D-04, D-05 | Password field and browsers rule per section D; Start disabled until checks pass | `pnpm -C apps/desktop e2e host-create` | M1-4, M1-5, M1-19 | L | S | — |
| M1-21 | Hosting window: QR (full screen, print), host code, counters, attendee table, new post, danger section (End wired in M3) | §4.2 steps 9–10; D-06, D-09 | Matches the mockup outline in both modes (event, quick) | `pnpm -C apps/desktop e2e host-live` | M1-10, M1-9 | L | S | — |
| M1-22 | Events page: Your events, Join with a link, Nearby, join dialog | §4.3; D-02, section D | Paste of a valid link joins; password and host-code paths work; error copy per section D | `pnpm -C apps/desktop e2e events` | M1-13, M1-19 | M | S | — |
| M1-23 | Joined event page: header, pinned post, tabs with Posts and Attendees, Photos tab shell | §4.6, §9; D-03, D-08 | A post made on the host appears without reload; verified badges per §9 | `pnpm -C apps/desktop e2e joined` | M1-12, M1-22 | M | S | — |

### Guest web page

| ID | Goal | Digest; screens | Accept | Verify | Deps | Size | Model | Gate |
|---|---|---|---|---|---|---|---|---|
| M1-24 | Join page (cover art, name, counts rule), session cookie, in-memory index over WebSocket, gallery shell with tabs, "Open in the app" link; assets embedded in the host binary | §4.3 browser guest, §4.6 step 7, §9, DU-11; W-01, W-02 | Works in Chromium, Firefox and WebKit with the network offline except the host; password event shows "needs the app"; no request leaves the LAN | `pnpm -C apps/web-guest e2e` (three engines) | M1-6, M1-7, S5 | L | S | — |

### Phone apps

| ID | Goal | Digest; screens | Accept | Verify | Deps | Size | Model | Gate |
|---|---|---|---|---|---|---|---|---|
| M1-25 | Flutter app shell: theme from S5 tokens, navigation, Welcome, sign up and in, guest name, consent, Account | §4.1, §9; P-01, P-09, section D | Widget tests for each screen and state; integration test signs in against the local service on the iOS simulator and the Android emulator | `flutter test`, `just mobile-it ios auth`, `just mobile-it android auth` | M1-15, S5 | M | S | — |
| M1-26 | Flutter Home, QR scanner, Join (cover art port, password, host code), host-on-computer sheet | §4.3, §9; P-02, P-04 | Golden tests: cover art matches the reference renderer for 5 event IDs; join through a pasted URL in an integration test on both platforms | `flutter test`, `just mobile-it ios join`, `just mobile-it android join` | M1-25, M1-28 | L | S | Camera and Local Network prompts need a device (M1-31) |
| M1-27 | Flutter Event page: header, banners, pinned post, Posts and Attendees tabs, Photos tab shell, host's-phone strip and post composer | §4.6, §4.11; P-03, P-05 | A live update from a harness host shows within 2 s on both platforms; widget tests for every banner state | `flutter test`, `just mobile-it ios event`, `just mobile-it android event` | M1-26 | L | S | — |
| M1-28 | `locale_net` plugin: iOS Local Network priming and denial detection; Android multicast lock while browsing Nearby | §4.3, §5.6; TR-1 | Native unit tests; denial maps to `local_network_denied`; Nearby on the emulator finds a harness host | `just plugin-test ios`, `just plugin-test android` | M1-15 | M | S | Real prompt behaviour checked in M1-31 |
| M1-31 | M1 exit demo on real devices: two phones and a browser on one Wi-Fi, laptop host; checklist of joins, posts, roster, host restart, plus a screenshot review of the derived screens from `screen-inventory.md` section D (Q8) | MVP §11 | Checklist written; results recorded | Runbook `runbooks/m1.md` | all M1 | S | H writes runbook | **Person runs it (about 30 minutes)** |

---

## M2. Photo pipeline

Exit (MVP §11): SC2, SC3, SC5, SC6, SC7 met.

| ID | Goal | Digest | Accept | Verify | Deps | Size | Model | Gate |
|---|---|---|---|---|---|---|---|---|
| M2-1 | Submit and admission: idempotent insert, duplicate rule, claims precheck, storage check, upload token | §4.4 steps 6–10, §5.3, §7 | Same submission ID twice returns the same result; concurrent duplicates create one row; viewer gets `role_denied` | `cargo test -p host-core admission` | M1-7 | M | O | — |
| M2-2 | Chunk endpoints with token binding, offsets, heartbeat, cancel, quarantine writes | §4.4 steps 11–13, §5.3, §8.2 | Wrong token → `unauthorized`; bytes beyond the claimed size rejected; nothing in quarantine is reachable by any GET | `cargo test -p host-core transfer` | M2-1, M0a-2 | M | O | — |
| M2-3 | Validation pipeline in the worker: integrity, magic bytes, brand, decode, host strip, facts, preview, move, commit transaction, ack, fan-out | §4.4 steps 14–18, §7, §8.4, §8.5 | Truncated, wrong-magic and decoder-crash fixtures end `invalid_image`; hash mismatch follows DU-13; crash between move and commit leaves no visible item (kill test) | `cargo test -p host-core validate` | M2-2, M0a-3, M0a-5 | L | O | Windows run on the Actions runner (TR-12) |
| M2-4 | State machine module with a transition table test: every legal row passes, every other pair is rejected | §7 | Test generated from the table in `proto`; 100% of state pairs covered | `cargo test -p host-core state_table` | M2-1 | S | S | — |
| M2-5 | Sweeper rules 1–3 with a fake clock | §4.8 | Stalled, abandoned and cleanup timings exact; a resubmit after `stalled` resumes | `cargo test -p host-core sweeper` | M2-2 | M | S | — |
| M2-6 | Download endpoints: preview, original, compatible on demand (single-flight), ranges, hash header | §4.5, §5.3 | 20 concurrent first requests for one compatible copy run one conversion; range resume yields an identical file; non-`AVAILABLE` item → `not_available` | `cargo test -p host-core download` | M2-3 | M | S | — |
| M2-7 | Download counts and hot score, batched `counts_updated`, event log rows without item ID | §4.12, DU-18, DU-19 | Score halves after 15 fake minutes; at most one batch per 30 s; log row has no target | `cargo test -p host-core counts` | M2-6 | S | S | — |
| M2-8 | Client outbox: durable record and file copy, precheck, strip, hash, consumer triggers, caps, user-visible states | §4.4 steps 2–5, §4.8 | Survives a process kill at any step; 10 attempts or 24 h → Expired; never a second item for one pick | `just harness outbox_faults` | M2-2, M1-12 | L | O | — |
| M2-9 | Client fetch and cache: preview and full image, version choice by device capability, hash check, saved list, cache purge on tombstone | §4.5, §6 | Hash mismatch retries once then errors; tombstone deletes cached files | `cargo test -p client-core cache` | M2-6 | M | S | — |
| M2-10 | Host upload over loopback (copy, never move) | §4.4 host uploads | Source file untouched after event end | `cargo test -p host-core host_upload` | M2-3 | S | S | — |
| M2-11 | Desktop: Add photos (drop, picker), upload tiles, photo panel with Save as and folder, Download all originals | §4.3 desktop, §4.5; D-03, D-06, section D | Default version by OS; Download all resumes and skips saved files | `pnpm -C apps/desktop e2e photos` | M2-8, M2-9, M1-23 | L | S | — |
| M2-12 | Shared gallery logic in the client core: people filter, four sorts, 30-minute grouping, Show filters | §4.12 | Golden tests for each sort and filter on a 300-item index | `cargo test -p client-core gallery` | M1-12, M2-7 | M | S | — |
| M2-13 | Desktop gallery UI: people pills, sort menu, mixed grid | §4.12, §9; D-03 | Matches the mockup outline; 1,000 items scroll without dropped frames in the test profile | `pnpm -C apps/desktop e2e gallery` | M2-12 | M | S | — |
| M2-14 | Flutter: picker through `locale_media`, Uploads screen, upload tiles | §4.4, §4.8; P-06, P-08 | Integration test picks a HEIC on the iOS simulator and a JPEG on the emulator; each becomes `AVAILABLE` on a harness host with the expected hash | `just mobile-it ios upload`, `just mobile-it android upload` | M2-8, M1-27, M2-16 | L | S | — |
| M2-15 | Flutter: gallery, viewer, Save (Original or Compatible), Delete entry point | §4.5, §4.12; P-05, P-07 | Saved file hash equals the item hash for Original on both platforms; HEIC original displays on API 28+ | `flutter test`, `just mobile-it ios gallery`, `just mobile-it android gallery` | M2-9, M2-12, M2-16 | L | S | Save permission prompt on a device: M2-20 |
| M2-16 | `locale_media` production: add-only Save to Photos (iOS) and MediaStore save (Android), cancellation, large files, limited-library access, error mapping | §4.4 step 1, §4.5 step 8; TR-1 | Native unit tests; saving never re-encodes (hash check); denial maps to a typed error | `just plugin-test ios`, `just plugin-test android` | M0a-7 | M | S | — |
| M2-18 | Guest web: gallery grid, people row, sort menu, photo view with Original or Compatible download | §4.5; W-02, W-03 | Chromium and Firefox show the compatible copy of a HEIC; WebKit shows the original; no write endpoint is reachable (SC3) | `pnpm -C apps/web-guest e2e photos` | M2-6, M1-24 | M | S | — |
| M2-19 | Pipeline telemetry events and error tagging | §11 | Events carry only schema properties; reason codes from §5.2 | `cargo test -p proto telemetry_pipeline` | M1-17, M2-3 | S | S | — |
| M2-20 | Success-criteria suite in the harness: SC2 (HEIC viewable and both downloads), SC3, SC5 (push < 2 s p95, 50 clients), SC6 (drop and kill), SC7 (service code has no event or media route; network capture during a run shows no service traffic) | §1 | All five pass on loopback | `just harness success_criteria` | all M2 | M | O | **Person repeats SC2, SC5, SC6 on real phones over Wi-Fi and reviews screenshots of the photo screens (about 1 hour)** |

---

## M3. Lifecycle and hardening

Exit (MVP §11): SC8 met.

| ID | Goal | Digest | Accept | Verify | Deps | Size | Model | Gate |
|---|---|---|---|---|---|---|---|---|
| M3-1 | Delete photo and post: role checks, tombstone, file removal, log; UI entry points on all clients | §4.7 | Uploader and host may delete; others get `role_denied`; cached copies gone on every client | `just harness delete` | M2 | M | S | — |
| M3-2 | Remove attendee with optional delete of their uploads; desktop dialog | §4.7; section D | Session revoked, socket closed, re-join refused | `just harness remove` | M2 | M | S | — |
| M3-3 | Edit details; extend and shorten with the 7-day cap; system post; client cache-expiry update; desktop dialogs | §4.7, §4.9 | Cap enforced; paused event refused; shorten into the past ends the event; all clients show the notice | `just harness extend` | M2 | M | S | — |
| M3-4 | End and expire: push, client purge, host deletes folder and index, stop server; sweeper rules 4–5; "expiring soon" notice | §4.7, §4.8 | After end no event file remains on the host; each client's cache is empty; saved files remain (SC8) | `just harness end_expire` | M3-3 | L | O | — |
| M3-5 | Paused events and offline expiry on clients; host-offline UI on all clients | §4.7 paused | A client offline at expiry deletes its cache on time by its own clock; reconnect after extension resyncs | `just harness paused_expiry` | M3-4 | M | O | — |
| M3-6 | Quick-session limits: 12 h auto-end, Keep going, 24 h hard limit, Add details conversion with the same QR | §4.10 steps 5–8 | Converted session keeps event ID and token; not saved to Past events unless converted | `cargo test -p host-core quick` | M3-4 | M | S | — |
| M3-7 | Past events and Host again | §4.9 reopen; D-07 | New event has new ID, token and key; old QR fails with `event_ended` or unreachable | `pnpm -C apps/desktop e2e past` | M3-4 | S | S | — |
| M3-8 | Event log screen and export; host counters for failures in the last hour | §10; D-10 | Export contains joins, denials, uploads, downloads without item IDs, deletes, removals | `pnpm -C apps/desktop e2e log` | M2-7 | S | S | — |
| M3-9 | Rate-limit and abuse pass: uploads, joins, password attempts, oversized bodies, slow clients | §5.3, §8 | Each limit has a test; a 51st client is handled without affecting the first 50 | `just harness abuse` | M2 | M | S | — |
| M3-10 | Worker sandbox hardening: macOS sandbox profile, Windows job object and restricted token | §8.4 | Worker cannot open a network socket or read outside its input (tests attempt both) | `cargo test -p image-worker sandbox` | M2-3 | M | O | Windows run on the Actions runner (TR-12) |
| M3-11 | Firewall and hotspot help: Windows firewall rule prompt and Mobile Hotspot control, macOS prompt guidance, real reachability test | §4.2 steps 4–5, §4.10 step 2 | Code paths behind the OS trait have unit tests with fakes; manual checklist written | `cargo test -p host-core os_help` | M1-5 | L | S | **Person checks on real macOS and Windows, including an isolated guest Wi-Fi** |
| M3-12 | Error tracking with path scrubbing and stage tags; surveys | §11 | Scrubber test: no event name, file name or home path survives | `cargo test -p proto scrub` | M1-17 | S | S | — |
| M3-13 | Load test at the targets: 50 clients, 1,000 photos, join with a 500-item index | §10 | Join < 5 s, push < 2 s p95 on loopback; memory and file-handle use recorded | `just harness load` | M2-20 | M | S | Real-network numbers come from M4 |
| M3-14 | Security review of pinned TLS, tokens, credential checks, D17 leakage, plain-HTTP write rejection | §8 | Written findings, each fixed or accepted with a reason | Verifier re-runs the attack tests listed in the review | M3-1…9 | M | O | **Person reads the findings** |
| M3-15 | Accessibility and copy pass on all surfaces; role labels through one string table | §9 | Every control has a label; contrast holds in both themes | Automated accessibility checks in each UI suite | M2 | M | S | — (role labels decided, Q7) |
| M3-16 | Packaging: macOS app bundle and Windows installer with the worker and libheif; iOS and Android release builds | §3 | Unsigned builds install and run locally | `just package` | M3 | M | S | **Person supplies signing identities, notarises, and final identifiers (setup items 12, 14); Windows builds internal-only until licensing advice (TR-15)** |
| M3-17 | SC8 suite and M3 exit checklist | §1 | SC8 passes in the harness; device checklist written | `just harness sc8` | M3-4, M3-5 | S | S | **Person repeats SC8 on two phones** |

---

## M4. Field test

| ID | Goal | Digest | Accept | Verify | Deps | Size | Model | Gate |
|---|---|---|---|---|---|---|---|---|
| M4-1 | Field-test runbook, pre-flight checklist and a log-triage script that turns an exported event log into a ranked failure list | §10, §11 | Script output on a sample log is correct | `just triage sample.jsonl` | M3 | S | H | — |
| M4-2 | PostHog dashboards for the success criteria and setup friction, as saved definitions | §11; PH §6 | Definitions exist for each PH §6 insight | Verifier compares against PH §6 | M1-17 | S | H | **Person creates the PostHog EU project and imports them (setup item 13)** |
| M4-3 | Run the event (15–30 people) and fix the top issues | MVP §11 | Top failures fixed and re-verified | Per-fix tests | M4-1 | L | S and O | **Person runs the event; agents fix from the exported log** |

---

## M5. Phone hosting (plan only until M0b's go / no-go)

| ID | Goal | Digest | Accept | Verify | Deps | Size | Model | Gate |
|---|---|---|---|---|---|---|---|---|
| M5-1 | Host core in the Flutter app with `locale_host_service`: Android foreground service owns the core; iOS foreground hosting mode with keep-awake (TR-2) | D10, §4.2; TR-2 | A phone-hosted event on a simulator and an emulator passes `join_matrix`; Android keeps serving with the UI destroyed | `just mobile-it ios host`, `just mobile-it android host` | M0b-3 go | L | O | — |
| M5-2 | Phone capability check: battery, storage allocation, hotspot guidance | MVP §13 | Checks behind the OS trait with fakes | unit tests | M5-1 | M | S | — |
| M5-3 | Phone Host an event and Quick session flows; warning when joining another event while hosting | §4.10, MVP §5.11 steps 10–11 | UI tests for create, live window, end | simulator UI tests | M5-1 | L | S | — |
| M5-4 | `phone_hosting_enabled` flag and M5 telemetry events | PH §11 M5 | Schema tests | `cargo test -p proto telemetry` | M5-1 | S | S | — |
| M5-5 | Repeat the M4 test with a phone host; battery under 15% per hour | MVP §9, §11 | Results recorded | Runbook | M5-1…4 | L | — | **Person runs it** |

---

## Task counts

| Milestone | Tasks | Tasks with a human gate | Can the milestone run unattended? |
|---|---|---|---|
| S | 6 | 3 (tool install, device photos, CI access) | Yes after the three inputs |
| M0a | 10 | 2 (Windows, device exit check) | Build yes; exit check no |
| M0b | 3 | 1 | Build yes; the measurement is the point and is human |
| M1 | 29 | 1 exit demo (plus prompts noted) | Yes, up to the exit demo |
| M2 | 19 | 1 exit repeat on devices | Yes, up to the device repeat |
| M3 | 17 | 4 | Mostly; OS help, signing and sign-offs are human |
| M4 | 3 | 2 | No |
| M5 | 5 | 1 (and depends on M0b) | Build mostly; verification no |
| **Total** | **92** | | |
