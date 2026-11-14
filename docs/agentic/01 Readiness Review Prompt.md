# Prompt 1: Readiness review (for Claude Fable 5.2)

How to run: open this repository in Claude Code, set the model to **Claude Fable 5.2** and effort to **high**, then paste everything below the line. Expect a long turn (tens of minutes). It writes only to `docs/agentic/` and changes nothing else.

---

You are reviewing the Locale repository to decide whether its design is complete enough to build the MVP through a hands-off, agent-driven process: in a later session you will act as the orchestrator, and lower-cost subagents will write code, run checks and review each other's work. In this session you build nothing. Your job is a clear readiness verdict, plus a set of compact files that the build phase will load instead of the raw design documents.

## What Locale is

Locale lets people at the same place share full-quality photos across iPhone, Android and computers, with no cloud holding the content. One device (a laptop in the MVP, a phone later) hosts a temporary event; attendees join by QR code over the local network; everything is deleted when the event ends.

## Sources, and how to read them cheaply

| Path | What it is | How to read it |
|---|---|---|
| `docs/Locale MVP Flows.md` | The primary spec: scope, decisions D1–D17, flows 5.1–5.12, state model, data model, screens and UI rules, targets, build order (M0a–M5), risks | Read it fully yourself. It wins over every other source when they disagree |
| `docs/Locale PostHog Plan.md` | Analytics, error tracking and remote config plan | Read sections 2, 4, 5 and 7 yourself; skim the rest |
| `docs/agentic/source/screens.md` | All UI mockups, already converted from the canvas source into one outline per screen: tweaks (alternate states), what is on screen (with `▸ if` for conditional parts and `▸ for each` for repeated parts), links between screens, and the sample data and state logic. About 12,000 tokens | Read it yourself. `Accent: soft indigo` and `Event page options` are decision records (soft indigo accent; event page "Option A"): note the decisions, skip the detail. Open the original `design/mvp-ui/project/*.dc.html` only if the outline leaves a specific question open |
| `docs/agentic/source/content-submission-design.md`, `docs/agentic/source/design-review.md` | The two Word documents, already converted to Markdown with headings and tables: the older, deeper design of the upload pipeline and its subroutines, and the design review. TODO IDs T1–T36 come from here. About 17,000 tokens together | Read the sections the MVP doc relies on (subroutines, state model, TODOs, open questions). The MVP doc supersedes them where they disagree. Never open the `.docx` files |
| `docs/Locale notes.md` | The original PRD and scratch notes | Only for product intent; the MVP doc supersedes it |
| `flows/*.png` | Flow diagrams | Skip; the design doc transcribes them |

Ignore `docs/PostHog Interview Prep.*` (personal notes, not part of the product) and editor folders.

## What "ready" means

The design is ready for agentic development when an implementer agent, given only the spec digest and one task, can build that task without guessing at anything that matters, and a separate verifier agent can confirm it is done without asking a person. Judge the repository against that standard. Areas to examine:

- **Scope and acceptance:** are the success criteria and per-feature behaviour testable as written?
- **Stack and architecture decisions:** which are settled (D1–D17) and which would an agent have to guess? For example, the phone UI framework, the shape of the Rust host core, the desktop shell, the app service.
- **Interfaces:** host API endpoints and payloads, index sync messages, the QR join URL, reason codes, the telemetry schema. Are they specified well enough to code against and to test? (The design review lists missing request and response contracts as T26.)
- **Data and state:** can a schema, migrations and state-transition tests be written straight from the data model and transition table?
- **Security:** pinned TLS, session and upload tokens, signed account credentials, image validation. Is each one implementable as specified?
- **UI:** does every screen in the MVP doc's screens section have a mockup? Are empty, loading, error and permission-denied states covered? Which copy is still a TODO (for example the role names)?
- **Testability:** for each milestone, what can be verified automatically (unit, integration, simulators and emulators, a local multi-device harness), and what needs real devices, real networks or a person?
- **Hands-off blockers:** what agents cannot do alone. For example, physical phones on a shared Wi-Fi, iOS and Android signing and developer accounts, hosting the account service, an email or SMS provider for sign-in codes, the HEVC/libheif licensing decision, a PostHog project.
- **Sequencing:** does the build order break down into tasks with clear dependencies?

## Use subagents to save tokens

You are the most expensive model in this setup: per token, Claude Sonnet 5.5 costs a quarter of what you do and Claude Haiku 5.5 costs one eightieth. Spend your own tokens on judgment, not on reading long files.

- The mockups and Word documents are already converted (see the table above), so most reading is cheap enough to do yourself. Use subagents for mechanical, high-volume work, such as checking every screen against the screens section of the MVP doc, or listing every reason code and interface the docs mention.
- Send that mechanical work to **Haiku at low effort**.
- Send first drafts that need some judgment (for example a first pass of the screen inventory or of a digest section you name) to **Sonnet at medium effort**. Then review and correct the draft yourself.
- Brief each subagent fully: the exact files, the output path, the format and a size limit. Ask for a reply of ten lines or fewer, with the details written to the file.
- Run independent extractions in parallel.
- Don't start a subagent for something you can settle in a few reads, and don't have subagents re-review each other's summaries.

## Deliverables

Write these under `docs/agentic/`. They are inputs to the build phase, so keep them compact and precise. Give every item a source reference, such as "MVP §5.4 step 11" or "D11".

1. **`readiness-report.md`**
   - The verdict first: Ready, Ready with gaps, or Not ready, with two or three sentences on why.
   - A scorecard with one row per area above: Ready, Gaps or Blocking, with evidence.
   - The blocking gaps. For each: what is missing, why it blocks an agent, and a proposed resolution, marked "I can default this" or "Needs a human".
   - The hands-off blockers, and which milestones each one affects.
   - A feasibility read for each milestone: how much of it agents can build and verify unattended.
2. **`spec-digest.md`:** the build phase's single source of truth.
   - **Contents:** decisions, flows, interfaces, data model, state machine, UI rules, non-functional targets and telemetry rules, written for an implementer.
   - **Size:** aim for 12,000 tokens or fewer.
   - **Gaps you resolved yourself:** mark the choice "Default (unconfirmed)".
   - **Conflicts between sources:** follow the MVP doc and list the conflict.
3. **`screen-inventory.md`:** one entry per screen with:
   - purpose;
   - elements;
   - states;
   - actions and navigation;
   - data it needs from the host;
   - the flows it implements;
   - gaps.
4. **`work-breakdown.md`:** milestones, then tasks. Each task has:
   - goal;
   - the spec digest sections it needs;
   - acceptance criteria;
   - how to verify it (a test, a command, a check);
   - dependencies;
   - size (S, M or L);
   - recommended model and effort;
   - whether it needs a human gate.
5. **`open-questions.md`:** only the decisions a person must make. Order them by how early they block work. Give each your recommended answer and what happens if your default is taken.

## Ground rules

- Do not write application code, change existing docs or design files, or commit. You may create and edit files under `docs/agentic/` (leave `docs/agentic/source/` and `docs/agentic/tools/` as they are), and run read-only shell commands (`ls`, `grep`, `wc`).
- When something is ambiguous but low-impact, choose a sensible default, record it in the spec digest, and keep going. Put questions for a person only in `open-questions.md`; don't stop to ask.
- **Finishing work.** Don't stop while deliverables remain. If you hit an ordinary obstacle (a file that won't convert, a missing tool), work around it and note it. Stop early only for a real blocker, and say what it is.
- **Communicating clearly.** Write the report for an intelligent reader who hasn't seen your tool calls or the files you opened. Put the key points first, use plain words and full sentences, define any term you introduce, and keep bullets for lists of separate items.

When you finish, reply with:
- the verdict;
- the five most important blockers;
- a rough estimate of the build phase's size (number of tasks, and which milestones can run unattended);
- the paths of the files you wrote.
