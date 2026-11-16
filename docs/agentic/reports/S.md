# Milestone S report: setup done except one human gate

Nov 16, 2026 · This replaces the Nov 14 report, which recorded that the run stopped because the build tools were missing. A person installed them and restarted the run.

## Outcome

Milestone S is done except for one human gate. Five of its six tasks are verified and merged on `build/integration`: the repository skeleton (S1), the toolchain probe (S2), the interface contract (S3), the design tokens and UI kit (S5) and GitHub Actions CI (S6). The sixth, the fixture set (S4), has all its agent work merged and verified, and waits for a person to add four real phone photos. The run did not stop at this boundary; milestones M0a and M1 are already under way.

## What was built

- **Repository and tooling.** A Cargo workspace with eight crates, a pnpm workspace with two React apps and a shared UI package, one Flutter app and two Flutter plugins, a `justfile`, and `CLAUDE.md`. `just check` formats, lints and tests everything and needs no simulator, browser download or network.
- **Toolchain probe.** `just doctor` prints a table of tools and exits non-zero when one needed for M1 is missing. On this machine it exits 0. Missing but not needed for M1: `cargo-ndk`, Playwright browsers, `libheif`.
- **Interface contract.** `crates/proto` holds every endpoint, message, code, the item state table and the config defaults from digest §5–§7, with 24 generated JSON Schema files and `CONTRACT.md` rendered from the code. A test fails when either is stale.
- **Fixtures.** Twenty small committed images (plain, oriented, wide-colour, with location data in EXIF, XMP and IPTC, truncated, wrong type), two 50 MB files generated on demand, a manifest with hashes, and a verifier that checks the location claims with its own parsers.
- **Design tokens and UI kit.** `packages/ui` with one token file and one label table as the sources for CSS, TypeScript and Dart; seven components tested in light and dark.
- **CI.** One workflow on `ci/*` branches and pull requests: Rust on macOS and Windows, web checks with three browser engines, Flutter tests, script tests. `just ci <suffix>` pushes, waits and returns the run's status.

By product area, milestone S built only foundations: nothing yet in the desktop app, guest web page, phone app or account service beyond empty shells that build.

## How it was verified

Every task was checked three ways: the implementer ran its verification command and `just check`; the orchestrator re-ran both and read the real exit codes; a separate agent that had not seen the implementer's reasoning checked each acceptance criterion and wrote a report in the task file. After each merge `just check` was run again on `build/integration`; it passes at the time of writing.

CI proof for S6, read with `gh run view`: run 43838217560 passed every job; run 43838700494, with a deliberately failing Windows-only test, failed only the Windows Rust job.

No success criterion (SC1–SC8) is covered by an automated test yet. They need the pipeline tasks of M0a to M3.

## What needs a person

1. **S4: add four real device photos** (setup items 2 and 3). Runbook: `docs/agentic/runbooks/S4.md`. This unblocks the device-photo parts of M0a-5 (location strip) and M2-3 (validation). Nothing else waits on it.

Two gates in this milestone needed no action: every tool `just doctor` requires is installed (S2), and GitHub Actions is enabled and accepts `ci/*` pushes (S6).

One housekeeping item only a person can do: the remote branches `ci/s6-smoke` and `ci/s6-red` can be deleted. The second holds the deliberately failing test; it was never merged.

## Defaults and decisions made

Each can be overturned; the digest and `docs/Locale Design Tradeoffs.md` hold the details.

- DU-43: sizes use binary units (photo limit 52,428,800 bytes, chunk 1,048,576 bytes, storage 20 GiB and 5 GiB).
- DU-44: TLS uses rustls with the `ring` provider.
- DU-45: `packages/ui`, `scripts/` and `fixtures/` added to the layout.
- DU-46: dark-theme colours the sources do not give.
- DU-47: listener timeouts, port attempts and key file names.
- DU-48 and TR-16 (proposed): the desktop core sends telemetry with its own small sender and owns the offline queue; `posthog-rs` is not used.
- DU-49: contract details fixed in code (encodings, HTTP statuses, the exact signed bytes, how the two authenticated service endpoints authenticate, config bounds). Listed in digest §5.8.
- DU-50: capability-check reasons and fix identifiers.
- DU-51: a `crates/telemetry` crate; `proto` keeps only the telemetry schema.
- DU-52: the decode worker as built on macOS, and `just check-rust` running `fixtures-verify` first.
- Plan changes: S4 went to Sonnet instead of Haiku; M1-17 was split into M1-17a (schema) and M1-17b (queue and sender).

## Problems

- **Two interruptions by the account's usage limit.** The second ended six running subagents at once. Nothing was lost, because each agent's work was on disk and the agents were resumed, but it cost time, and it will recur on long runs.
- **CI found three real Windows defects in merged work:** a missing icon file, a timeout that did not kill a child's children, and fixtures altered by line-ending conversion. All are fixed. Windows code should be pushed to CI before it merges, not after.
- **Exit codes hidden by pipes.** Several agents reported a check as passed or failed from output read through `tail`. `CLAUDE.md` now tells agents to write output to a file and print the exit code.
- **A test printed "FAIL" while passing,** and two agents took it for a failure. Fixed.
- **Haiku was a poor fit for a research note with source citations** (M0a-9 needed two returns). The next one goes to Sonnet.
- **One agent wrote a force-push into a recipe,** which the project's permissions forbid. It was removed before verification.
- **Verifiers could not write under `.claude/worktrees/` from outside a worktree.** They now get their own worktree.

What I would design differently: the workspace dependency table conflicts on almost every merge, because parallel tasks each add a line. A script resolves it, but seeding the common dependencies in S1 would have avoided it.

## Cost

Subagents launched for milestone S: 6 implementers (1 Opus, 4 Sonnet, 1 Haiku) and 6 verifiers (all Sonnet). Returns: S4 once (committed bytecode), S6 three times before verification (two for defects in other tasks that CI exposed, one for the force-push). S3, the Opus task, needed none. Five CI runs.

## To restart

The run is not paused. If it stops, `docs/agentic/run-state.json` gives the reason; run `docs/agentic/start-build.sh` to continue.
