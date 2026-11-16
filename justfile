set shell := ["bash", "-euo", "pipefail", "-c"]

flutter_dirs := "apps/mobile packages/locale_media packages/locale_net"

# List recipes
default:
    @just --list

# Everything that must pass before a commit
check: check-rust check-web check-mobile check-scripts

# Fixtures first: tests read fixtures/generated/ (the 50 MB files), which only exist after fixtures-verify
check-rust: fixtures-verify
    cargo fmt --all --check
    pnpm install --frozen-lockfile
    pnpm -C apps/desktop build
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace

check-web:
    pnpm install --frozen-lockfile
    pnpm format:check
    pnpm -r --if-present format:check
    pnpm -r --if-present lint
    pnpm -r --if-present typecheck
    pnpm -r --if-present test

check-mobile:
    for d in {{flutter_dirs}}; do \
      (cd "$d" && flutter pub get && dart format --set-exit-if-changed . && flutter analyze && flutter test); \
    done

# Runs the script tests only when they exist
check-scripts:
    if [ -d scripts/tests ]; then python3 -I -m unittest discover -s scripts/tests -t .; else echo "check-scripts: no scripts/tests yet"; fi

# Apply formatting everywhere
fmt:
    cargo fmt --all
    pnpm format
    for d in {{flutter_dirs}}; do (cd "$d" && dart format .); done

# Push HEAD to origin as ci/<suffix>, wait for the CI run, exit with its status
ci suffix:
    #!/usr/bin/env bash
    set -euo pipefail
    branch="ci/{{suffix}}"
    case "{{suffix}}" in ""|/*|*..*|*" "*|-*) echo "ci: bad suffix" >&2; exit 2;; esac
    sha=$(git rev-parse HEAD)
    if ! git push origin "HEAD:refs/heads/$branch"; then
      echo "ci: origin/$branch has different history; pick a new suffix, for example {{suffix}}-$(git rev-parse --short HEAD)" >&2
      exit 3
    fi
    id=""
    for _ in $(seq 1 30); do
      id=$(gh run list --workflow ci.yml --branch "$branch" --event push --commit "$sha" --limit 1 --json databaseId --jq '.[0].databaseId // empty')
      [ -n "$id" ] && break
      sleep 5
    done
    [ -n "$id" ] || { echo "ci: no run started for $branch at $sha" >&2; exit 1; }
    echo "ci: run $id"
    gh run watch "$id" --exit-status

doctor:
    python3 -I scripts/doctor.py

fixtures-verify:
    python3 -I scripts/fixtures_verify.py

mobile-it platform suite:
    @echo "mobile-it: not implemented yet (task M0a-6)"

plugin-test platform:
    @echo "plugin-test: not implemented yet (task M0a-7)"

harness scenario:
    @echo "harness: not implemented yet (task M1-16)"

package:
    @echo "package: not implemented yet (task M3-16)"

triage file:
    @echo "triage: not implemented yet (task M4-1)"
