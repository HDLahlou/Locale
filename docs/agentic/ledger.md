# Locale MVP build ledger

This file is the record of the build run. It overrides anyone's memory of the conversation. Statuses: todo, ready, in-progress, verifying, done, failed, blocked: <reason>, gate: <what the person must do>. Details for each task live in `docs/agentic/tasks/<ID>.md`.

## 1. Tasks

| ID | Status | Branch | Model | Attempts | Verified | Note |
|---|---|---|---|---|---|---|
| S1 | done | merged 9fab878 | S | 1 | PASS (Sonnet), `just check` green on build/integration | Skeleton, justfile, CLAUDE.md. Tauri crate is `locale-desktop`; TypeScript pinned to 6.0.3 |
| S2 | in-progress | worktree | H | 1 | — | Has a human gate (person installs what `just doctor` reports missing) |
| S3 | in-progress | worktree | O | 1 | — |  |
| S4 | in-progress | worktree | S (was H) | 1 | — | Has a human gate (person adds four real device photos) |
| S5 | todo | — | S | 0 | — |  |
| S6 | todo | — | S | 0 | — | Has a human gate |
| M0a-1 | in-progress | worktree | O | 1 | — |  |
| M0a-2 | todo | — | S | 0 | — |  |
| M0a-3 | todo | — | O | 0 | — |  |
| M0a-4 | todo | — | O | 0 | — | Has a human gate |
| M0a-5 | todo | — | O | 0 | — | Has a human gate |
| M0a-6 | todo | — | S | 0 | — |  |
| M0a-7 | todo | — | S | 0 | — |  |
| M0a-8 | todo | — | S | 0 | — | Has a human gate |
| M0a-9 | todo | — | H | 0 | — |  |
| M0a-10 | todo | — | H writes runbook | 0 | — | Has a human gate |
| M0b-1 | todo | — | S | 0 | — |  |
| M0b-2 | todo | — | S | 0 | — |  |
| M0b-3 | todo | — | H | 0 | — | Has a human gate |
| M1-1 | todo | — | S | 0 | — | Has a human gate |
| M1-2 | todo | — | O | 0 | — |  |
| M1-3 | todo | — | S | 0 | — |  |
| M1-4 | todo | — | S | 0 | — |  |
| M1-5 | todo | — | S | 0 | — | Has a human gate |
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
- DU-45 (default, unconfirmed) · Three folders are added to the DU-7 layout: `scripts/` (developer scripts: doctor, fixture generator and verifier), `fixtures/` (test images and manifest; small files committed, two 50 MB files generated on demand), `packages/ui` (shared web UI kit and design tokens, pnpm package `@locale/ui`).

## 3. Pending digest edits

- §10 and §12: add DU-43 (binary units for the photo limit and chunk size).
- §3 "Core libraries" and §12: add DU-44 (rustls with `ring`).
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
