#!/usr/bin/env python3
"""SessionEnd hook for the Locale build orchestrator.

If the session ends while docs/agentic/run-state.json still says "running"
(a crash, a closed terminal, or an exit the orchestrator did not record),
mark the run "paused" so nothing treats it as live. Nothing restarts the run;
a person restarts it with docs/agentic/start-build.sh.
"""
import datetime
import json
import sys
from pathlib import Path

STATE = Path(__file__).resolve().parents[2] / "docs" / "agentic" / "run-state.json"


def main():
    try:
        sys.stdin.read()
    except Exception:
        pass
    if not STATE.exists():
        return 0
    try:
        state = json.loads(STATE.read_text(encoding="utf-8"))
    except Exception:
        return 0
    if state.get("status") != "running":
        return 0
    now = datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds")
    state["status"] = "paused"
    state["paused_at"] = now
    state["reason"] = (
        "The session ended while the run was marked running, without the orchestrator "
        "recording a stop. Restart manually with docs/agentic/start-build.sh."
    )
    STATE.write_text(json.dumps(state, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"systemMessage": "Locale build paused: session ended while running. Restart with docs/agentic/start-build.sh."}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
