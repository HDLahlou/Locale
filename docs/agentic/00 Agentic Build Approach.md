# Agentic build approach for the Locale MVP

Nov 13, 2026 · Draft · How to go from the design docs to a working MVP with Claude Fable 5.2 orchestrating cheaper subagents, and how to keep the token bill down.

## Summary

Run it in two prompts, with a short human checkpoint between them:

1. **Readiness review** (`01 Readiness Review Prompt.md`). Fable reads the design once, decides whether it is buildable without a person in the loop, and writes compact files that replace the raw docs for every later agent: a spec digest, a screen inventory, a work breakdown and a list of open questions.
2. **Human checkpoint.** You answer the open questions and sort out the things agents can't do alone, such as developer accounts, devices and signing.
3. **Build orchestration** (`02 Build Orchestration Prompt.md`, outlined below). Fable plans and dispatches tasks to Sonnet and Haiku subagents. Fresh-context verifier agents check each task against its acceptance criteria. Progress lives in files in the repository, not in any one conversation.

The biggest saving comes from step 1. Every agent afterwards reads a 12,000-token digest instead of about 150,000 tokens of design docs, HTML mockups and Word files.

## Is there enough here? (my preliminary read)

The design is strong on product scope, flows, decisions, the state model and the UI. Fable's review should confirm this, but expect these gaps:

- **Interfaces are described, not specified.** There are no endpoint list or payload schemas for the host API, no index sync message formats and no error code table (the design review's T26). Agents can write these, but someone should approve them before parallel work starts.
- **Stack choices are still open.** The Rust core and Tauri desktop shell are proposals (D10). The phone UI framework (decided later: Flutter, see `docs/Locale Design Tradeoffs.md`) and the account service's stack and hosting are undecided.
- **Some states have no mockup.** Account, settings, permission prompts, empty and error states, the host's own gallery, and the Posts tab on desktop.
- **Role names are a copy TODO** (UI rules).
- **Hands-off limits.** Agents can build and test most of the host core, sync, validation and the desktop app on one machine. They cannot replace real phones on a real Wi-Fi network, App Store and Play signing, developer accounts, an email or SMS provider for sign-in codes, or the libheif licensing decision. Plan human gates there (M0a, M0b, M4, M5).

## Model roster

Prices are per million tokens (input / output), from Anthropic's current price list.

| Role | Model | Effort | Price | Use for |
|---|---|---|---|---|
| Orchestrator and planner | Claude Fable 5.2 (`claude-fable-5-2`) | high | $8 / $40 | Readiness review, the build plan, briefing subagents, judging verifier reports, settling conflicts |
| Hard modules | Claude Opus 5.5 (`claude-opus-5-5`) | high | $4 / $20 | The sync protocol, pinned TLS and tokens, the HEIC/validation pipeline: work where a subtle bug is expensive |
| Implementer | Claude Sonnet 5.5 (`claude-sonnet-5-5`) | medium | $2 / $10 | Most tasks: features, screens, tests, fixes |
| Verifier | Claude Sonnet 5.5 | low to medium, fresh context | $2 / $10 | Checking each task against its acceptance criteria and tests, without seeing the implementer's reasoning |
| Reader and helper | Claude Haiku 5.5 (`claude-haiku-5-5`) | low | $0.10 / $0.50 | Extraction, code search, inventories, boilerplate, docs, changelogs |

Rules of thumb:
- Start Fable at **high**, not xhigh or max. Most of its gain arrives by high.
- Prefer a cheaper model at a sensible effort over a more expensive model at low effort for well-specified tasks.
- Judge cost per completed task. A cheap agent that needs three retries isn't cheap.

## Ways to save tokens, most valuable first

1. **Convert sources with scripts before any model reads them (done).** `docs/agentic/tools/` holds two stdlib Python scripts that turn the Word docs into Markdown and the mockup HTML into per-screen outlines in `docs/agentic/source/`. This costs zero model tokens. The mockups shrink from 180 KB of HTML to a 48 KB outline (an estimated 50,000+ tokens down to about 12,000). The Word docs come out about the same size as plain text (about 17,000 tokens together), but with real headings and tables, which plain-text conversion flattens into one cell per line. All three files together are about 29,000 tokens (estimates at roughly four characters per token). Re-run them whenever the mockups or Word docs change:
   ```sh
   python3 -I docs/agentic/tools/docx_to_md.py "docs/Locale Content Submission System Design.docx" docs/agentic/source/content-submission-design.md
   python3 -I docs/agentic/tools/docx_to_md.py "docs/Locale Design Review and System Design Feedback.docx" docs/agentic/source/design-review.md
   python3 -I docs/agentic/tools/extract_mockups.py design/mvp-ui/project docs/agentic/source/screens.md
   ```
2. **Digest once, reuse everywhere.** The readiness review writes `spec-digest.md` and `screen-inventory.md`. Every later agent reads those plus the task brief, never the raw docs.
2. **Delegate reading to Haiku.** At one eightieth of Fable's price, use Haiku for every "read these files and summarise" job. Fable reads only the primary spec and the digests.
3. **Small, complete task briefs.** Each subagent gets a goal, the digest sections it needs, the files to touch, acceptance criteria and a verification command. Complete briefs prevent clarifying round-trips; small ones keep context short.
4. **Short replies, details in files.** Subagents reply in ten lines or fewer and write details to the task's file or to the progress ledger. The orchestrator's context then stays small over a long run.
5. **Deterministic checks before model review.** Type checks, linters, unit tests and builds run first and cost no tokens. Verifier agents review only work that already passes them.
6. **Fresh-context verifiers, not self-review loops.** One independent verifier per task beats repeated self-critique, and costs less than chains of agents reviewing each other.
7. **Stable prompt prefixes for caching.** Keep `CLAUDE.md` and the digest unchanged during a milestone. Claude Code caches stable prefixes automatically, and cache reads cost a fraction of normal input. Edit them between milestones, not mid-run.
8. **Limit unneeded delegation.** Tell Fable what a subagent costs and when one is worth it. Don't invite it to delegate small tasks or to have subagents review each other.
9. **No token budgets in the prompt text.** Fable 5.2 spends differently when it sees a budget; caps belong in the harness (turn limits, stop conditions).
10. **Batch work that isn't urgent.** If you later script work through the API, such as generating test fixtures or bulk copy variants, the Message Batches API halves the cost.

Running inside Claude Code uses your plan's limits rather than per-token billing, but the same habits stretch those limits just as much.

### Where each measure is implemented

| Measure | Implemented in |
|---|---|
| Convert sources with scripts | `docs/agentic/tools/docx_to_md.py`, `extract_mockups.py`; output in `docs/agentic/source/` |
| Digest once, reuse everywhere | `spec-digest.md`, `screen-inventory.md`, `work-breakdown.md` |
| Read only the sections a task needs | `docs/agentic/tools/digest_section.py`; every brief in `02 Build Orchestration Prompt.md` uses it (a typical task reads about a sixth of the digest) |
| Delegate reading to Haiku | Orchestration prompt, "Token discipline"; Haiku verifiers for document-only tasks |
| Small, complete briefs; short replies, details in files | Brief and verifier templates in the orchestration prompt; task files `docs/agentic/tasks/<ID>.md` |
| Deterministic checks before model review | Task loop step 5; output trimmed to failures |
| Fresh verifiers, not review loops; reuse warm implementers for fixes | Task loop steps 5–7; two return rounds, then one Opus escalation |
| Stable prefixes for caching | Fixed opening paragraph in both templates; digest and `CLAUDE.md` frozen during a milestone, with queued edits applied at the boundary |
| Limit unneeded delegation | "Token discipline": model prices, no agents for small reads, batching of small same-model tasks |
| No token budgets in prompts | Stated in "Token discipline"; limits live in the harness |
| Stop paying for a broken environment | Circuit breaker and pause in the orchestration prompt; `run-state.json` |
| Message Batches API | Not applicable inside Claude Code; use it only if fixtures or copy are later generated through the API |

## Running it unattended

- **Start or restart:** `docs/agentic/start-build.sh`. It shows the run state, switches to `build/integration` and launches Claude Code with `.claude/orchestrator-settings.json`.
- **What the settings do:**
  - Fable 5.2 at high effort.
  - An allowlist; anything else is denied instead of prompting, so the session never hangs on a prompt.
  - Agent worktrees branch from the current branch, not `origin/main`.
  - A SessionEnd hook (`.claude/hooks/pause_on_session_end.py`) marks the run paused if the session ends while running.
- **Pausing:** the orchestrator pauses itself when progress stops (repeated environment failures, a recurring error, no task finished in eight dispatches, repeated denials, a broken integration build). It writes the reason to `docs/agentic/run-state.json` and the milestone report.
- **Restarting:** nothing restarts it automatically. Read the reason, fix what it names, and run the start script again.

## The phases end to end

| Phase | Who | Output | Gate |
|---|---|---|---|
| 0. Readiness review | Fable (high) + Haiku/Sonnet readers | `docs/agentic/` digest, inventory, work breakdown, open questions, report | You read the report |
| 1. Decisions | You | Answers in `open-questions.md`; accounts, devices and licences sorted | Everything blocking M0–M2 is answered |
| 2. Project setup | Fable plans, Sonnet builds | Repository skeleton, `CLAUDE.md` (stack, conventions, commands), CI with tests and lint, the progress ledger | CI green on an empty build |
| 3. Build loop, per milestone | Fable orchestrates; Sonnet/Opus implement; Sonnet verifies; Haiku helps | Merged tasks with tests, ledger updates, a milestone report | Milestone demo checklist passes; human gate where the work breakdown marks one |
| 4. Field test (M4) | You, with agents fixing | Bug list from real events, fixes | Success criteria met |

## Outline of the second prompt (build orchestration)

Write this after phase 1, so it can reference the final digest and answers. It should cover:

1. **Goal and ownership.** Build the MVP described in `docs/agentic/spec-digest.md`, milestone by milestone, following `work-breakdown.md`. Say that Fable owns the codebase's design, so it restructures where needed rather than piling on.
2. **State lives in files.**
   - `docs/agentic/ledger.md`: each task's status, owner agent, branch, verification result and notes.
   - The orchestrator rereads the ledger at the start of each milestone instead of relying on conversation history, so long runs survive compaction and restarts.
3. **The task loop.** Pick ready tasks whose dependencies are done, then for each:
   - write a brief;
   - dispatch it to the model and effort in the work breakdown, in parallel where tasks don't touch the same files (separate git worktrees);
   - run the deterministic checks;
   - send passing work to a fresh verifier;
   - merge, or return it with the verifier's findings, at most two rounds before escalating to Opus or flagging for a human;
   - update the ledger.
4. **Subagent briefs.** A fixed template: goal, digest sections, files in scope, acceptance criteria, verification command, "reply in ten lines, details in the task file".
5. **What is pre-approved and what isn't.**
   - **Pre-approved:** creating branches and worktrees, running tests and builds, local simulators and emulators, merging verified tasks into the integration branch.
   - **Needs a person:** pushing to the remote, spending money, creating external accounts, touching signing keys, deleting data outside the repo, and anything marked as a human gate.
6. **Stop and report conditions.**
   - Stop at the end of each milestone, at any human gate, or when a task fails verification three times.
   - The milestone report says what was built, what was verified and how, what was defaulted, and what needs a person, written for a reader who wasn't watching.
7. **Finishing work and clear reports.** Reuse the "Finishing work" and "Communicating clearly" guidance from the first prompt.

## Risks to watch

- **Spec drift.** If agents change interfaces without updating the digest, parallel work diverges. Require every interface change to go through the orchestrator and into the digest.
- **Silent defaults.** Agents fill gaps with plausible choices. Keep the "Default (unconfirmed)" markers and review them at each milestone.
- **Over-delegation cost.** Subagent spawning can dominate the bill. Watch subagent count per milestone in the ledger.
- **Untestable surfaces.** Multi-device behaviour over real Wi-Fi, iOS backgrounding and hotspot setups need real hardware. Agents should build a local multi-instance test harness, but the M0b and M4 human checks stay.
