#!/usr/bin/env bash
# Start or manually restart the Locale build orchestrator.
# Usage: docs/agentic/start-build.sh
# Runs an interactive Claude Code session with the orchestrator settings
# (Fable 5.2 at high effort, no permission prompts, worktrees from the current
# branch, a SessionEnd hook that marks the run paused). Nothing restarts the
# run automatically: run this script yourself after reading the pause reason.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

echo "Current run state (docs/agentic/run-state.json):"
cat docs/agentic/run-state.json
echo

branch="$(git rev-parse --abbrev-ref HEAD)"
if [ "$branch" != "build/integration" ]; then
  if [ -n "$(git status --porcelain)" ]; then
    echo "You are on '$branch' with uncommitted changes. Commit or stash them, then switch to build/integration." >&2
    exit 1
  fi
  git switch build/integration
fi

read -r -p "Start the orchestrator now? [y/N] " answer
case "$answer" in
  y|Y|yes|YES) ;;
  *) echo "Not started."; exit 0 ;;
esac

exec claude --settings .claude/orchestrator-settings.json \
  "Run the Locale build. Read docs/agentic/02 Build Orchestration Prompt.md and follow it exactly. This start was requested by a person, so a paused run may resume."
