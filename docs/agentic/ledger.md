# Locale MVP build ledger

This file is the record of the build run. It overrides anyone's memory of the conversation. Statuses: todo, ready, in-progress, verifying, done, failed, blocked: <reason>, gate: <what the person must do>. Details for each task live in `docs/agentic/tasks/<ID>.md`.

## 1. Tasks

| ID | Status | Branch | Model | Attempts | Verified | Note |
|---|---|---|---|---|---|---|
| S1 | done | merged 9fab878 | S | 1 | PASS (Sonnet), `just check` green on build/integration | Skeleton, justfile, CLAUDE.md. Tauri crate is `locale-desktop`; TypeScript pinned to 6.0.3 |
| S2 | done | merged 1d45dab | H | 1 | PASS (Sonnet, low), `just check` green on build/integration | `just doctor` exits 0 on this machine: nothing needed for M1 is missing, so the human gate needs no action. Not needed and missing: cargo-ndk, Playwright browsers, libheif. Windows branches of the script are unrun until S6 |
| S3 | done | merged 2d1cb7b | O | 1 | PASS (Sonnet, medium), `just check` green on build/integration | `proto` crate: transport, codes, api, ws, service, mdns, model, state, config; 24 schema files and `CONTRACT.md` generated, with a staleness test. 61 tests. No returns. Contract choices recorded in digest §5.8 (DU-49) |
| S4 | gate: person adds four real device photos (setup items 2 and 3; runbook `runbooks/S4.md`) | merged 68029db | S (was H) | 1 | PASS (Sonnet, low), `just check` and `just fixtures-verify` green on build/integration | Agent work is done and merged: 20 committed fixtures, two 50 MB files generated on demand, manifest, independent verifier. Tasks that depend on S4 may start; only the device-photo parts of M0a-5 and M2-3 wait for the person. One return (bytecode) |
| S5 | done | merged 4afeea8 | S | 1 | PASS (Sonnet, medium), `just check` green on build/integration | `packages/ui` (`@locale/ui`), `tokens.json` and `labels.json` as single sources, generated CSS, TS and Dart. All contrast pairs at least 4.5:1 |
| S6 | done | merged ba7963b | S | 1 | PASS (Sonnet, low), `just check` green on build/integration | `ci.yml` (rust on macOS and Windows, web with three browser engines, mobile, scripts), `just ci <suffix>` (plain push, never forced), runbook `runbooks/ci.md`. Proof: smoke run 43838217560 all green; red run 43838700494 fails only the Windows Rust job. Found and fixed on the way: missing Windows icon (S1), a doctor timeout that did not kill the process tree on Windows (S2), fixtures altered by line-ending conversion. The human gate needed no action (Actions is enabled). Third-party actions are pinned by major tag, not commit SHA: a hardening item for M3-14. The remote branch `ci/s6-red` holds the deliberately failing test; only a person can delete `ci/*` branches |
| M0a-1 | done | merged 71034d4 | O | 1 | PASS (Sonnet, medium), `just check` green on build/integration | Listener with separate TLS and plain routers, `TlsIdentity`, pinned connector in client-core. Follow-ups: move `fingerprint_base64url` to proto's helper after S3; plain path has no header-read timeout yet (M3-9); two timing tests have a 500 ms margin |
| M0a-2 | in-progress | worktree | S | 1 | — |  |
| M0a-3 | done | merged a27323c | O | 1 | PASS (Sonnet, medium), `just check` green on build/integration | `image-worker`: pure-Rust `sniff`, `ImageCodec` trait with ImageIO back end, one-job-per-process worker with a clean environment, 10 s kill and reaping. Known limits, accepted: on macOS the 1 GB memory limit is a parent-side poll every 10 ms, not a kernel limit; ImageIO decodes truncated JPEG and PNG without error, so a structure check runs first; the kill tests run only in debug builds |
| M0a-4 | todo | — | O | 0 | — | Has a human gate |
| M0a-5 | verifying | worktree-agent-aab711a36bb24e185 | O | 1 | — | Orchestrator ran `cargo test -p image-worker strip` (56 pass) and `just check` (exit 0). `miniz_oxide` accepted as a dependency (compressed PNG text chunks). Has a human gate (device photos from S4) |
| M0a-6 | todo | — | S | 0 | — |  |
| M0a-7 | todo | — | S | 0 | — |  |
| M0a-8 | in-progress | worktree | S | 1 | — | Has a human gate (Windows result from CI) |
| M0a-9 | done | merged 69b02ce | H | 1 | PASS on the third check (Haiku), after two returns for citations | Findings: `posthog-rs` 0.33.1 does not persist unsent events (memory queue only); flags need a network call and no cache survives a restart, so offline they must fall back to compiled defaults; the crate forces aws-lc-rs and sets person profiles on when a group is added. Decision TR-16 |
| M0a-10 | todo | — | H writes runbook | 0 | — | Has a human gate |
| M0b-1 | todo | — | S | 0 | — |  |
| M0b-2 | todo | — | S | 0 | — |  |
| M0b-3 | todo | — | H | 0 | — | Has a human gate |
| M1-1 | todo | — | S | 0 | — | Has a human gate |
| M1-2 | verifying | worktree-agent-a150b31ae610548f4 | O | 1 | — | Orchestrator ran `cargo test -p proto` and `just check` (exit 0). One return with an approved contract change: `/join` carries the raw credential and proof so a malformed one means guest, not a rejected join |
| M1-3 | verifying | worktree-agent-a7122ab5e057dfa23 | S | 1 | — | Orchestrator ran `cargo test -p host-core schema` (26 pass) and `just check` (exit 0) |
| M1-4 | todo | — | S | 0 | — |  |
| M1-5 | done | merged 18e496e | S | 1 | PASS (Sonnet, medium), `just check` green on build/integration | `host-core::capability`: `OsProbe` trait, `FakeProbe` behind the `test-util` feature, check rows with reason and fix, reachability `ok`/`isolated`/`unknown`. The real probe answers only free space, port binding and private address; firewall, power and mDNS reply are left for M3-11, which also holds the manual check on real machines |
| M1-6 | todo | — | O | 0 | — |  |
| M1-7 | todo | — | O | 0 | — |  |
| M1-8 | todo | — | S | 0 | — |  |
| M1-9 | todo | — | S | 0 | — |  |
| M1-10 | todo | — | S | 0 | — |  |
| M1-11 | todo | — | O | 0 | — |  |
| M1-12 | todo | — | O | 0 | — |  |
| M1-13 | todo | — | S | 0 | — |  |
| M1-14 | todo | — | S | 0 | — |  |
| M1-15 | todo | — | S | 0 | — |  |
| M1-16 | todo | — | O | 0 | — |  |
| M1-17a | ready | — | S | 0 | — | Split from M1-17: telemetry schema, buckets, ID hashing, consent type and the allowlist filter in `proto` (pure). Verify: `cargo test -p proto telemetry` |
| M1-17b | todo | — | S | 0 | — | Split from M1-17: new `crates/telemetry` facade: consent storage, SQLite offline queue, own HTTP sender (TR-16) with a loopback fake, hand-off sink for the phone SDK. Depends on M1-17a. Verify: `cargo test -p telemetry`. PostHog key (setup item 13) is not needed for tests |
| M1-18 | todo | — | S | 0 | — |  |
| M1-19 | todo | — | S | 0 | — |  |
| M1-20 | todo | — | S | 0 | — |  |
| M1-21 | todo | — | S | 0 | — |  |
| M1-22 | todo | — | S | 0 | — |  |
| M1-23 | todo | — | S | 0 | — |  |
| M1-24 | todo | — | S | 0 | — |  |
| M1-25 | todo | — | S | 0 | — |  |
| M1-26 | todo | — | S | 0 | — | Has a human gate |
| M1-27 | todo | — | S | 0 | — |  |
| M1-28 | todo | — | S | 0 | — | Has a human gate |
| M1-31 | todo | — | H writes runbook | 0 | — | Has a human gate |
| M2-1 | todo | — | O | 0 | — |  |
| M2-2 | todo | — | O | 0 | — |  |
| M2-3 | todo | — | O | 0 | — | Has a human gate |
| M2-4 | todo | — | S | 0 | — |  |
| M2-5 | todo | — | S | 0 | — |  |
| M2-6 | todo | — | S | 0 | — |  |
| M2-7 | todo | — | S | 0 | — |  |
| M2-8 | todo | — | O | 0 | — |  |
| M2-9 | todo | — | S | 0 | — |  |
| M2-10 | todo | — | S | 0 | — |  |
| M2-11 | todo | — | S | 0 | — |  |
| M2-12 | todo | — | S | 0 | — |  |
| M2-13 | todo | — | S | 0 | — |  |
| M2-14 | todo | — | S | 0 | — |  |
| M2-15 | todo | — | S | 0 | — | Has a human gate |
| M2-16 | todo | — | S | 0 | — |  |
| M2-18 | todo | — | S | 0 | — |  |
| M2-19 | todo | — | S | 0 | — |  |
| M2-20 | todo | — | O | 0 | — | Has a human gate |
| M3-1 | todo | — | S | 0 | — |  |
| M3-2 | todo | — | S | 0 | — |  |
| M3-3 | todo | — | S | 0 | — |  |
| M3-4 | todo | — | O | 0 | — |  |
| M3-5 | todo | — | O | 0 | — |  |
| M3-6 | todo | — | S | 0 | — |  |
| M3-7 | todo | — | S | 0 | — |  |
| M3-8 | todo | — | S | 0 | — |  |
| M3-9 | todo | — | S | 0 | — |  |
| M3-10 | todo | — | O | 0 | — | Has a human gate |
| M3-11 | todo | — | S | 0 | — | Has a human gate |
| M3-12 | todo | — | S | 0 | — |  |
| M3-13 | todo | — | S | 0 | — | Has a human gate |
| M3-14 | todo | — | O | 0 | — | Has a human gate |
| M3-15 | todo | — | S | 0 | — |  |
| M3-16 | todo | — | S | 0 | — | Has a human gate |
| M3-17 | todo | — | S | 0 | — | Has a human gate |
| M4-1 | todo | — | H | 0 | — |  |
| M4-2 | todo | — | H | 0 | — | Has a human gate |
| M4-3 | todo | — | S and O | 0 | — | Has a human gate |
| M5-1 | todo | — | O | 0 | — |  |
| M5-2 | todo | — | S | 0 | — |  |
| M5-3 | todo | — | S | 0 | — |  |
| M5-4 | todo | — | S | 0 | — |  |
| M5-5 | todo | — | — | 0 | — | Has a human gate |

## 2. Defaults and decisions made during the build

- DU-43 (default, unconfirmed) · Sizes use binary units: the 50 MB photo limit is 52,428,800 bytes and the 1 MB chunk is 1,048,576 bytes. The digest says "50 MB" and "1024 KB" without defining the unit.
- DU-44 (default, unconfirmed) · TLS uses rustls with the `ring` crypto provider, not aws-lc, so the phone and Windows builds need no extra native build tools.
- DU-52 (default, unconfirmed) · `just check-rust` (and so `just check` and the CI Rust job) runs `fixtures-verify` first, because tests read the generated 50 MB fixtures. Image-worker defaults (M0a-3): every job carries the uploader's claimed format and a mismatch with the sniffed format fails; the compatible copy keeps the orientation tag and colour profile only; size buckets are in MiB.
- DU-51 (default, unconfirmed) · A crate `crates/telemetry` is added to the DU-7 layout for the telemetry facade (consent storage, offline queue, sender), so `crates/proto` holds only the schema and stays free of I/O.
- DU-50 (default, unconfirmed) · Capability check details (M1-5): reasons `no_network`, `firewall_blocked`, `low_space`, `low_power`; fixes `turn_on_hotspot`, `show_network_instructions`, `allow_firewall`, `choose_smaller_size`, `plug_in`; link-local and 100.64.0.0/10 addresses count as isolated; the reachability test is capped at 5 s overall with the 2 s mDNS wait inside.
- DU-49 (contract, applied to the digest at once) · Contract details fixed in code by S3: unpadded base64url; uppercase item states; join-failure HTTP statuses; 204 for DELETE and saved; cookie `locale_session`; snapshot items carry counts; credential signature over `v1.<payload segment>`; 70-byte proof message with a big-endian millisecond timestamp; Bearer refresh token on the two authenticated service endpoints; config JSON layout and bounds; which enums accept unknown values. `schemars` stays a normal dependency of `proto`. Listed in digest §5.8.
- TR-16 (proposed) · The desktop core sends telemetry with its own small sender over PostHog's HTTP API; the facade owns the SQLite queue and the flag cache; `posthog-rs` is not used. Reason: the crate does not persist, has no offline flag cache, and forces aws-lc-rs into the build. Entry added to `docs/Locale Design Tradeoffs.md`.
- DU-48 (default, unconfirmed) · Digest §11 rule 7 becomes: "On desktop the facade owns a small SQLite queue and sends over PostHog's HTTP API itself (TR-16)."
- DU-46 (default, unconfirmed) · Dark-theme values the digest does not give (S5): selected tab `#A5B4FC`; badge tint `#312E81` with ink `#E0E7FF`; status pairs are the light pairs swapped (ok `#14532D`/`#DCFCE7`, warn `#78350F`/`#FEF3C7`, error `#912018`/`#FEE4E2`). Spacing, radius and type scales and the extra label "Verified" are also S5 defaults, in `packages/ui/tokens.json` and `labels.json`.
- DU-47 (default, unconfirmed) · Listener defaults (M0a-1): 20 port attempts after 7420, 10 s first-byte and TLS handshake timeouts, 5 s shutdown grace, HTTP/1.1 and h2 without ALPN, key and certificate stored as `tls/key.der` (mode 0600) and `tls/cert.der`; the pinned client does not check certificate dates (the pin is the trust decision).
- DU-45 (default, unconfirmed) · Three folders are added to the DU-7 layout: `scripts/` (developer scripts: doctor, fixture generator and verifier), `fixtures/` (test images and manifest; small files committed, two 50 MB files generated on demand), `packages/ui` (shared web UI kit and design tokens, pnpm package `@locale/ui`).

## 3. Pending digest edits

None. DU-44 to DU-52 were applied to the digest and `CLAUDE.md` at the milestone S boundary (2026-11-16).

## 4. Plan changes

- 2026-11-16 · S4 routed to Sonnet instead of Haiku. The fixture set as briefed needs hand-built EXIF, XMP and IPTC location blocks, a HEIC with GPS written through ImageIO, deterministic 50 MB files and an independent metadata check, which is more than boilerplate. The verifier stays Sonnet.
- 2026-11-16 · M1-17 split into M1-17a and M1-17b. The plan put the whole telemetry facade in `proto`, but `proto` must stay free of I/O (it is the contract and is compiled into the phone app), and the offline queue and sender need SQLite and HTTP. M1-17a is the pure schema and filter in `proto`; M1-17b is a new crate `crates/telemetry` used by both cores. Tasks that depended on M1-17 depend on M1-17a (M1-18, M2-19, M3-12, M4-2) or on both (desktop consent UI in M1-19).
- 2026-11-16 · M0a-1 also delivers the pinned-TLS connector in `crates/client-core/src/tls_pin.rs`, because its acceptance needs a pinned client and M1-11 and M0a-2 reuse it. The fingerprint's text forms stay in `proto` (S3).

## 5. Milestone log

- 2026-11-14T02:22Z · S · Run started by a person. Ledger created from the work breakdown (92 tasks).
- 2026-11-14T02:22Z · S · Tool probe before S1: present are Node 26.7, npm, Flutter 3.47.7 (Dart 3.13.5), Python 3.12, sips, JDK 17, Homebrew. Missing are rustup, cargo, rustc, just, pnpm, full Xcode (only Command Line Tools, so no xcodebuild or simctl), the Android SDK (adb, emulator), flutter_rust_bridge_codegen and gh.
- 2026-11-14T02:22Z · S · Stopped, waiting for a person. S1 cannot be verified without cargo and just, and every other task depends on S1. No subagents were launched. See `docs/agentic/reports/S.md`.
- 2026-11-16T04:18Z · S · Resumed at a person's request. Tool probe: every tool that was missing is now on PATH (cargo 1.100.0, just 1.59.0, pnpm 12.18.0, full Xcode with iOS 27.2 simulators, adb and emulator, flutter_rust_bridge_codegen, gh signed in). Rust targets for iOS and Android are installed. No leftover worktrees or task branches. S1 moved from blocked to ready.
- 2026-11-16T05:20Z · S · S1 done: 1 Sonnet implementer, 1 Sonnet verifier, no returns. Dispatched S2 (Haiku), S3 (Opus), S4 (Sonnet) and M0a-1 (Opus) in parallel worktrees.

- 2026-11-16T06:05Z · S · S2 done: 1 Haiku implementer, 1 Sonnet verifier, no returns. S4 returned once (bytecode committed). S5 dispatched (Sonnet).
- 2026-11-16T07:00Z · S, M0a · Done: S4 (agent part; now at its human gate), S5, M0a-1, each with one implementer and one verifier and no verifier returns. S6 and M0a-9 returned once each. Verifiers that run outside a task worktree cannot use the Edit tool under `.claude/worktrees/` (the `.claude/**` deny rule); from now on verifiers get their own worktree and check out the task branch detached. Dispatched M1-5 (Sonnet).
- 2026-11-16T07:35Z · M0a · M0a-9 done: 1 Haiku implementer, 1 Haiku verifier, two returns for citations (a research note with source citations is a poor fit for Haiku at low effort; route the next one to Sonnet). Docs-only merge, so `just check` was not re-run for it. Dispatched M0a-3 (Opus).

- 2026-11-16T09:11Z · S, M0a · The session reached its usage limit. The three running implementers (S6, M1-5, M0a-3) were stopped by the orchestrator, then resumed in the same session with their context and worktrees intact when the limit reset; nothing was lost. S3's implementer finished; its checks pass and it is with a Sonnet verifier that works in its own worktree on a detached checkout of the task branch. Note: timestamps earlier in this log were estimates and run up to two hours ahead of the real clock; from here they come from `date -u`.

- 2026-11-16T09:17Z · S, M1 · M1-5 implementer done, with verifier. Orchestrator fixed one test's output in its own commit (7324433): a passing fixture test printed a "FAIL" line into `just check` output and two agents read it as a failure.
- 2026-11-16T09:21Z · S · S3 done: 1 Opus implementer, 1 Sonnet verifier, no returns. Digest §5.8 added with the contract details in commit 3859f12 (the one digest edit allowed mid-milestone); DU-43 moved into the digest with it. No `done` task used `proto` types before this merge, so no tests needed re-running beyond `just check`. Dispatched M1-2 (Opus). Follow-up for a later host task: replace M0a-1's private `fingerprint_base64url` helper with `proto::transport::Fingerprint`.
- 2026-11-16T09:23Z · M1 · M1-5 done: 1 Sonnet implementer, 1 Sonnet verifier, no returns. Workspace dependency-table conflicts at merge are now resolved by a small script that keeps both sides sorted and stops if the same crate is specified two ways.

- 2026-11-16T09:33Z · S, M0a, M1 · S6 and M0a-3 implementers done, each with a Sonnet verifier. Dispatched M1-3 and M0a-2 (Sonnet). Running implementers: M0a-5, M1-2, M1-3, M0a-2.
- 2026-11-16T14:11Z · S, M0a, M1 · Environment failure: all six running subagents (implementers for M0a-5, M0a-2, M1-2, M1-3; verifiers for S6 and M0a-3) ended with "You've hit your 5-hour limit" (HTTP 429) from the model API. That meets the circuit-breaker rule of three environment failures in a row. No work was lost: each agent's worktree holds its partial work, and M1-2 and S6 have their commits. The limit then reset and the session was continued. Because this session was started at a person's request, the run is resumed under the "Resuming" steps: the cause is checked first with one cheap resume (the S6 verifier); if that fails again the run pauses at once with this reason.
- 2026-11-16T14:24Z · S, M0a · The probe succeeded, so all agents were resumed from their saved work; `consecutive_env_failures` reset. S6 done (1 Sonnet implementer, 1 Sonnet verifier, three pre-verification returns, 5 CI runs) and M0a-3 done (1 Opus implementer, 1 Sonnet verifier, no returns). Milestone S now has every task done except S4, which waits at its human gate. M1-2, M1-3 and M0a-5 are with verifiers; M0a-2 and M0a-8 are being implemented. A CI run of the merged branch was started with `just ci`.
- 2026-11-16T14:25Z · S · Milestone S boundary: every S task is done except S4, which is at its human gate. Queued digest edits DU-44 to DU-52 and the `CLAUDE.md` changes were applied in one commit (6a7732c). Report written: `docs/agentic/reports/S.md`. Totals for S: 6 implementers (1 Opus, 4 Sonnet, 1 Haiku), 6 Sonnet verifiers, 5 CI runs. The run continues with M0a and M1.
