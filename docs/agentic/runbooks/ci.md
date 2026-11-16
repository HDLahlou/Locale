# CI runbook

CI is `.github/workflows/ci.yml`. It runs on every push to a `ci/*` branch and on every pull request. This is the only place Windows code is compiled and tested (TR-12).

Jobs: `rust` and `scripts` on macOS and Windows; `web` (checks plus guest-web Playwright in Chromium, Firefox, WebKit) on Ubuntu; `mobile` (Flutter 3.47.7, `just check-mobile`) on macOS. The matrices do not fail fast, so a Windows failure still shows the macOS result. `just doctor` is shown in the `scripts` job but never fails it.

## Run CI for a task

1. Commit your work on your branch.
2. `just ci <task>-<what>`, for example `just ci s7-windows`. It pushes the current commit to `origin` as `ci/<suffix>`, waits, and exits with the run's status. It refuses a suffix that is empty, starts with `/` or `-`, or contains `..` or spaces.
3. `just ci` never force-pushes. If `origin/ci/<suffix>` already has different history the push is rejected and the recipe prints "ci: origin/ci/<suffix> has different history; pick a new suffix, for example <suffix>-<short sha>" and exits 3. Pick a new suffix and run it again.
4. Old `ci/*` branches (including `ci/s6-red`, which holds a deliberately failing test and is never merged) can only be deleted by a person.
5. Only `ci/*` branches are ever pushed for CI. Never push `main` or `build/*` for this.

A new push to the same branch cancels the older run. Runs cost shared minutes: do not push on every edit.

## Read a failure

1. `gh run list --branch ci/<suffix> --limit 3` gives the run ID.
2. `gh run view <id> --json conclusion,jobs --jq '.jobs[] | [.name, .conclusion] | @tsv'` shows which jobs failed.
3. `gh run view <id> --log-failed | tail -80` shows the failing step output.

## Run one crate's Windows tests

```
gh workflow run ci.yml --ref <ci-branch> -f package=<crate> -f filter=<test name filter>
gh run list --workflow ci.yml --event workflow_dispatch --limit 1
gh run watch <id> --exit-status
```

`package` is a Cargo package name (`host-core`, `client-core`, `proto`, ...); `filter` may be empty. This runs the desktop web build, then `cargo test -p <package> <filter>` on `windows-latest` only.

`workflow_dispatch` can be started only after the workflow file exists on the default branch (`main`). Until then, push a `ci/*` branch with `just ci` instead.

## Minutes used in the smoke run

Smoke run 43838217560 (wall-clock per job; macOS minutes bill at 10x, Windows at 2x on private repositories):

| Job | Minutes |
|---|---|
| mobile (macOS) | 2.25 |
| web (Ubuntu) | 1.5 |
| rust (Windows) | 2.75 |
| rust (macOS) | 1.2 |
| scripts (Windows) | 0.7 |
| scripts (macOS) | 0.7 |

Whole run about 3 minutes of wall time. These are with a warm or first-time cache as it happened; expect the first run on a branch with new dependencies to be slower.

## If runs do not start

A person must enable Actions on the repository: GitHub, Settings, Actions, General, "Allow all actions and reusable workflows" (setup item 5). If a push to `ci/*` shows no run after about a minute, stop and report the exact command and message; do not try other ways to trigger.
