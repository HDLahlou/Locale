# Locale MVP build ledger

This file is the record of the build run. It overrides anyone's memory of the conversation. Statuses: todo, ready, in-progress, verifying, done, failed, blocked: <reason>, gate: <what the person must do>. Details for each task live in `docs/agentic/tasks/<ID>.md`.

## 1. Tasks

| ID | Status | Branch | Model | Attempts | Verified | Note |
|---|---|---|---|---|---|---|
| S1 | done | merged 9fab878 | S | 1 | PASS (Sonnet), `just check` green on build/integration | Skeleton, justfile, CLAUDE.md. Tauri crate is `locale-desktop`; TypeScript pinned to 6.0.3 |
| S2 | done | merged 1d45dab | H | 1 | PASS (Sonnet, low), `just check` green on build/integration | `just doctor` exits 0 on this machine: nothing needed for M1 is missing, so the human gate needs no action. Not needed and missing: cargo-ndk, Playwright browsers, libheif. Windows branches of the script are unrun until S6 |
| S3 | verifying | worktree-agent-afd7cbc871f0e3440 | O | 1 | — | Orchestrator ran `cargo test -p proto` (61 pass) and `just check` (pass). 26 implementer defaults in the task file; decisions on them are recorded when the verifier reports |
| S4 | gate: person adds four real device photos (setup items 2 and 3; runbook `runbooks/S4.md`) | merged 68029db | S (was H) | 1 | PASS (Sonnet, low), `just check` and `just fixtures-verify` green on build/integration | Agent work is done and merged: 20 committed fixtures, two 50 MB files generated on demand, manifest, independent verifier. Tasks that depend on S4 may start; only the device-photo parts of M0a-5 and M2-3 wait for the person. One return (bytecode) |
| S5 | done | merged 4afeea8 | S | 1 | PASS (Sonnet, medium), `just check` green on build/integration | `packages/ui` (`@locale/ui`), `tokens.json` and `labels.json` as single sources, generated CSS, TS and Dart. All contrast pairs at least 4.5:1 |
| S6 | in-progress | worktree-agent-a1b93593b0b50aaab | S | 1 | — | Two CI runs so far (43805881453, 43806612675), both failed on Windows; icon and doctor-test fixes are committed; implementer is reading the second failure. Has a human gate (Actions is enabled: runs start) |
| M0a-1 | done | merged 71034d4 | O | 1 | PASS (Sonnet, medium), `just check` green on build/integration | Listener with separate TLS and plain routers, `TlsIdentity`, pinned connector in client-core. Follow-ups: move `fingerprint_base64url` to proto's helper after S3; plain path has no header-read timeout yet (M3-9); two timing tests have a 500 ms margin |
| M0a-2 | todo | — | S | 0 | — |  |
| M0a-3 | in-progress | worktree | O | 1 | — |  |
| M0a-4 | todo | — | O | 0 | — | Has a human gate |
| M0a-5 | in-progress | worktree | O | 1 | — | Runs alongside M0a-3 in the same crate (different modules). Has a human gate (device photos from S4) |
| M0a-6 | todo | — | S | 0 | — |  |
| M0a-7 | todo | — | S | 0 | — |  |
| M0a-8 | todo | — | S | 0 | — | Has a human gate |
| M0a-9 | done | merged 69b02ce | H | 1 | PASS on the third check (Haiku), after two returns for citations | Findings: `posthog-rs` 0.33.1 does not persist unsent events (memory queue only); flags need a network call and no cache survives a restart, so offline they must fall back to compiled defaults; the crate forces aws-lc-rs and sets person profiles on when a group is added. Decision TR-16 |
| M0a-10 | todo | — | H writes runbook | 0 | — | Has a human gate |
| M0b-1 | todo | — | S | 0 | — |  |
| M0b-2 | todo | — | S | 0 | — |  |
| M0b-3 | todo | — | H | 0 | — | Has a human gate |
| M1-1 | todo | — | S | 0 | — | Has a human gate |
| M1-2 | todo | — | O | 0 | — |  |
| M1-3 | todo | — | S | 0 | — |  |
| M1-4 | todo | — | S | 0 | — |  |
| M1-5 | in-progress | worktree | S | 1 | — | Has a human gate |
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
| M1-17 | todo | — | S | 0 | — | Has a human gate |
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
- TR-16 (proposed) · The desktop core sends telemetry with its own small sender over PostHog's HTTP API; the facade owns the SQLite queue and the flag cache; `posthog-rs` is not used. Reason: the crate does not persist, has no offline flag cache, and forces aws-lc-rs into the build. Entry added to `docs/Locale Design Tradeoffs.md`.
- DU-48 (default, unconfirmed) · Digest §11 rule 7 becomes: "On desktop the facade owns a small SQLite queue and sends over PostHog's HTTP API itself (TR-16)."
- DU-46 (default, unconfirmed) · Dark-theme values the digest does not give (S5): selected tab `#A5B4FC`; badge tint `#312E81` with ink `#E0E7FF`; status pairs are the light pairs swapped (ok `#14532D`/`#DCFCE7`, warn `#78350F`/`#FEF3C7`, error `#912018`/`#FEE4E2`). Spacing, radius and type scales and the extra label "Verified" are also S5 defaults, in `packages/ui/tokens.json` and `labels.json`.
- DU-47 (default, unconfirmed) · Listener defaults (M0a-1): 20 port attempts after 7420, 10 s first-byte and TLS handshake timeouts, 5 s shutdown grace, HTTP/1.1 and h2 without ALPN, key and certificate stored as `tls/key.der` (mode 0600) and `tls/cert.der`; the pinned client does not check certificate dates (the pin is the trust decision).
- DU-45 (default, unconfirmed) · Three folders are added to the DU-7 layout: `scripts/` (developer scripts: doctor, fixture generator and verifier), `fixtures/` (test images and manifest; small files committed, two 50 MB files generated on demand), `packages/ui` (shared web UI kit and design tokens, pnpm package `@locale/ui`).

## 3. Pending digest edits

- §10 and §12: add DU-43 (binary units for the photo limit and chunk size).
- §3 "Core libraries" and §12: add DU-44 (rustls with `ring`).
- §11 rule 7 and §12: add DU-48 (facade owns the queue and the sender; TR-16).
- §9 and §12: add DU-46 (dark values chosen in S5).
- §5.1 and §12: add DU-47 (listener timeouts, port attempts, key file names).
- §3 "Repository layout" and §12: add DU-45 (`scripts/`, `fixtures/`, `packages/ui`).

## 4. Plan changes

- 2026-11-16 · S4 routed to Sonnet instead of Haiku. The fixture set as briefed needs hand-built EXIF, XMP and IPTC location blocks, a HEIC with GPS written through ImageIO, deterministic 50 MB files and an independent metadata check, which is more than boilerplate. The verifier stays Sonnet.
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
