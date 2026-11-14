<!-- Generated from Locale Design Review and System Design Feedback.docx by docx_to_md.py. Do not edit; re-run the script. -->

## Locale: Design Review and System Design Feedback

Oct 6, 2026 · @Hakim

### Rating

The Locale design rates 6.5 out of 10 overall: the submission pipeline is close to build-ready at about 8, while the full system design is held back by missing consistency, non-functional, and read-path work.

Why 6.5. The pipeline logic is sound and unusually well reviewed. Client and server roles are separated, validation is split into admission and content checks, state is server-owned, and failures defer or fail on purpose. What keeps the whole design from an 8 or 9 is what sits around the pipeline: the index has no consistency model, there are no scale or latency targets, the read path does not exist yet, and a few recovery loops can still run forever.

How to read the scale. The overall score is the mean of ten dimension scores (each out of 5), doubled. A 9 or 10 would be a design a team could build from without further questions. A 6 to 7 means the core is right and the remaining gaps are known and listable. Every gap behind this score is already in the design doc's TODO list, which is a good sign.

What reaching 9 takes. Close the blocking items (T1 to T8), add the non-functional requirements and an observability plan, publish the state transition table, design the read path, and add the topology and sequence diagrams.

### Ratings by dimension

The design is strongest in how it is decomposed and weakest in what it says about running at scale and being observed.

[figure]

reviewer scores from the design doc and flow diagrams · 10 dimensions

Each score reflects the design as drawn today, not intent. The table gives the evidence behind each one.

| Dimension | What drives the score |
|---|---|
| Separation of concerns | Client and server lanes, shared subroutines composed correctly, admission versus content validation, emitter and subscriber split |
| Requirements and scoping | Strong PRD with use cases, phases, and named open questions; the pipeline is built ahead of what Phase 1 needs |
| Failure handling | Typed transfer outcomes, bounded retries, deferral to Queued versus terminal Failed; a few loops still lack exits |
| Security model | Server-authoritative validation, scan before strip, re-authorizing token refresh, encryption after validation; key management and node trust unspecified |
| Recovery and self-healing | Sweeper with heartbeat and max attempts, queue worker, outbox consumer, failed cleanup; no singleton, no heartbeat consumer, no outbox limit |
| Documentation clarity | One design doc with glossary, decisions, and TODOs; diagrams are dense and there is no sequence view |
| State management | State legend with reason codes; no transition table, no quarantine state, terminal states described as editable |
| Concurrency and consistency | Atomic lock with bounded retry; table-wide lock, no lease or fencing, no sync semantics or version scheme |
| Non-functional requirements | No file size, duration, concurrency, latency, durability, or cost targets |
| Observability | No correlation IDs, metrics, alerts, or audit log |

### System design principles

Ten principles are applied well in this design and eleven need more work. Each row gives the principle, what it means, and where it shows up (or is missing) in Locale. IDs refer to the TODO list in the design doc.

#### Applied well

| Principle | Definition | Example in Locale |
|---|---|---|
| Separation of concerns | Each component has one job and one reason to change | The client advises and the server decides; admission checks claims before the stream and content validation checks bytes after it |
| Reuse (DRY) | One implementation of a rule, shared by every caller | Access Validation serves both the request gate and the queue worker; Basic Content Precheck serves client and server |
| Fail fast | Reject invalid work at the cheapest point, before spending resources | Rate limit runs first; type, size, and duplicate checks run before any bytes move over Bluetooth or relays |
| Defense in depth | Several independent layers, so one failure does not open the system | Client preflight, server revalidation, malware scan, metadata strip, and re-authorization when the queue worker resumes an item |
| Never trust the client (trust boundaries) | Anything from outside the trusted side is a claim to verify, not a fact | The server never accepts a preflight result as input and recomputes everything; key management and node trust are still open (T7) |
| Graceful degradation | Keep working in a reduced mode when a dependency fails | Wi-Fi falls back to Bluetooth and relays, a failed host falls over to the next ranked host, and an unreachable network keeps the item in the outbox |
| Asynchronous decoupling | Move slow, non-critical work off the path the user waits on | Tag generation and downstream index updates run async, and index processing is enqueued rather than called inline |
| Load awareness | Spread work using current load rather than a fixed assignment | Suitable Host Determination orders by traffic, deprioritizes recently selected hosts and hosts near their data limit, and returns a ranked list |
| Single owner of state | Exactly one component may change a piece of state, so there is one truth | Only the server writes item state; the client sends transfer reports and the server validates the transition |
| Self-healing | The system detects stuck work and repairs it without a person | Sweeper re-enqueues stuck items using heartbeat and max attempts, the queue worker resumes them, and failed cleanup removes residue |

#### Needs more work

| Principle | Definition | Example in Locale | What is left |
|---|---|---|---|
| Idempotency | Repeating an operation has the same effect as doing it once | The duplicate check now has a "submission retry" outcome, so a replay does not flag itself | Let a unique constraint on the submission ID decide, rather than check-then-act (T1) |
| Bounded failure handling | Every wait, retry, and loop has a limit and an exit | Retry Mechanism caps attempts at 3 with exponential backoff | Retry budgets multiply across flows, outbox items never expire, and some loops lack an exhausted-list exit (T3, T12, T17) |
| Concurrency control | Concurrent writers cannot corrupt shared data or block each other forever | Index writes take an atomic lock with a bounded wait and requeue on timeout | One table-wide lock serializes every write, and there is no lease or fencing token; use conditional updates on a version column (T5) |
| State-machine rigor | States, legal transitions, and invariants are written down and enforced | A state legend with reason codes for Queued | No transition table, no quarantine state, and terminal states described as editable (T2, T13) |
| Interface contracts | Callers and callees agree on typed inputs and outputs | Data Stream returns typed outcomes (unauthorized, retriable timeout, transfer complete) | Other subroutines return names callers do not branch on, such as Approved versus Access Validated (T8, T26) |
| Consistency and sync | Replicas converge, and clients can detect and repair gaps | Subscribers compare master and local index versions and fetch only differences | No version scheme, gap detection, or full-resync fallback, and one change has three delivery paths (T15) |
| Transactional integrity | Related writes succeed or fail together, so state and events never diverge | Fan-out was moved off the ack path | Commit Completed and the fan-out event in one transaction with an outbox, then ack (T6) |
| Observability | You can tell from outside whether the system is healthy and why | None drawn | Correlation ID per submission, metrics for queue depth and stuck count, alerts, and an audit log (T21) |
| Capacity planning | Design choices are checked against stated load and limits | A Bluetooth size gate and a large-file async path | No file size, concurrency, latency, durability, or cost targets (T22) |
| Data lifecycle | Data has a defined retention, deletion, and audit trail | Failed State Cleanup deletes partial content | Delete quarantined blobs on hosts, keep a tombstone or audit record, and set retention (T19) |
| Least privilege | Each actor gets only the access its job needs, for as long as it needs it | Session tokens carry a TTL and refresh by re-authorizing with the server | Bind tokens to submission, user, host, and size limit; define the queue worker's identity (T4, T11) |

### What the design needs

Seven fixes would make the pipeline build-ready, and five additions would make the whole design review-ready. IDs refer to the TODO list in the design doc.

| Priority | Need | Why it matters | Ref |
|---|---|---|---|
| Blocking | Idempotent insert through a unique constraint on the submission ID | Concurrent retries can create duplicates or flag a submission as its own duplicate | T1 |
| Blocking | A quarantine state, and an exit for inconclusive scans | Unvalidated bytes must never be visible, and Queued is overloaded today | T2 |
| Blocking | Recovery loops that terminate: claimed entries, attempt counters, singleton sweeper, expiry for abandoned New entries | Prevents endless re-enqueue and double processing | T3, T4 |
| Blocking | Real concurrency control on the index | A table-wide lock serializes all writes, and a crashed holder blocks everyone | T5 |
| Blocking | A transactional outbox | The ack and the fan-out must not diverge from the state write | T6 |
| Blocking | A decision on where bytes land and who holds keys | Scanning, encryption, and the goal of privacy from the cloud all depend on it | T7 |
| Blocking | Typed result contracts | Callers branch on names the callees never return | T8 |
| Next | State transition table | Resolves the terminal-state contradiction and makes every state testable | T13 |
| Next | Non-functional requirements | Lock granularity, fan-out, and host selection cannot be judged without numbers | T22 |
| Next | Observability plan | Stuck items and queue growth are invisible without it | T21 |
| Next | Index sync and a stated consistency model | Clients, replicas, and webhooks can disagree | T15 |
| Next | Read path and host fallback | Half the product is retrieval and none of it is designed | T28, T35 |
| Later | Manual review, host lifecycle, edits and deletes | Needed before real events, not before the first demo | T14, T29 to T31 |
| Later | A Phase 1 vertical slice | The pipeline is built for relays and multi-host failover, but Phase 1 is one phone; a slice would test the core early (not yet in the TODO list) | New |

One decision to make first. The design doc's open question about what the server side is (main host phone, app service, or both) changes who can scan content, where the index lives, and whether the privacy goal holds. Settle it before closing T1 to T8, because several of those fixes read differently depending on the answer.

### Your system design skills

Your strongest skills here are decomposition, designing for failure, and using review well; the main growth area is turning good instincts into explicit invariants and numbers before drawing flows. This is based on one design and the review conversation around it, so it describes this work, not your ability in general.

#### Strengths

- Decomposition and reuse (strong). Access Validation is reused by the request gate and the queue worker, and Basic Content Precheck is shared by client and server. You pulled out Secure Data Transfer Protocol as its own flow when review showed session security was tangled into transport selection.
- Designing for unreliable conditions (strong). The PRD treats weak connectivity as normal, and the design follows through: Bluetooth and relay paths, a ranked host list, resumable transfers, a local outbox, and deferral instead of failure.
- Using review (strong). Between versions you applied feedback quickly and well: typed transfer outcomes, an atomic lock with a bounded wait, async tags and fan-out, a heartbeat in the sweeper. You also pushed back accurately. Pointing out that the index lock does not block file-length uploads corrected my overstatement, and asking what happens to a stuck Queued item if the sweeper skips it exposed a flaw in the rule as I had phrased it.
- Probing boundaries (strong). Questions like whether precheck belongs on both sides, whether metadata can be stripped before scanning, and whether end-to-end is a kind of transport encryption each tested an assumption that mattered, and each led to a better design.
- Product context and scoping (solid). The PRD has use cases, phases, and named open questions, and the TODO list kept the review items visible.

#### Growth areas

- State and invariants first (developing). Many review items trace back to states that were not pinned down: terminal states described as editable, Queued doing three jobs, no quarantine state, no transition table. Before drawing a flow, write the states, the legal transitions, and a few one-line invariants (only the server writes state, nothing is visible before validation). Then check every arrow in every flow against that table.
- Concurrency and consistency (developing). Locking the index was the right instinct. Atomic acquisition, leases, fencing tokens, and the optimistic alternative (a version column with conditional updates) each needed prompting. Good next reading: the chapters on transactions, replication, and consistency in Designing Data-Intensive Applications, plus a refresher on idempotency keys and the transactional outbox.
- Threat modeling up front (developing). You reached sound answers on scanning order, encryption layers, and who holds keys, but through questions. Start from a short list: assets, attacker positions (a modified client, a rogue relay, a revoked user), and what each component trusts. The design then shows where each position is stopped.
- Quantification (developing). There are no sizes, rates, or latency targets, so choices such as table lock versus conditional update, or fan-out cost, cannot be judged. One page is enough: users per event, uploads per minute, file size spread, acceptable latency, availability, and retention.
- Operability (early). Nothing yet says how you would know the system is unhealthy. Add a correlation ID per submission, a handful of metrics (queue depth, stuck count, state age), alerts, and an audit log.
- Diagram craft (solid, improving). The flows are consistent and readable, but they sit on dense canvases with mixed terminology and unlabeled cross-lane arrows. Give each diagram one question to answer, add a sequence view and a topology view, and settle terms in the glossary.
- Scoping and sequencing (developing). The pipeline already handles Phase 3 mechanics (relays, multi-host failover) while Phase 1 is a single phone. A vertical slice for one host would test the core early and show which of the complex parts earn their cost.

A habit that would help most. Across review rounds, the same kinds of defects recurred: loops with no exit, outcome names that did not match what the caller branches on, and enqueue exits that never told the server. Keep a short checklist and run it on every flow before sharing it: does every loop have an exit and a cap, does every outcome have a typed name the caller uses, does every defer send a report, does every state change have an owner.

### What an early senior developer should have

An engineer who has just reached senior level is expected to design a well-bounded system on their own and defend its trade-offs, but not yet to set direction across several teams. This design meets that bar on decomposition, reliability, and communication, and falls short on numbers, operations, and sequencing. Expectations vary by company, so treat the table as a typical profile, not a standard.

#### Expected skills by area

| Area | What is expected at this level | How this design compares |
|---|---|---|
| Requirements and scoping | Clarifies the problem, states constraints and non-goals, and puts numbers on load, size, and latency | Partly: strong PRD with use cases and phases, but no quantified targets |
| Decomposition and interfaces | Draws clean boundaries, names components by responsibility, and defines typed contracts between them | Meets on boundaries; partly on contracts (outcome names still mismatch) |
| Data and state | Models the data, writes down state transitions and invariants, and picks storage and consistency to match access patterns | Partly: states and reason codes exist, no transition table or consistency model |
| Reliability | Plans for failure: timeouts, retries with limits, idempotency, graceful degradation, recovery | Meets: this is the strongest area, with a few loops still lacking exits |
| Security | Draws trust boundaries, separates authentication from authorization, and reasons about attackers | Mostly meets: right model, but reached through questions and key management is open |
| Scalability and performance | Estimates capacity, finds the bottleneck, and uses queues, caching, and partitioning deliberately | Not yet: no estimates, and the table-wide lock is an unexamined bottleneck |
| Operability | Designs for monitoring, alerting, rollout, and debugging in production | Not yet: no observability plan |
| Trade-offs and decisions | Weighs alternatives, records why a choice was made, and says what would change it | Partly: a decisions table records choices and reasons, but not the alternatives rejected |
| Communication | Writes clear docs and diagrams, takes review well, and revises quickly | Meets: iterated through many review rounds and applied feedback |
| Scope and delivery | Sequences work into increments, cuts scope to learn early, and manages risk | Partly: phases exist, but the pipeline is built ahead of Phase 1 |
| Leadership | Reviews others' designs, raises risks early, and mentors | Not visible in a single design doc |

#### More principles to know well

These complement the 21 principles in the earlier section. Each is something an early senior engineer should be able to explain and apply without notes.

| Principle | Definition | Where it applies in Locale |
|---|---|---|
| KISS | Prefer the simplest design that meets the requirement | Phase 1 is one phone, so the first build does not need relay meshes or multi-host failover |
| YAGNI | Do not build for needs that are not yet real | Add the Bluetooth relay path once one-host transfers work and the need is measured |
| Reversible decisions | Spend design effort in proportion to how hard a decision is to undo | Hide encryption and key handling behind one interface so the model can change later |
| Consistency versus availability | When a network splits, choose between answering with possibly stale data and refusing to answer | Index replicas live on phones that drop offline; an offline-first product usually favors availability with eventual consistency |
| Blast radius and bulkheads | Contain a failure so it cannot take down unrelated parts | One slow host should not stall other uploads, and the main node is both coordinator and fallback host |
| Backpressure | When a consumer is overloaded, slow or shed producers instead of queuing forever | Fan-out grows with attendees times submissions, so cap queue depth and batch updates |
| Versioning and compatibility | Changes must keep working with older clients and data | Phones run different app versions, so the index diff and transfer protocol need version fields |
| Design for testability | Build so each behavior can be tested, including failures | The transition table yields test cases, and Bluetooth drops can be injected to test resume and failover |

### Next steps

Answer the server-side question first, then close the blocking items in dependency order, then fill the gaps that stop the design from being reviewable.

- Decide what the server side is, who holds the bytes while an item is Queued, and where bytes land before validation.
- Write the state transition table with invariants, add the quarantine state, and give Queued reason codes their own handling (T2, T13).
- Make the insert idempotent with a unique constraint, then update the sweeper and queue worker so every loop has a claim, an attempt cap, and an exit (T1, T3, T4).
- Replace the table lock with conditional updates, add the transactional outbox, and move to typed results (T5, T6, T8).
- Write the one-page non-functional requirements and an observability plan (T21, T22).
- Add the sequence diagram, then design the read path and host fallback (T25, T28, T35).
- Run the loop, outcome, defer, and ownership checklist on every flow before the next review.

After step 4, ask for another review pass. The pipeline should then land at about 8.5 out of 10, and the remaining distance to 9 is documentation and the flows still to design.
