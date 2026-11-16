set shell := ["bash", "-euo", "pipefail", "-c"]

flutter_dirs := "apps/mobile packages/locale_media packages/locale_net"

# List recipes
default:
    @just --list

# Everything that must pass before a commit
check: check-rust check-web check-mobile check-scripts

check-rust:
    cargo fmt --all --check
    pnpm install --frozen-lockfile
    pnpm -C apps/desktop build
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace

check-web:
    pnpm install --frozen-lockfile
    pnpm format:check
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

doctor:
    python3 -I scripts/doctor.py

fixtures-verify:
    @echo "fixtures-verify: not implemented yet (task S4)"

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
