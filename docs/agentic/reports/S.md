# Milestone S report: stopped before the first task

Nov 14, 2026 (02:40 UTC)

## Outcome

The run stopped before any task was dispatched, because the machine does not have the build tools. Rust, `just` and `pnpm` are missing, so the first task (S1, the repository skeleton) cannot be verified, and all 91 other tasks depend on S1. No code was written and no subagents were launched. The run is waiting for a person to install the tools (setup item 1 in `open-questions.md`).

## What was built

Nothing in any area (host core, client core, desktop, guest web, phone app, service). The only new files are the ledger (`docs/agentic/ledger.md`, 92 task rows), the updated run state and this report.

## How it was verified

There is nothing to verify yet. No test suite exists, and no success criterion (SC1–SC8) is covered.

## What needs a person

Install the missing tools, in this order of how much each unblocks. I checked each by running it from the session's shell; "missing" means the command was not found on `PATH`.

| Tool | State | Unblocks |
|---|---|---|
| Rust through `rustup` (`cargo`, `rustc`), stable toolchain | Missing (`~/.cargo/bin` does not exist) | Every task. Nothing can start without it |
| `just` | Missing | Every task; `just check` is the common verification command |
| `pnpm` | Missing (Node 26.7 and npm are present) | S1, S5, and all desktop and guest-web tasks |
| Full Xcode with an iOS simulator, selected with `xcode-select` | Missing. Only the Command Line Tools are active, so `xcodebuild` and `xcrun simctl` fail | The macOS image codec builds, all iOS simulator tests, Tauri bundling |
| Android SDK, NDK, platform tools (`adb`) and an emulator image (API 28 and current) | Missing (`~/Library/Android` does not exist) | All Android emulator tests |
| `flutter_rust_bridge_codegen` | Missing. It can be installed with `cargo install` once Rust is present; the orchestrator may do this itself on the next run | M0a-6, M1-15 and the phone app |
| GitHub CLI `gh`, signed in | Missing | S6 and every Windows check on GitHub Actions (setup items 5–6) |

Already present: Node 26.7, npm, Flutter 3.47.7 with Dart 3.13.5, Python 3.12, `sips`, JDK 17, Homebrew.

With only the first three installed, the Rust, desktop and guest-web tasks can run, and the phone and Windows tasks will be marked blocked until the rest arrive. Task S2 will then produce `just doctor`, which checks finer points this probe did not, such as Rust cross-compilation targets and libheif.

The other human inputs in the setup checklist (device photos, CI access, accounts) are unchanged and are not needed yet.

## Defaults and decisions made

None.

## Problems

The only problem is the missing toolchain. I chose to stop rather than have agents write the skeleton unverified, because the brief requires `just check` to pass before a task counts as done, and unverified work would have to be redone or rechecked anyway.

## Cost

No subagents were launched. No task needed a return or an escalation.

## To restart

Install at least Rust (through `rustup`), `just` and `pnpm`, and ideally everything in the table above. Open a new terminal and confirm `cargo --version`, `just --version` and `pnpm --version` all print a version. Then run `docs/agentic/start-build.sh`.
