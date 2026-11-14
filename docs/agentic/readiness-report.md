# Locale MVP: readiness for agent-driven development

Nov 13, 2026 · Review of the repository against one standard: an implementer agent, given only the spec digest and one task, can build that task without guessing at anything that matters, and a separate verifier agent can confirm it is done without asking a person.

## Status update, Nov 13, 2026 (after the review): ready to start development

All open questions are now resolved (`open-questions.md`): Flutter phones and Tauri desktop, the interface contract approved, email sign-in through Postmark on Fly.io, Windows on GitHub Actions, and confirmations of the "original", save-counting, role-label, derived-screen, licensing and naming defaults. The reasons and rejected options are in `docs/Locale Design Tradeoffs.md`. Blocking gaps B1–B11 below are closed. What remains is the human setup checklist at the end of `open-questions.md`: none of it is needed before task S2, and each item names the first task that needs it. The rest of this report is the original review, kept as written.

## Verdict at review time: Ready with gaps

The product design is unusually complete for this stage: scope, 17 recorded decisions, twelve step-numbered flows, a state transition table, a data model, numeric targets and mockups for most screens. What is missing is the layer agents code against. There is no endpoint list, no message format and no credential format, and several stack choices are still open. I have filled those gaps with defaults in `spec-digest.md`, each marked "Default (unconfirmed)", so the build can start once a person approves the interface contract (question Q2). The stack was decided after this review: Flutter for the phones (`docs/Locale Design Tradeoffs.md`, TR-1). Nothing in the design has to be rethought.

Separately, about a fifth of the work cannot be verified by agents at all, because it needs real phones, a real Wi-Fi network, a Windows machine, signing identities or a hosted service. Those are scheduled as human gates and do not stop the build between them.

**Terms used below.** A *default* is a choice I made where the sources were silent, recorded in the digest with an ID (DU-n). A *human gate* is a task a person must perform or approve. The *harness* is a test tool planned in the work breakdown (task M1-16) that runs a real host and many simulated clients on one machine, with a controllable clock and injected network faults.

## Scorecard

| Area | Rating | Evidence |
|---|---|---|
| Scope and acceptance | **Ready** | Eight success criteria and non-goals (MVP §2); an in/out table for every idea in the notes (MVP §1); per-flow "out of scope" lines. Six criteria are testable as written. Two need a definition: "byte-identical to the original" conflicts with metadata stripping (Q5), and "typical home network" has no stated speed |
| Stack and architecture | **Gaps** | Settled: host device as server, app service for accounts only, SQLite single writer, same-network transport, Rust core with Tauri (D1–D10). **Update Nov 13, 2026:** phone UI decided as Flutter over the Rust core (`Locale Design Tradeoffs.md`, TR-1). Still defaulted: the web UI framework, the app service's stack and hosting, how the core is split between host and client (DU-3 to DU-7); desktop UI follow-up Q1b |
| Interfaces | **Blocking** at review; **resolved** when the contract was approved (Q2) | Only three things are specified to code level: the QR URL shape (MVP §5.2 step 9), the list of index change kinds (MVP §5.6 step 3) and nine item reason codes (MVP §6). Missing: every endpoint and payload, the sync message format, error format, join-failure codes, the upload chunk protocol, mDNS fields, how TLS and HTTP share an address, the fingerprint encoding. This is review item T26, still open. Full proposal in digest §5 (DU-30 to DU-33); Q2 |
| Data and state | **Ready, small gaps** | Tables and a 13-row transition table with invariants (MVP §6, §7) are enough to write a schema and table-driven tests. Gaps: the event table has no join token, host account, browser-guest flag or frozen config; there is no session table; one flow step (integrity re-send) has no transition; same-user duplicates have no defined row state. Defaults DU-12, DU-13, DU-34 |
| Security | **Gaps** | Pinned TLS, session and upload token bindings, offline credential checks, image allowlist and a separate decode process are all decided (D4–D7, MVP §5.4). Not specified: credential and signature format, key rotation, token formats, what "sandboxed" requires, how to strip HEIC metadata without re-encoding, how the client-isolation test works. One design weakness: the credential is a bearer token that any host can replay elsewhere. Defaults DU-31, DU-36 to DU-38 |
| UI | **Gaps** | 22 screens are listed in MVP §8. 12 have a full mockup, 5 a partial one, 5 none (desktop sign-in, the host's own event view, upload picker, Account, guest gallery). Loading, empty, error, permission-denied, event-ended and confirmation states are almost entirely unmocked; only "host offline" and one failed capability check are drawn. Six mockup details contradict the MVP doc. Role names are a copy TODO. Colours and layout rules are precise. Defaults in `screen-inventory.md` section D; Q7, Q8 |
| Testability | **Gaps** | The transition table, numeric timeouts and success criteria convert directly into tests. Nothing in the repository says how to test multi-device behaviour; the work breakdown adds the harness for that. Real radios, OS prompts, firewalls, hotspots and phone backgrounding cannot be automated. Detail per milestone below |
| Hands-off blockers | **Gaps** | Ten external needs, listed below. None blocks the start; the first one bites at the M0a exit check |
| Sequencing | **Ready** | MVP §11 gives seven milestones with exit checks. It does not place extend, Host again, posts or telemetry in a milestone (DU-41), and it has no setup step. The work breakdown resolves both: 92 tasks with dependencies |
| Telemetry | **Ready** | The PostHog plan gives rules, an event list with properties, flags with defaults and bounds, and an exit test ("forbidden properties are dropped"). Two reason-code lists differ slightly from the MVP doc (digest §13, C9) |

## Blocking gaps

Each of these would make two agents build incompatible things, or make a verifier unable to say "done".

| # | What is missing | Why it blocks an agent | Proposed resolution | Who |
|---|---|---|---|---|
| B1 | Host API, sync messages, error codes, chunk protocol (T26) | Four clients and the host are built in parallel against it; any guess diverges | Digest §5; task S3 turns it into a shared types crate and a contract document | I can default this; **a person should approve it once** (Q2) |
| B2 | Phone UI framework, web UI framework, app service stack | The first task (repository skeleton) depends on it; a wrong guess redoes every phone task | **Resolved Nov 13, 2026:** Flutter phones (TR-1); React for web and a small Rust service remain defaults (DU-3, DU-6) | Decided by a person |
| B3 | Credential format, and replay of bearer credentials by a malicious host | The service, host and clients must agree on bytes; and the gap lets one host impersonate a verified user, or an event's host, elsewhere | Ed25519-signed payload naming a device key, plus a signed join proof (DU-31) | I can default this; flagged in Q2 because it changes the design |
| B4 | How TLS and plain HTTP share the address in the QR URL | The URL is `http://ip:port` but apps must use TLS; no second port is given | One port, protocol chosen by the first byte (DU-30) | I can default this |
| B5 | "Byte-identical to the original" versus stripping metadata on client and host | SC1's test cannot be written; step 15 (strip) and step 16 (move unchanged) contradict | "Original" = the file after the location-only strip; host re-hashes if its strip changes bytes (DU-20) | I can default this; **confirm** (Q5) |
| B6 | Integrity re-send has no state transition; same-user duplicate has no row state | The state-table test and the idempotency test cannot both pass | Add `VALIDATING → UPLOADING` for the first two mismatches; duplicates end `FAILED` with reason `duplicate` and a pointer (DU-12, DU-13) | I can default this |
| B7 | What counts as a download, given "nothing about saves is sent to the host" | "Most downloaded", "Hottest" and "Saved by n" cannot be implemented consistently | Explicit save report, counted per photo, logged without the photo ID (DU-19) | I can default this; **confirm** (Q6) |
| B8 | Missing tables and columns (sessions, join token, host account, browser-guest flag, frozen config, original file name) | The schema cannot serve flows 5.2, 5.3 and the PostHog plan as written | Additions listed in digest §6 (DU-34) | I can default this |
| B9 | Five screens and most non-happy-path states have no mockup | UI agents would each invent a different sign-in, error and empty design | One rule set and a table of derived designs (`screen-inventory.md` section D, DU-40) | I can default this; review at M1 exit (Q8) |
| B10 | Method for lossless location stripping in HEIC, and for the client-isolation test | Both are stated as requirements with no method; both are hard | In-place overwrite of GPS data (DU-37); a three-signal heuristic returning ok, isolated or unknown (DU-38). Both are spiked early (M0a-5, M1-5) and checked on real hardware | I can default this; real-world check is a human gate |
| B11 | No way to run the Windows host on the build machine | libheif bundling, firewall, hotspot and mDNS on Windows cannot be verified | macOS first; Windows on a CI runner (Q3) | **Needs a human** |

Lower-impact gaps I defaulted without a question are in the digest's register (§12): session lifetimes for browsers, editable fields, resume after a host restart, hot-score maths, change-log retention, mDNS fields, dark theme, and where unplaced scope lands in the milestones.

## Hands-off blockers

What agents cannot do alone, and which milestones each affects.

| Blocker | Why agents cannot do it | Milestones | Unblocked meanwhile by |
|---|---|---|---|
| Physical iPhone and Android phone on a shared Wi-Fi | Real radios, permission prompts, camera, backgrounding | M0a exit, M0b, M1 exit, M2 exit, M3 exit, M4, M5 | Simulators, emulators and the harness |
| Apple Developer account and signing identity | Needed to install on a real iPhone; credentials must not be handled by agents | M0a exit onward; store builds at M3 | Simulator builds need no signing |
| Windows machine or CI runner | Build machine is macOS | M0a-4, M2-3, M3-10, M3-11, M3-16 | Windows code sits behind two interfaces |
| Real-device sample photos (HDR HEIC, Live Photo, Ultra HDR JPEG) | Cannot be synthesised faithfully | M0a-5, M2-3 | Script-made HEIC, JPEG, PNG fixtures |
| Hosted app service, domain, production signing key | Spending money, creating accounts, holding a secret | M1 exit on devices, M4 | Local service with a development key |
| Email (or SMS) provider for sign-in codes | External account | M1 exit on devices, M4 | Console sender |
| HEVC / libheif licensing decision | Legal judgment | Public Windows distribution after M3 | Internal builds |
| PostHog project and key | External account | M4 dashboards | Schema and filter tests need no key |
| Firewall prompts, Mobile Hotspot, isolated guest Wi-Fi | OS dialogs needing a person or administrator rights; needs a real hostile network | M3-11 | Fakes behind an OS interface |
| macOS notarisation and Windows code signing | Certificates and accounts | M3-16, M4 | Unsigned local builds |
| A gathering of 15 to 30 people | It is the test | M4, M5 | — |
| Role names, product name, bundle IDs, download link | Product judgment | Copy pass and packaging in M3 | String table and placeholders |

## Feasibility by milestone

"Build" is the share of tasks agents can implement. "Verify unattended" is the share whose acceptance can be confirmed by tests an agent runs on the build machine.

| Milestone | Build | Verify unattended | What can be automated | What needs devices, networks or a person |
|---|---|---|---|---|
| S. Setup | 100% | 60% | Skeleton, shared types and their round-trip tests, tokens, script-made fixtures | Installing toolchains, approving the contract, supplying device photos |
| M0a. Host core spike | 100% | 70% | TLS and HTTP listener, 50 MB resumable upload with injected drops, macOS HEIC decode and conversion, location strip, test apps in simulator and emulator | The exit check itself (SC1 and SC4 on real phones over Wi-Fi), everything on Windows |
| M0b. Phone host spike | 100% of the test apps | 10% | That the core starts inside a phone app in a simulator | The whole point: two hours of foreground and background behaviour, battery and prompts on real phones |
| M1. Accounts and events | 100% | 90% | Service and credential tests; every access-type and role combination; sync convergence under drops (property tests); 50 simulated clients; desktop and guest web end-to-end tests in three browser engines; phone UI tests against a local host | Camera scan and Local Network prompt; mDNS between real devices; the exit demo (about 30 minutes) |
| M2. Photo pipeline | 100% | 90% | State table, idempotency, sweeper timings with a fake clock, kill-and-restart at every step, single conversion under concurrent requests, SC2, SC3, SC5, SC6, SC7 on loopback | Timing over real Wi-Fi, Save to Photos prompt, real camera-roll files, Windows decode; an hour to repeat three criteria on phones |
| M3. Lifecycle and hardening | 95% | 75% | Delete, remove, extend cap, end and expiry including offline clients, SC8, rate limits, load at 50 clients and 1,000 photos, sandbox escape attempts | Firewall and hotspot behaviour, an isolated network, signing, the licensing and security sign-offs |
| M4. Field test | Fixes only | 0% of the test | Runbook, log triage script, dashboard definitions, fixing and re-verifying issues | Running the event |
| M5. Phone hosting | 80% | 30% | Packaging the core for phones, phone capability checks with fakes, UI tests | Depends on M0b's go / no-go; battery and backgrounding checks; the repeated field test |

**Reading across the rows:** M1 and M2, which hold 48 of the 92 tasks, can run unattended from an approved contract to their exit demos. M0a and M3 can be built unattended but end in human checks. M0b, M4 and M5 are human-led with agent support.

## What I would do before starting the build

1. Answer Q2 in `open-questions.md` (the contract). Q1 (stack) is decided; Q2 is the only question left that blocks the first tasks.
2. Run task S2 (the toolchain probe) and install what it reports missing.
3. Put a handful of real phone photos into the fixtures folder (S4).
4. Decide Q3 (Windows) before M0a-4; everything else can be answered while M1 runs.

## Method and limits of this review

I read the MVP doc, the PostHog plan, the converted mockup outlines and the relevant parts of the two converted design documents in full. A Sonnet subagent drafted the screen inventory from the mockup outlines and MVP §8; I reviewed it, corrected several entries and added section D. I did not open the original mockup HTML, the flow images, the Word files or the interview-prep notes. I did not check which toolchains are installed on this machine, or the current feature set of the PostHog Rust SDK or of any crate named in the digest; tasks S2 and M0a-9 exist to check those.
