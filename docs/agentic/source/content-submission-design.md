<!-- Generated from Locale Content Submission System Design.docx by docx_to_md.py. Do not edit; re-run the script. -->

## Locale: Content Submission System Design

Oct 6, 2026 · @Hakim

### Overview

Locale turns phones into temporary servers so events can host forums and full-quality files without the cloud. This doc captures the current design of the content submission pipeline (a user uploading content to an event), its supporting flows, and the work still open.

Mission. Temporary peer-to-peer data connection and file hosting for file transfer, communication, forums, and location. Think of a locally hosted, temporary Discord where each server is an event page.

Problems it solves

- No quality loss: content is stored on local hosts instead of being compressed for cloud storage
- Privacy from the cloud: event data lives on the host device
  - Puts privacy in the hands of the people and trusted participants
- Easy sharing across Apple, Android, and computers
- Not everything needs to be permanent or on the internet
- Staying connected without Wi-Fi or service (festivals, hiking, offline settings)

Scope of this doc

| Covered | Placeholder only (see Other flows to design) |
|---|---|
| Submission flow (client and server lanes) | Read and retrieval path |
| Validation, host selection, data stream, secure transfer | Host initialization and decommission |
| Retry, index access, downstream index update | Manual review, edits, deletes, retention |
| Recovery: stuck sweeper, queue worker, outbox, failed cleanup | Sync protocol, flagged content processor |

Reading order. Architecture and the state model first, then the submission flow, then the subroutines it calls. TODO IDs (T1, T2, ...) are listed in the last sections and referenced throughout.

### Use cases and phasing

The product targets in-person events first, with unreliable connectivity as the normal case. Streaming and non-local use come later.

| Use case | What it needs from the system |
|---|---|
| Parties and event hosting | Event pages, photo and video sharing, chat, feed |
| Festivals, markets, fairs | Works with poor reception, vendor listings and pricing, finding people and stands |
| Hiking and offline scenarios | Bluetooth and relay transfer, location finding without service |
| Productivity and work groups | Open hub for file sharing, shared docs, to-do lists, notes boards |
| Academic classrooms | Access limited to edu accounts |
| Local event finding | Interest and hobby groups, trivia nights, impromptu sessions |

Access model (from the PRD). Only verified or permanent profiles can upload, which supports traceability and limits abuse and minors' exposure. Temporary and guest accounts can view and download. Events can be invite-only, password-protected, request-and-approve, proximity-only, same-network, or Bluetooth-only.

Hosting stages

- Phase 1: one phone acts as the data host
- Phase 2: multiple fixed phones host
- Phase 3: multiple hosts that can drop and switch

Functionality stages

- Phase 1: user profiles, event listings, image sharing, other static content
- Phase 2: private chat, feed and threads, live content, text chat
- Phase 3: video and audio streams

The submission flow in this doc is built for the multi-host case (Phase 2 and 3) but degrades to a single main host: when only one host exists, host determination is skipped and the main host is used.

### Architecture

Content lives on host phones, a main node coordinates through an index table, and clients render from a local copy of that index instead of polling hosts. The server-authoritative pieces are the admission checks, content validation, and every state transition.

Authorized Clients can act as hosts, since Hosts are users in the network as well.

Roles

| Role | Responsibility |
|---|---|
| Client | Selects media for upload/download, runs the light precheck, holds the file in a local outbox while offline, streams bytes to a host, reports transfer results, download data from hosts, submits texts/messages |
| Main node | Owns the host ID table and index table, handshakes with clients, default host when only one exists |
| Host nodes | Store large and dynamic data, receive the data stream, hold backups as read-only copies |
| Service provider (app) | User profiles and account data, larger events such as fairs and vendors |
| Subscribers | Active clients with a local index, updated by push, webhook, or poll |

Where data lives (from the PRD)

| Data | Stored on |
|---|---|
| Images, video, large files | Host devices |
| Polls, threads, other dynamic data | Host devices (single source of truth) |
| Ledgers, Event passwords, event details, sensitive data | Host devices |
| Private messages | Users |
| Profiles and related user data | Service provider |

Index table. Records where each object lives (pointer to host), its backups, a linked list of users who downloaded it (potential backup nodes), view and download history, content previews, original uploader, content type, and tags. Writes to source table are locked, any updates fan out as read-only copies to auxiliary nodes, and clients that open the app pull the current index version for content/feed rendering.

Control plane and data plane. Control traffic (submit request, admission, host list, transfer reports, state changes) goes between client and the server side. Data traffic (the file bytes) goes from the client to a host directly, over Wi-Fi, Bluetooth, or relay nodes. Keeping these separate is why validation is split into admission before the stream and content validation after it.

[figure]

control plane and data plane · 5 components

Client and server side exchange control messages, the file bytes go from the client to a host, and the index table tells subscribers where each item lives.

Known architectural gaps

- The main node is both control plane and fallback host, which makes it a single point of failure (T20)
- "Server side" in the flow diagrams is not yet mapped to a concrete component: main host phone, app service, or both (see Open questions)
- A happy-path sequence diagram is missing, and the diagram above omits relays, replicas, and the host registry (T25)

### Item state model

Every submission has one index entry, and the server is the only writer of its state. Clients send reports and requests; the server validates each transition.

States as drawn today

| State | Group | Meaning | Reason codes |
|---|---|---|---|
| New | Processing | Entry created after user validation, before upload is authorized | None |
| In Progress | Processing | Upload authorized, bytes expected or flowing | None |
| In Review | Needs manual intervention | Held for a human decision | Likely duplicate |
| Queued | Enqueued | Waiting to be retried | Waiting for connection, waiting for scan retry, lock contention |
| Completed / Uploaded | Terminal | Validated, encrypted, and indexed | None |
| Failed | Terminal | Rejected or retries exhausted | Per failure path (access, validation, integrity, auth) |

Stuck items. An item in New or In Progress for an extended time is considered stuck, but large uploads may still be running. A stuck check must therefore look at an upload heartbeat or last-progress time, not only time in state. A Queued item that was previously stuck or never fully uploaded can also get stuck.

Transitions as drawn in the flows

| From | To | Trigger |
|---|---|---|
| (none) | New | Submit request passes user validation |
| New | In Review | Duplicate check returns likely duplicate |
| New | Failed | Basic content precheck denied |
| New | Queued | No host connections available |
| New | In Progress | Upload authorized with ranked host list |
| In Progress | Queued | Transfer report: could not upload or connection timeout; or content validation could not complete |
| In Progress | Failed | Unauthorized transfer; integrity check retries exceeded; content validation denied |
| In Progress | Completed | Content validation approved, encrypted, index updated |
| Queued | Failed | Max retry attempts exceeded (stuck sweeper) |
| Queued | In Progress | Queue worker or outbox resubmits through the submission flow (entry point to be defined, T4) |

Open items on the model

- Add a quarantine state (for example Validating) between transfer finished and Completed, invisible to other users and excluded from fan-out (T2)
- The legend says terminal states can still be updated. Make the payload immutable and only metadata mutable, with a published transition table (T13)
- Add failure reason codes marked retryable or not, and an Expired outcome for the outbox time limit (T13)
- Consider renaming In Progress to Upload Authorized so it matches what the transition actually means (T13)

### Submission flow

The client advises and the server decides. Cheap checks on claims run before any bytes move (admission), and byte-level checks run after the bytes arrive (content validation). Only the server writes index state.

Client lane

- User selects media and suggested tags.
- Client Content Precheck runs locally: type, size, strip metadata. A denial rejects the submission with a constraints error and never creates server state.
- Connection Validation on Main Node. If it fails, Retry Mechanism runs; when retries are exhausted the item stays in the local outbox as "waiting to upload" and the Client Outbox Consumer picks it up later.
- Create the payload and send the Submit API Request.

Server lane: admission

- User Validation: rate limit first, then session, then Access Validation. Failures return a rate-limit or auth error.
- Insert the initial index entry as New.
- Duplicate Idempotency Check. Likely duplicate: set In Review and return a duplicate error requiring manual review. Unique or a retry of the same submission: continue.
- Basic Content Precheck on the claimed type and size. Denied: set Failed and reject.
- Host selection. With more than one host, Suitable Host Determination returns a ranked list; with one, default to the main host. If no host connections are available, set Queued and tell the client the upload is waiting for a connection.
- Set In Progress and return the ranked host list plus an upload authorization to the client.

Client lane: transfer

- Large media goes async (the user is notified and can leave while the upload continues in the background); everything else runs inline.
- Data Stream sends the bytes. Its typed outcomes come back to the server as transfer reports:
  - Transfer complete, returning a transfer report
  - Content could not be uploaded, or connection timeout: set Queued
  - Unauthorized connection: set Failed and return an auth error

Server lane: content validation

- Validate integrity and checksum. Invalid: Retry Mechanism; retries exceeded sets Failed.
- Server Content Validation. Denied: set Failed. Unable to complete (for example an inconclusive scan): set Queued.
- On approval, generate content tags (async), encrypt the content at rest, and set Completed.
- Return the completion acknowledgement to the client, and run Downstream Index Update asynchronously so fan-out never delays the ack.

Design rules this flow follows

- Admission before stream: invalid submissions cost no bandwidth, which matters most over Bluetooth and relays
- Integrity before validation: a scan result is only meaningful for the exact bytes sent
- Encryption at rest after validation, because scanning needs plaintext
- Retryable failures defer to Queued; non-retryable failures go to Failed

Known gaps in this flow

- The duplicate check should rely on a unique constraint on the submission ID rather than check-then-act (T1)
- No quarantine state between transfer finished and Completed (T2)
- The ack and the COMPLETED write should commit with an outbox event in one transaction (T6)
- Server lane steps and the transfer reports need defined request and response contracts (T26)

### Subroutines

The flows are split into reusable subroutines, grouped as validation, core mechanisms, and data movement. Each is documented here as it is drawn today, with open items called out.

#### Validation

##### Access Validation

Checks that the account may act on this event.

- Is the account registered? No: set Failed and throw an access error.
- Does it have the proper event roles? No: set Failed and throw an access error.
- Return Access Validated.

##### User Validation

Entry gate on the server for every submit request.

- Rate limit check. Limited: return a rate-limit error.
- Valid session? No: prompt login.
- Run Access Validation. Denied: return an access error.
- Continue in the submission flow.

##### Connection Validation on Main Node

Confirms the client can reach the main node and keeps the ID table current.

- Commence the handshake.
- Can the host be accessed? No: retry, else throw a 504.
- Query the host ID table.
- Any differences in the ID table? Yes: enqueue Index Update Processing so this flow does not wait on it.
- Return the successful connection validation result.

##### Client Content Precheck

Light check on the device before any server contact. Advisory only.

- Run Basic Content Precheck. A constraint error goes back to the caller.
- Strip media metadata.
- Return approval.

##### Basic Content Precheck

Claims-based check shared by the client and the server.

- Determine the file type. Not acceptable: throw a file type constraint.
- Within the size limit? No: throw a file size constraint.
- Return approval.

##### Duplicate Idempotency Check

Decides whether this is the same submission again or the same content submitted anew.

- Compare the content against the index, excluding the submission's own ID.
- Unique, or a retry of the same submission: continue.
- Likely duplicate: throw a duplicate error and require manual review (In Review in the main flow).

Open: let a unique constraint on the submission ID arbitrate instead of check-then-act (T1).

##### Server Content Validation

Authoritative validation of the bytes that arrived.

- Run Basic Content Precheck.
- Run Duplicate Idempotency Check. Likely duplicate: throw a duplicate error and require manual review.
- Scan for security concerns or malicious content. Content flagged: throw a validation error.
- Scan timeout: Retry Mechanism, then scan again. Inconclusive result: set Queued and submit to the queue.
- Scan successful: strip media metadata.
- Return approval.

The scan runs on the raw bytes before the strip, because stripping parses untrusted content. Open: run the strip as a sandboxed re-encode with a fail-closed branch, and define which form the duplicate hash covers (T16).

#### Host selection

##### Suitable Host Determination

Load-balancing style ranking of hosts for an upload.

- Grab the current host pool.
- Can the hosts be accessed? No: retry and validate which hosts are available. Any nodes available for hosting? No: throw an error.
- Initiate host trait analysis.
- Order hosts by current traffic load.
- Deprioritize recently selected hosts (traffic may be ramping), then hosts reaching their data limit.
- Consider special designations: host capabilities, data preferences, proximity.
- Return the top 3 ranked hosts, so a dropped host has a fallback.

#### Data movement

##### Data Stream

Picks the transport and keeps the transfer alive across drops and host failures.

- Wi-Fi enabled, a connection available, and on the same network? Yes: go to Secure Data Transfer. Different network: continue only if the session allows out-of-network connections.
- Otherwise attempt a peer-to-peer connection over Bluetooth. File too large for Bluetooth: enqueue for upload over Wi-Fi.
- Is the connection close or strong enough? Yes: go to Secure Data Transfer.
- No: are there enough active nodes to relay? Yes: create and validate a connection map, then go to Secure Data Transfer. No: enqueue for future upload.
- Handle how Secure Data Transfer ends:
  - Unauthorized connection: fail the transfer.
  - Time limit exceeded: fail with a retriable timeout.
  - Session ended successfully: return the transfer report.
  - Session disconnected: run Retry Mechanism. If the host is still valid and a retry is allowed, query the host for its offset and continue from the last valid byte. If retries on the host are exhausted, invalidate the partial transfer and select the next host from the ranked list, then reassess the transport. When the host list is exhausted, enqueue for future upload and return a deferred report.

##### Secure Data Transfer Protocol

Authenticates the session and moves the bytes under an expiring token.

- Establish a connection with the node, then authenticate the node and client. Not authenticated: throw an authorization error.
- Estimate the transfer time and start the heartbeat.
- Provide a submission session auth token with a TTL.
- Start the transfer over an end-to-end encrypted session.
- Loop until the transfer is complete:
  - Interrupted with no progress for X seconds: run Retry Mechanism. Unable to re-establish: end the session as disconnected. Connection restored: continue.
  - Session auth no longer valid: if the refresh limit is exceeded, end the session as time limit exceeded; otherwise re-authorize with the server and get a new token and TTL.
- Complete: end the heartbeat and end the session.

##### Retry Mechanism

Shared retry rule used by the flows above.

- Has the max retry attempts (3) been exceeded? Yes: fail the attempt.
- Otherwise perform an exponential backoff.
- Attempt the routine again.

#### Index and notification

##### Index Table Accessing

Only the index table needs locks; data is indexed before writes happen to other tables.

- Is this a read? Yes: perform the read, no lock needed.
- Otherwise obtain the lock atomically. If it is held, retry with a bounded timeout.
- Perform the write. It touches only the index table, so it should be quick.
- Is the write complete? Yes: release the lock. No: if the lock time limit is exceeded, abort the transaction, set Queued, and submit to the queue; otherwise check the write state again.

##### Downstream Index Update (emitter)

Tells subscribers that the index changed.

- An event updates the index table.
- Get the current pool of nodes and open connections.
- Push a live update to local index tables for active connections.
- Send an event webhook for other users as a background process.
- Enqueue Index Update Processing.

##### Index Update Processing (subscriber)

Brings a client's local index up to date with the least data.

- The client receives the index update notification.
- Grab the current master index version and the local version.
- Find the differences and send only the necessary updates.
- Validate that the content matches.
- Enqueue the update to the client feed.
- Update completed.

Open items on subroutines

- Index locking: add a lease with TTL and fencing token, or replace the table lock with conditional row updates; no flow calls Index Table Accessing yet (T5)
- Data Stream thresholds and failover: define "close or strong enough", "enough active nodes", the large-file cutoff for Bluetooth, and chunk size; confirm the retry-invalid path reaches failover before enqueue (T9, T10)
- Session tokens should be bound to submission, user, host, and size limit, with a new token per host (T11)
- Retry budgets across subroutines need one policy (T12); the PRD's fixed 3 second backoff conflicts with the exponential backoff drawn now
- Callers branch on names the callees do not return (Approved, Granted, Access Validated). Move to one typed result everywhere (T8)
- Index sync: version scheme, gap handling, idempotent apply; one change currently has three delivery paths (T15)

### Recovery and queue logic

Four background flows keep stuck, deferred, and failed submissions from lingering. They are drawn, but their rules have not all caught up with the main flow.

##### Resolving Stuck Submissions

Scheduled check, every X minutes, that finds submissions that stopped moving.

- Check the index table.
- Select entries in a processing state (New, In Progress) and entries in Queued for more than 5 minutes. None: no items are stuck, stop.
- For processing entries: have they been in that state for more than 2 minutes? No: leave the entry alone to be reassessed on the next run.
- Has the entry exceeded the max retry attempts? Yes: set Failed.
- Is it flagged as a large data set? No: set Queued and send it to the queue for reprocessing.
- Yes: is the upload heartbeat still happening? Yes: leave the entry alone to be reassessed later. No: set Queued and send it to the queue for reprocessing.

Open: claim entries so only one actor handles each, run as a singleton, and expire abandoned New entries (T3); nothing on the server receives the heartbeat yet (T18).

##### Submission Queue Processing

Drains the queue and resumes valid submissions, on a separate thread from the submission flow's setup.

- Pick up a message from the queue.
- Is the message context still available? No: remove it from the queue, then check for context on the message origin. Origin known: send a notification to the source to initiate a retry. Origin unknown: update state to failed.
- Has the message already been processed? Yes: drop the message, to prevent duplicates.
- Invalidate any stale submission data.
- Run Access Validation, so a user whose access was revoked while waiting is stopped. Denied: throw an access error.
- Granted: go through the submission flow.

Open: a denied access check should set Failed, the re-entry point and the identity used to re-authorize need defining, and notifications to the source need a cap (T4).

##### Client Outbox Consumer

Scheduled check on the client for API requests held while offline.

- Check the outbox.
- Are any API requests waiting? No: nothing to submit.
- Run Connection Validation on Main Node. No connection: leave the request in the outbox.
- Valid connection: submit the API request.

Open: an attempt cap, a time limit with an Expired outcome, and triggers beyond the schedule such as connectivity change and app foreground (T17).

##### Failed State Cleanup

Scheduled check on the index table for failed items and their leftovers.

- Check the index table.
- Are any entries in the Failed state? No: nothing to clean up.
- Is the entry within the look-back window? No: nothing to clean up.
- Is there any residual content from the item's upload attempt? No: nothing to clean up.
- Delete the failed partial content.

Open: also delete quarantined blobs on hosts, keep a tombstone or audit record, and set a retention period (T19).

Gaps to close

- Sweeper should claim entries so only one actor handles an entry, expire abandoned New entries, and not re-enqueue entries that already have a recent outstanding message (T3)
- Queue worker: a denied access check throws an error but never sets Failed, so the sweeper retries the entry again; define the re-entry point, who holds the bytes while Queued, and which identity the worker uses to re-authorize (T4)
- Outbox needs a cap on attempts and a time limit, with an Expired outcome and a user-visible state (T17)
- Nothing on the server receives the transfer heartbeat yet, so the sweeper cannot actually use it (T18)
- Failed cleanup should also delete quarantined blobs on hosts, keep a tombstone or audit record, and apply a retention period; index the sweeper query on state and updated time (T19)
- Add a flagged content queue and processor (T33)

### Other flows to design

These flows are not designed yet. Each has a template below, built from the patterns the current flows use (user validation, index reads and writes through Index Table Accessing, Data Stream, Retry Mechanism, server-owned state, Downstream Index Update). Everything in a template is a proposal until it is marked decided, and each ends with the decisions still open.

| ID | Flow | What exists today | Status |
|---|---|---|---|
| T28 | Read and retrieval path | Note outline | Template drafted |
| T29 | Host node initialization | Capability checklist in notes | Template drafted |
| T30 | Host node decommission | Heading only | Template drafted |
| T31 | Edit, delete, invalidate, cancel, expire | Legend says terminal entries can be invalidated or repointed; retention settings in the PRD | Template drafted |
| T14 | Manual review (In Review) | State exists, nothing resolves it | Template drafted |
| T32 | Index sync protocol | Version diff in the subscriber flow only | Template drafted |
| T33 | Flagged content queue and processor | Listed in TODOs; inconclusive scans already set Queued | Template drafted |
| T34 | Join session by NFC tap | One line in TODOs | Template drafted |
| T35 | Item fallback when a host goes down | Note outline | Template drafted |
| T36 | User content rendering and messaging sockets | Notes only | Template drafted |

#### Template flows

##### T28 Read and retrieval path (template)

Gets an item to a client from the host named in the index, with integrity checked.

- The user selects an item.
- Run User Validation (rate limit, session, Access Validation). Guest accounts may read and download.
- Run Connection Validation on Main Node and sync the local index if the versions differ.
- Look up the item in the index table. This is a read, so no lock is needed. Not found, or not Completed: return a not-available result.
- Verify the entry: the content has not been altered or replaced, by comparing the stored hash.
- Record the read in the index history and add the client as a potential backup source. Do this asynchronously through Index Table Accessing so it never delays the download.
- Run Data Stream from the host in the entry. If the host fails, fall back to the next backup (T35).
- Validate integrity and checksum of the downloaded bytes. Invalid: Retry Mechanism, then fail the read.
- Decrypt the item and return it.

To decide: how keys reach the reader and whether the host also authorizes reads (T7), and whether history writes should ever block.

##### T29 Host node initialization (template)

Admits a phone into the host pool only if it can do the job.

- The user opts in to hosting and chooses how much storage to allocate.
- Check capabilities: permissions enabled, power and battery, connection test, processing power meets the minimum, space meets the minimum, and proximity to the network or other hosts.
- Does any check fail? Yes: refuse with the reason and do not register.
- Authenticate the node with the main node and issue its credentials (T7).
- Register the host in the host pool with its capacity and traits, so Suitable Host Determination can rank it.
- Update the host ID table and enqueue Index Update Processing so other nodes learn about it.
- Start the heartbeat so availability is tracked.

To decide: minimums for battery, processing, and space, and how host credentials are issued and revoked.

##### T30 Host node decommission (template)

Removes a host without losing the items it holds.

- Trigger: the user stops hosting, the battery or space falls below the minimum, or the heartbeat is lost.
- Mark the host as draining in the host pool, so Suitable Host Determination skips it.
- For each item whose primary is this host: does a valid backup exist? Yes: promote it through Item Fallback (T35). No: invalidate the entry and notify the original uploader to re-upload.
- For each item that has this host only as a backup: schedule a new backup copy elsewhere.
- When the host holds nothing the system still needs, remove it from the host ID table and enqueue Index Update Processing.
- Revoke its credentials and end its heartbeat.

To decide: how long a planned drain may take, and what happens to items on a host that disappears with no warning.

##### T31 Edit, delete, invalidate, cancel, expire (template)

Changes or ends an item after submission. The payload stays immutable and only metadata is editable (T13).

- The client sends a request: edit metadata, delete, replace, or cancel. Expiry runs the same path from a scheduled job when a retention time passes (for example a session set to delete after 24 hours, or a single notice after 60 minutes).
- Run User Validation, then check the user is the uploader or holds a host or moderator role for the event.
- Check the item state. Cancel is allowed in New, In Progress, or Queued: abort the transfer and set Failed with a cancelled reason. Edit and delete apply once the item is Completed.
- Edit metadata: write through Index Table Accessing, bump the version, and run Downstream Index Update.
- Delete or expire: write a tombstone in the index, enqueue deletion on the host and its backups, keep an audit record, and run Downstream Index Update so clients remove it.
- Replace payload: treat it as a new submission linked to the old entry, and invalidate the old entry only after the new one completes.
- Return the acknowledgement.

To decide: who may delete, how long tombstones and audit records are kept, and whether expiry can be set per object as well as per session.

##### T14 Manual review (template)

Resolves items held In Review, such as a likely duplicate.

- The entry is set In Review with a reason code, and the content stays in quarantine.
- Add a review task and notify the reviewer. Return the likely original alongside the held item.
- The reviewer decides:
  - Approve: continue the submission flow from host selection.
  - Reject: set Failed with a rejected reason and run cleanup.
  - Keep both, when the duplicate was intended: continue and record the link to the original.
- No decision within the time limit: set Failed with an expired reason and run cleanup.
- Write an audit record of the decision and who made it.
- Tell the submitter the result.

To decide: who reviews (the submitter, the event host, or a moderator), the time limit, and what the submitter sees while waiting.

##### T32 Index sync protocol (template)

Keeps each client's local index current without polling hosts for every item.

- A client opens the app or reconnects. Run Connection Validation on Main Node.
- The client sends its local index version.
- The server compares it with the master version. Equal: done.
- Is the gap small and the history still available? Yes: send only the changes since that version, in order, so applying them twice has no extra effect. No, or the version is unknown: send a full snapshot.
- The client applies the changes, validates that content matches, and stores the new version.
- While connected, Downstream Index Update pushes each change with its version number. If the client sees a missing version, it goes back to step 2.
- The webhook and queued paths carry the same versioned update, and the client drops any version it already applied.
- Invalidated items travel as tombstones so clients remove them.

To decide: the version scheme, how long the change history is kept, precedence between push, webhook, and queued delivery, and what "content matches" compares.

##### T33 Flagged content queue and processor (template)

Handles content held by an inconclusive scan, a user report, or a later detection.

- Source: Server Content Validation returns an inconclusive scan, a user reports an item, or a later scan flags a Completed item.
- Set the entry to the quarantine state (T2), keep the bytes in quarantine, and enqueue a message with the reason.
- The processor picks up the message. Is the context still available? No: remove it. Already processed? Yes: drop it.
- Re-run the scan with a deeper engine or updated signatures. A timeout goes through Retry Mechanism.
- Clean: release the item and continue the submission flow from encryption and completion.
- Flagged: set Failed, delete the blob, and notify the submitter.
- Still inconclusive after the max attempts: set In Review for a person (T14).
- Write an audit record of the outcome.

To decide: the scan engine and its time limits, how users report content, and how a takedown works for an item that is already visible.

##### T34 Join session by NFC tap (template)

Uses a tap only to exchange a join credential. The transfer then continues over Bluetooth or Wi-Fi, because NFC range is a few centimeters and its throughput is low.

- The host opens tap-to-join for the event, which creates a short-lived, single-use join token scoped to that event.
- The guest taps phones. NFC passes the token and connection parameters.
- The guest's app sends the token to the main node. Run User Validation (guest accounts allowed) and the event's access rules through Access Validation (invite-only, password, proximity, or approval).
- Verify the token and mark it used. Invalid or expired: refuse.
- Establish a Bluetooth or Wi-Fi session through Secure Data Transfer Protocol, which authenticates node and client.
- Sync the index (T32) and add the guest to the event roster.

To decide: platform limits on NFC peer-to-peer (iOS restricts it for third-party apps), the token's lifetime, and whether the tap can also help authenticate relay nodes (T7).

##### T35 Item fallback (template)

Keeps an item readable when its host fails or its content is compromised.

- Trigger: the host is unreachable during a read, its heartbeat is lost, or an integrity check fails.
- Check the index for backups of the item.
- Is there a valid backup? Yes: verify its hash, pop it from the linked list of backups, repoint the item to that node through Index Table Accessing, run Downstream Index Update, and retry the read there.
- No backup: if the original uploader's device still holds the file, use the stored path to the original. If it is inaccessible, invalidate the entry and prompt the uploader to re-upload, showing the cached compressed preview so they know which item it is.
- Schedule a new backup to restore the backup count.

To decide: the target backup count, when a viewer becomes a backup (on view or on download), and whether read-only copies update on every write or on an interval.

##### T36 User content rendering and messaging sockets (template)

Renders from the local index and uses open sockets for live content, so hosts are not polled per item.

- The client opens an event page. Run Connection Validation on Main Node and receive the current index table and version (T32).
- Render content from the local index, using cached previews. Do not poll hosts per item.
- Open a socket for live updates to threads, polls, and chat.
- On an event, the host commits it to its database, then pushes it to open sockets and runs Downstream Index Update.
- Messaging: a message goes over the open socket. Thread content is stored on the host, and private messages stay on users' devices.
- If the socket drops, reconnect through Retry Mechanism. On reconnect, run the sync (T32) to fill any gap.
- Fetch the full-quality original on demand through the retrieval path (T28).

To decide: message ordering, delivery of private messages to an offline user, and how threads and comments are stored (the notes suggest following how social media stores posts).

### Security and trust model

The client is advisory and the server is authoritative: nothing the server trusts depends on a client check having run. Client-side checks save bandwidth and give fast feedback, and the server re-derives everything that protects the system.

| Layer | Control in the design | Status |
|---|---|---|
| Identity | Session and role checks via User Validation and Access Validation, rate limit first | Drawn |
| Admission | Claims-based checks before any bytes move: duplicate check, type and size allowlist | Drawn |
| Content | Server Content Validation after upload: integrity, scan raw bytes, then strip metadata | Drawn; sandboxed strip and strip failure branch open (T16) |
| Transport | End-to-end session encryption from client to ingest, so relays carry only ciphertext | Labeled; key setup and node authentication open (T7) |
| Session | Submission session auth token with TTL, bounded refresh that re-authorizes with the server | Drawn; binding to submission, user, host, and size limit open (T11) |
| At rest | Content encrypted after validation with server-managed keys | Step drawn; key management open (T7) |
| Authorization on replay | Queue worker re-runs Access Validation before resubmitting | Drawn; worker identity open (T4) |
| Abuse | User rate limit, duplicate detection | Drawn; per-user storage and bandwidth quotas open (T22) |

Server-managed versus end-to-end content encryption. The design uses end-to-end encryption for the transfer session and server-managed keys for content at rest, because scanning needs plaintext. This means the platform can read content. The PRD also lists "data privacy from the cloud" as a goal, so the meaning of "platform" here (host phone, app service, or both) must be settled; see Open questions.

Rules the flows depend on

- Never scan after stripping: stripping parses untrusted bytes, so scan the raw bytes first and run the strip sandboxed
- A client preflight result is never an input to the server; the server re-runs its own checks
- Do not publish Completed until validation passes, and keep unvalidated bytes in a non-served quarantine location (T2)
- A checksum detects corruption, not tampering by a relay; relayed data needs authenticated encryption or a keyed digest delivered by the control plane
- Strip location metadata so GPS never leaves the device
- Keep an audit log of denials and manual reviews (T21)

### Design decisions

These choices are reflected in the current diagrams. Add a row whenever a decision changes.

| Decision | Choice | Why |
|---|---|---|
| Who validates content | Client runs a light preflight; server re-validates everything and is authoritative | Client checks can be bypassed; they only save bandwidth and give fast feedback |
| When server validation runs | Admission (claims only) before the stream; content validation after the bytes arrive | Rejects bad submissions cheaply while still validating what was actually received |
| Check order after upload | Integrity first, then content validation | A scan verdict only means something for the exact bytes sent; corruption is retryable, malice is not |
| Scan versus strip | Scan raw bytes, then strip metadata | Stripping parses untrusted content, so it must not run first |
| Encryption | End-to-end session encryption in transit; server-managed encryption at rest after validation | Scanning needs plaintext; relays must only see ciphertext |
| State ownership | Only the server writes index state; clients send transfer reports | Clients can be stale or modified |
| Client offline behavior | Keep the item in a local outbox, not a server state | The client cannot write to a server it cannot reach |
| Connectivity failure | Retry, fail over down the ranked host list, then defer to Queued | Flaky connectivity is the normal case |
| Host selection output | Ranked list of the top 3 hosts | A dropped host has a fallback without re-running selection |
| Duplicate versus retry | One check with a unique-or-submission-retry outcome | A replay must not flag itself as a duplicate |
| Rate limit placement | First step of User Validation | Cheapest check, runs before access lookups |
| Tags and fan-out | Generated and published asynchronously | Neither should delay the ack |
| Retry backoff | Exponential, capped at max retries | Fixed delays stack under load (PRD value of 3 seconds superseded; confirm in T12) |
| Index writes | Atomic lock with bounded wait, abort to Queued on timeout | Short critical section; contention should not fail the submission |
| Hosting dependency | Content on hosts, local index table on clients | Spreads load and avoids constant polling of the host |

### Open questions

The first three decide how much of the flows would change, so they come first.

- ☐ What is the "server side" lane: the main host phone, an app service, or both? This decides who can scan content and where the index lives, and whether "privacy from the cloud" holds.
- ☐ Who holds the bytes while an item is Queued: the client (worker only notifies it to resume, needs a durable outbox that survives app kill) or the server (needs staging storage and its own resume point at In Progress)?
- ☐ Where do bytes land before validation when the client streams straight to a host, and does that host run the scan or hand off to a scanner?
- ☐ Does the product promise that the platform cannot read user content? If yes, server-side scanning is dropped and controls move to the client and recipients.
- ☐ Who staffs manual review, what is the time limit, and what does the user see while waiting?
- ☐ What consistency does the index promise (single writer, replicas, how stale can a client be)?
- ☐ What file size and upload duration targets does the design assume, and what is the large-file cutoff for async and for Bluetooth?
- ☐ Should all hosting depend on the host device, or should the app service hold a placeholder for event creation and sharing (PRD question)?
- ☐ Should content only be postable by people within proximity of the event?
- ☐ Do Retry Mechanism calls across flows share one budget, or does each layer have its own?
- ☐ What does "Flag submission" persist and who consumes it?
- ☐ Are "index table", "host ID table", "core ID table", and "ID directory" the same thing? (see Glossary)

### TODOs and action items

Items are grouped by what happens if they are built as drawn. IDs (T1, T2, ...) are referenced from the sections above. Blocking items would produce wrong behavior in production.

Resolved since the last review. Failure paths now set state; heartbeat check in the sweeper; max retry attempts in the sweeper; exclude own ID in the duplicate check; atomic lock acquisition with bounded retry; async tags and downstream update; typed Data Stream outcomes; Secure Data Transfer Protocol with bounded token refresh; QUEUED reason codes; subroutines moved to their own page; rate limit moved first.

#### Blocking

- ☐ T1 Let a unique constraint on the submission ID arbitrate the index insert (conflict means a retry, return the existing result); same content hash with a different ID goes to review
- ☐ T2 Add a quarantine state (for example Validating) between transfer finished and Completed; keep it invisible and out of fan-out; give inconclusive scans a state with its own attempt count and time limit
- ☐ T3 Sweeper: claim entries so only one actor handles each, expire abandoned New entries, re-enqueue Queued entries only after a redelivery window, add an attempt counter on the entry, run as a singleton
- ☐ T4 Queue worker: set Failed on denied access, define the re-entry point and who holds the bytes, and the identity used to re-authorize (service identity acting for the submitter); clean up blobs on the no-context path
- ☐ T5 Index locking: lease with TTL and fencing token, or conditional row updates with a version column and a legal-transition check; call it from every index write; do not mark Failed on lock timeout
- ☐ T6 Transactional outbox: commit Completed and the fan-out event in one transaction, then ack, then fan out
- ☐ T7 Data trust model: where bytes land before validation, who scans, key management (scope, rotation, revocation), node authentication for relays, authenticated encryption for relayed data
- ☐ T8 One typed result contract across subroutines (Approved, Rejected with reason, NeedsReview, ScanPending) and one error convention (return or throw)

#### Should fix

- ☐ T9 Data Stream thresholds: close or strong enough, enough active nodes, large-file cutoff, Bluetooth size gate, chunk size, stall timeout value
- ☐ T10 Confirm the retry-invalid path reaches failover before enqueue; every enqueue exit sends a deferred report to the server
- ☐ T11 Bind session tokens to submission, user, host, and size limit; issue a new token per host
- ☐ T12 One retry policy: which layer retries which error, global attempt cap per submission, exponential backoff with jitter, retryable versus non-retryable classes, reconcile with the PRD's 3 second value
- ☐ T13 State model: transition table, immutable payload with mutable metadata, failure reason codes (retryable or not), Expired outcome, rename In Progress
- ☐ T14 Manual review flow for In Review: owner, time limit, approve and reject paths, what the user sees
- ☐ T15 Index sync: version scheme, gap detection with full resync, idempotent apply, tombstones for invalidated content, precedence between live push, webhook, and enqueued processing, and what "validate content matches" compares; fix the subscriber entry that nothing sends when Connection Validation enqueues an update
- ☐ T16 Server Content Validation: sandboxed strip with a failure branch (fail closed), define the canonical hash input, scan timeout handling
- ☐ T17 Outbox: attempt cap, time limit, Expired outcome, triggers (connectivity change, app foreground, timer), consider submitting to the server before validating hosts so waiting items are visible
- ☐ T18 Server-side consumer for the transfer heartbeat that updates last-progress
- ☐ T19 Failed cleanup: delete quarantined blobs on hosts, keep a tombstone or audit record, set retention, index on state and updated time

#### Hygiene

- ☐ T20 Remove the main node as a single point of failure or document the failover
- ☐ T21 Observability: correlation ID per submission, metrics (queue depth, stuck count, state age), alerts, audit log of denials and reviews
- ☐ T22 Non-functional requirements: file size and duration targets, concurrency, latency, durability and replication, per-user quotas, cost
- ☐ T23 Glossary and consistent terminology across diagrams
- ☐ T24 Diagram cleanup: clipped notes, label collisions, merge lines into single junctions, label every cross-lane arrow
- ☐ T25 Happy-path sequence diagram, and extend the topology diagram with relays, replicas, and the host registry
- ☐ T26 Request and response contracts for Submit API Request and each transfer report
- ☐ T27 Keep the design decisions table current

#### New flows (placeholders, see Other flows to design)

- ☐ T28 Read and retrieval path
- ☐ T29 Host node initialization
- ☐ T30 Host node decommission
- ☐ T31 Edit, delete, invalidate, cancel
- ☐ T32 Index sync protocol
- ☐ T33 Flagged content queue and processor
- ☐ T34 Join a session by NFC tap (range is a few centimeters and throughput is low, so use it for authentication or pairing and hand the transfer to Bluetooth or Wi-Fi)
- ☐ T35 Item fallback when a host goes down
- ☐ T36 User content rendering and messaging sockets

### Glossary

Several terms are used interchangeably across the diagrams. Entries marked to confirm need a decision before they can be treated as one thing (T23).

| Term | Meaning in this doc | Note |
|---|---|---|
| Main node | The host that coordinates: holds the index and host ID tables, handshakes with clients, default host when only one exists | Also called main host |
| Host node | A phone that stores content and receives the data stream | Also called host or host node |
| Server side | The lane in the flow diagrams where the authoritative steps run | To confirm: main node, app service, or both |
| Index table | Table of where each object lives, backups, history, previews, uploader, type, tags | Source of truth for item state |
| Host ID table, core ID table, ID directory | Table of hosts and ID mappings checked in Connection Validation | To confirm: same as the index table or separate |
| Admission | Server checks that use only claims in the request, before any bytes move | Proposed term |
| Content validation | Server checks that use the received bytes: integrity, scan, strip | Proposed term |
| Quarantine | Non-served storage and a non-visible state for bytes that have not passed validation | Proposed, not yet drawn (T2) |
| Outbox | Durable client-side store for submissions waiting to be sent | Client Outbox Consumer drains it |
| Transfer report | Client message to the server describing how a stream ended | Server decides the resulting state |
| Heartbeat | Periodic progress signal during a transfer | Server consumer not yet drawn (T18) |
| Ranked host list | Top 3 hosts from Suitable Host Determination | Used for failover |
| Stuck | New, In Progress, or Queued with no recent progress | Must use heartbeat or last-progress time |
