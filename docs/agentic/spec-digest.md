# Locale MVP: spec digest

Nov 13, 2026 · Single source of truth for the build phase. Built from `docs/Locale MVP Flows.md` rev 2 (D10 updated for Flutter), `docs/Locale Design Tradeoffs.md` ("TR") ("MVP"), `docs/Locale PostHog Plan.md` ("PH"), `docs/agentic/source/screens.md` ("screens"), and the older design doc and review (TODO IDs T1–T36). The MVP doc wins every conflict (§13).

**How to read it.** Each item cites its source. **DU-n** marks a *Default (unconfirmed)*: a gap the sources leave open, filled here so agents do not guess differently. All DUs are listed in §12; changes go through the orchestrator into this file. **Status Nov 13, 2026: all open questions are resolved** (`open-questions.md`). DUs a person confirmed are marked **Decided** in §12; the rest are accepted defaults, and agents build on them without asking. Screens are in `screen-inventory.md`; tasks in `work-breakdown.md`.

**Conventions (DU-1).** Timestamps are Unix milliseconds, UTC. IDs are UUIDv4 strings unless stated. JSON uses snake_case. Sizes are bytes. Hashes are lowercase hex SHA-256.

---

## 1. Product and scope

**Goal (MVP §2).** Up to 50 people on one Wi-Fi network. A host starts an event from a laptop; guests join by QR code; verified users upload photos from the phone or desktop app; everyone browses and downloads at original quality in an app or a browser. The event and its photos are deleted when it expires.

**Success criteria (MVP §2), used as acceptance tests**

| # | Criterion |
|---|---|
| SC1 | iPhone and Android both upload to a desktop-hosted event; each downloads the other's photos byte-identical to the original (see DU-20 for what "original" means) |
| SC2 | An iPhone HEIC photo is viewable in the Android app and in Chrome, and downloadable as original HEIC or compatible JPEG |
| SC3 | A guest joins with no account, from app or browser, and downloads; a guest cannot upload |
| SC4 | A 12 MP photo transfers host to client in under 5 s on a typical home network |
| SC5 | New photos appear on every connected attendee's screen within 2 s of upload completing |
| SC6 | An interrupted upload resumes, or retries from the outbox, with no re-selection |
| SC7 | No photo bytes ever touch the app service |
| SC8 | At end or expiry all photos are deleted from the host and attendees' in-app caches; files saved to the camera roll or a folder stay |

**In scope (MVP §1):** guest and verified profiles; nearby discovery; open, password and invite-link events; event page with Photos, Posts (host-only) and Attendees; photo pipeline with validation and metadata stripping; per-event retention, extend, reopen-as-new; quick session; one hosted event per account; local event log; host storage allocation; desktop host, desktop attendee, browser guest; HEIC handling.

**Non-goals (MVP §2):** phone hosting (M5), multiple hosts, Bluetooth, relays, video, Live Photo motion, browser uploads, chat, feed, polls, location, games, ledgers, cloud search, Linux host, request-and-approval access, NFC, webcam QR scan, tags, documents, co-hosting, restore after expiry.

---

## 2. Decisions D1–D17 (MVP §3)

| ID | Decision |
|---|---|
| D1 | The **host device** is the server for everything about an event. The **app service** does accounts and credentials only and never sees event data |
| D2 | No cloud event placeholder. Invites are QR codes and links made on the host |
| D3 | The client holds bytes while an upload waits, in a durable outbox. No server queue |
| D4 | Bytes land in a **quarantine folder** on the host, never served |
| D5 | The host validates content: image-only allowlist plus a decode check |
| D6 | Native apps: TLS to the host, certificate fingerprint pinned from the QR code. Browsers: plain HTTP, view-only, session token. At rest: OS disk encryption only |
| D7 | The app service issues a signed credential (7-day life); the host verifies it offline with the service's public key |
| D8 | Same local network only. mDNS for nearby discovery. No Bluetooth |
| D9 | Single writer: the host process writes the index in SQLite transactions |
| D10 | Desktop host first (macOS, Windows): a **portable Rust core** in a thin **Tauri** shell. Phone apps are one **Flutter** app calling the core through **flutter_rust_bridge**; from M5 the core also hosts on the phone (TR-1, TR-2) |
| D11 | Store the original byte-for-byte. Make a JPEG **preview** at upload and a **compatible** JPEG on first request, cached. Clients choose Original or Compatible |
| D12 | Native iOS, Android and desktop apps for anyone who uploads. A host-served web page for view-and-download guests, only for open and invite-link events |
| D13 | Host can extend before expiry within a 7-day total cap; everyone is told. After expiry content is gone; "Host again" makes a new empty event from saved settings |
| D14 | Quick session: one click, invite-link only, 12 h max, retention 0, convertible to a full event with the same QR code |
| D15 | One active hosted event per account, enforced by the host app only. Hosts may join other events. The host's phone gets the `host` role but hard-to-undo actions stay on the host device |
| D16 | PostHog EU cloud, IP capture off, opt-in, metadata only, one allowlisted schema in the shared core; flags and remote config frozen per event by the host; session replay off |
| D17 | Before joining, a device sees only: event name, host name, times, access type, generated cover art, and (open and invite-link events only) photo and attendee counts. Nothing from inside the event |

---

## 3. Architecture and stack

**Components (MVP §4)**

| Component | Responsibility |
|---|---|
| App service | Sign-up, sign-in, issue and refresh signed credentials. Never receives event data or photo bytes (SC7) |
| Host core (Rust library) | Event API and socket, guest web page, event index (SQLite), quarantine and content folders, validation, previews, compatible copies, access control, sweeper and expiry |
| Client core (Rust library) | Join, pinned TLS, local index and sync, outbox, downloads and cache. Shared by desktop and, through flutter_rust_bridge, the Flutter phone app (MVP §5.3 "Desktop app join") |
| Desktop app | Tauri shell over both cores: host UI and attendee UI |
| Phone app | One Flutter (Dart) app for iOS and Android over the client core. Native code only in our own plugins: `locale_media` (picker returning the unconverted original, Save to Photos), `locale_net` (iOS Local Network permission, Android multicast lock for mDNS), and from M5 `locale_host_service` (TR-1, TR-2) |
| Guest web page | Static page served by the host; in-memory index; view and download only |

The design doc's "main node" and "host node" are one device; "index table", "host ID table" and "ID directory" are one **event index** (MVP §4).

**Stack choices.** D10 fixes the Rust core and the Tauri desktop shell; TR-1 fixes Flutter for the phones. The rest is unspecified in the sources:

| Area | Choice | Status |
|---|---|---|
| Phone UI | Flutter (current stable at S1, pinned), Dart, one app for iOS 16+ and Android minSdk 28. Core bindings generated by `flutter_rust_bridge`. Do not use general-purpose picker or gallery-saver plugins for originals: `locale_media` calls `PHPickerViewController` with `preferredAssetRepresentationMode = .current` on iOS and the system photo picker on Android (MVP §5.4 step 1). State management: Riverpod (**DU-42**) | **Decided** (TR-1) |
| Desktop and guest web UI | TypeScript + React + Vite. Guest page assets are embedded in the host binary and load with no internet (no CDN, no web fonts) | **Decided** (TR-4) |
| Core libraries | tokio, axum or hyper, rustls, a WebSocket crate, rusqlite with bundled SQLite in WAL mode, an mDNS crate, rcgen, Ed25519, Argon2id, flutter_rust_bridge. Equivalent crates are fine | **DU-4** |
| Image codecs | One `ImageCodec` trait. macOS: ImageIO. Windows: bundled libheif for HEIC (internal builds only until licensing advice, TR-15), OS decoder (WIC) for JPEG and PNG (MVP §5.4 step 15). All decoding runs in the worker process | MVP; trait is **DU-5** |
| App service | Small Rust HTTP service with SQLite, one process; code sender behind a trait (Postmark in production, console in development). Deployed as one Fly.io machine in an EU region with a SQLite volume and daily snapshots; production signing key only as a Fly secret | **Decided** (TR-13) |
| Repository layout | Cargo workspace: `crates/proto` (wire types, reason codes, telemetry schema), `crates/host-core`, `crates/client-core`, `crates/image-worker`, `crates/mobile-bridge` (flutter_rust_bridge API), `crates/harness`; `apps/desktop`, `apps/web-guest`, `apps/mobile` (Flutter), `apps/service`; `packages/locale_media`, `packages/locale_net`, later `packages/locale_host_service` (Flutter plugins with Swift and Kotlin code) | **DU-7** |

---

## 4. Flows

Step numbers match the MVP doc so tests can cite them.

### 4.1 Sign up and sign in (MVP §5.1)
1. Choose **Continue as guest** or **Sign up / Sign in**.
2. Guest: local profile (display name, random guest ID). No network.
3. Sign up: one-time code through the app service. Email only in the MVP (**Decided**, TR-13; the doc says "email or phone").
4. Service returns a signed credential: account ID, display name, role `verified`, expiry.
5. The app refreshes it whenever online and within 2 days of expiry.
6. If it expires offline, the user has guest permissions until refreshed; show "Viewing only" (MVP §12).
7. Browser guests give a display name; a random guest ID is kept in the browser for the event.
8. The desktop app uses the same sign-in. Hosting and uploading need a verified account; joining as a viewer does not.

### 4.2 Create an event and host (MVP §5.2)
1. Desktop: **Host an event**. Phone (MVP): the same button explains that hosting runs on Locale Host for Mac or Windows and offers to email the download link; guests are asked to sign up first.
2. Form: name, description, start, end, optional links, access type (`open`, `password`, `invite`), retention (default 24 h after end; choices in mockup: at end, 24 h, 3 days), browser guests allowed (default on; not offered for password events).
3. Storage allocation (default 20 GB; mockup offers 10, 20, 50) and folder (default `~/Locale/Events`).
4. **Capability check**, blocking on failure with reason and fix: network connected or hotspot on; firewall allows the port; free space ≥ allocation; on a laptop, plugged in or battery > 50%.
5. No usable network: offer hotspot (Windows Mobile Hotspot) or travel-router instructions.
6. Generate a TLS key pair for the event; start API, socket and guest page.
7. Create the index; write the event record (version 1).
8. Advertise over mDNS (open and password events only, §5.6).
9. Show the QR code (full screen, print). URL format in §5.1. The password is never in the QR code.
10. Hosting window shows attendee count, uploads in progress, storage used, End event. Prevent sleep while hosting.

### 4.3 Discover and join (MVP §5.3)
1. Scan the QR code, open an invite link, or pick from **Nearby** (open and password events only).
2. Connect over TLS; compare the certificate fingerprint with the QR code. Nearby joins: show the host code and trust on first use.
3. Send the credential or guest profile.
4. **User Validation** on the host, in this order: rate limit → session → **Access Validation**: (a) credential signature valid and unexpired, else treat as guest; (b) access rule for the event's type: `invite` needs a valid join token, `password` needs the correct password, `open` needs nothing (**DU-9**: a password event always needs the password, even with a QR token); (c) not on the removed list.
5. Add to roster with role `host` (credential's account equals the event's host account), `uploader` (verified) or `viewer` (guest). Issue a session token bound to attendee and event, TTL 12 h. The app refreshes silently while connected; if it lapses while away the app re-joins with its stored join token and credential and keeps its roster entry.
6. Host returns the snapshot and version (4.6).
7. Open the event page.

**Browser guest:** open the QR URL over HTTP → page reads the join token from the fragment → sends it with a display name → host validates as `viewer` (event must be open or invite, browser guests allowed) → read-only session token (TTL 2 h, refreshed while the socket is open, **DU-10**) → gallery with banner "View only. Get the app to upload and save at full quality."

**Desktop app join:** Events page lists Your events, Join with a link (paste the QR URL) and Nearby. Same steps 2–7, `client_kind = desktop`. Sign-in optional for viewing. Extras: Add photos by drop or picker; Save Original or Compatible to a folder (default `Downloads/Locale/<event name>`; default version Original on macOS, Compatible on Windows); **Download all originals**, resumable, skipping saved files. Joining while hosting is allowed; the app never changes networks; the hosted event keeps bandwidth priority.

**Opening a QR URL when the app is installed (DU-11).** A phone camera opens the URL in the browser. The guest page shows "Open in the Locale app", a `locale://join?u=<url-encoded join URL>` link. The in-app scanner reads the URL directly.

### 4.4 Upload a photo (MVP §5.4)
Client:
1. Pick photos. iOS: `PHPickerViewController` with `preferredAssetRepresentationMode = .current`. Android: system photo picker. Live Photo: still only.
2. **Client Content Precheck:** type JPEG, HEIC/HEIF or PNG; size ≤ 50 MB; strip location metadata without re-encoding (§8.5). Failure: inline error, nothing sent.
3. New submission ID (UUID); SHA-256 of the stripped file.
4. Write to the outbox (file copy plus record). Survives app kill from here.
5. Show in the gallery as "Uploading", visible to the uploader only.

Host admission:
6. User Validation: rate limit 30 uploads per 5 min per user; session valid; role `uploader` or `host`.
7. Insert the item as `PENDING`; the unique constraint on `submission_id` decides retries. If it exists, return its current state.
8. Duplicate: same hash from the same user returns the existing item (**DU-12**: the new row becomes `FAILED` with reason `duplicate`, not retryable, and the response carries `duplicate_of`; the client treats it as success). Same hash from another user is allowed.
9. **Basic Content Precheck** on claimed type and size; check allocation has room. Failure: `FAILED` with reason.
10. Set `UPLOADING`; return an upload token bound to submission, user and maximum size, plus chunk size.

Transfer:
11. Chunks of 1 MB over TLS; each chunk updates `last_progress_at`.
12. On a drop, ask the host for the received offset and resume. Retry Mechanism: 3 attempts, exponential backoff with jitter (base 500 ms, PH §7); then the item waits in the outbox (4.8).

Host validation:
13. All bytes in: `VALIDATING`. Entry invisible to others.
14. Integrity: SHA-256 equals the claim. Mismatch: discard and ask for a re-send up to 3 times, then `FAILED` (mechanics in §7, DU-13).
15. **Server Content Validation**, in a separate worker process (§8.4): magic bytes and HEIC brand match an allowed type; full decode (fail closed → `invalid_image`); strip remaining location metadata without re-encoding (§8.5); record width, height, format, HDR gain map flag; make the preview (JPEG, 1024 px long edge, orientation applied, quality 80 **DU-14**).
16. Move the original from quarantine to the content folder.
17. One SQLite transaction: `AVAILABLE`, preview path, image facts, change-log row with the next version.
18. Ack the uploader, then push to every open socket.

The compatible copy is made on first request (4.5). For a JPEG or PNG original, the original is the compatible copy.

**Host uploads from the desktop:** same precheck, same API over loopback with its own submission ID, file **copied** into quarantine (never moved or referenced), counts toward the allocation.

### 4.5 Browse and download (MVP §5.5)
1. Render the gallery from the local index and cached previews; never poll per item.
2. Open a photo: show the preview, then fetch the full image: original where the device decodes it (iOS, Android 9+, Safari, macOS), else compatible (Chrome, Firefox, Edge, Windows without codecs).
3. Host checks the session and that the item is `AVAILABLE`. Guests allowed.
4. Compatible copy on demand: decode in the worker, JPEG quality 92, full resolution, keep the colour profile, store, record path and hash. Concurrent requests share one job.
5. Stream; byte-range resume (apps).
6. App verifies the hash (original's or compatible copy's); mismatch → retry once, then error.
7. Save offers **Original** (default on iPhone, Mac, Android 9+) or **Compatible (JPEG)** (default in browsers without HEIC, on Windows, on older Android). One choice only when they are the same file.
8. App: Save to the camera roll (desktop: folder). Browser: file download; on iOS note that it lands in Files.
9. Host logs the download and version asynchronously and updates counts (4.12).

### 4.6 Event page and index sync (MVP §5.6)
1. On join or reconnect the client sends its local version (0 if new).
2. Host: equal → nothing; version within the retained change log → changes since, in order; otherwise → full snapshot. The change log is kept for the whole event (**DU-15**).
3. While connected, the host pushes each change as `{version, change}`.
4. Client applies a change only if `version == local + 1`. Lower or equal: drop. Gap: resync from step 1.
5. Deletes and expiries are tombstones: remove the item and its cached files.
6. Socket drop: reconnect with the Retry Mechanism, then resync.
7. Browser guests use the same protocol over WebSocket, index in memory, fresh snapshot on reload.

**Event page:** header (name, times, links, deletion time or status banner); pinned card with the host's latest post or a system notice; tabs **Photos** (own in-progress uploads visible only to the uploader), **Posts** (host posts newest first, system notices styled differently), **Attendees** (roster with role and photo count).

### 4.7 Delete, moderate, end, expire (MVP §5.7)
- **Delete a photo:** uploader or `host` role. Host writes a tombstone (new version), deletes file, preview and compatible copy, logs the actor, pushes.
- **Delete a post:** `host` role, from either host device or host's phone (MVP §5.11 table).
- **Remove an attendee (host device only):** revoke the session, close the socket, add account or guest ID to the removed list, optionally delete all their uploads.
- **Edit details (host device only):** write the event record, bump version, push. Editable: name, description, links, browser-guests setting. Times change through 4.9. Access type and password are fixed for the event's life (**DU-16**).
- **End:** host clicks End event, or `expires_at` passes. (1) Push `event_ended`. (2) Clients delete cached originals and previews; saved files stay. (3) Host deletes content, quarantine and index, stops server, page and advertising. Only an `event_template` survives.
- **Paused:** if the host leaves the network, sleeps or quits, the event is paused, not ended. Attendees see "Host offline". On return, clients resync. If `expires_at` passes while paused, each client deletes its cache on schedule from the expiry it already holds, and the host ends the event when it next starts. On launch the host app resumes an unexpired event automatically with the same TLS key, join token and port if free (**DU-17**).

### 4.8 Recovery (MVP §5.8)
**Client outbox consumer.** Runs on app foreground, network change, socket reconnect, and every 60 s while the event is open. Per item: event active and session valid? No → `Expired`, tell the user. Resubmit with the same submission ID. Cap: 10 attempts or 24 h, then `Expired` with **Retry**. User-visible states: Waiting for connection, Uploading (n%), Failed (reason), Expired. (Mockup labels: "62%", "Checking", "Waiting", "Not sent", "Stopped", "Shared".)

**Host sweeper,** every minute, in-process singleton:
1. `UPLOADING` with no chunk for 2 min → `FAILED` `stalled`; delete partial bytes. A resubmit resets it to `UPLOADING`.
2. `PENDING` older than 10 min → `FAILED` `abandoned`.
3. `FAILED` older than 1 h → delete the row and residue.
4. Event past `expires_at` → End.
5. One hour before `expires_at` → notify the host with **Extend**; push an "expiring soon" notice.
6. Quick session past `max_ends_at` → End. One hour before → warn the host.

### 4.9 Extend, shorten, reopen (MVP §5.9)
`expires_at = ends_at + retention`. Host device only. Choices: +1 hour, +6 hours, +1 day, custom end and retention.
1. Check: actor is the host device; event `active` (a paused event cannot be extended); new `expires_at ≤ created_at + 7 days`; free space within allocation (warn, still allow).
2. One transaction: update `ends_at`, `retention_hours`, `expires_at`; increment `extension_count`; append `event_extended` with old and new expiry; write the event log; add a system post.
3. Push. Attendees see "Photos now delete on <date>", linking to My uploads. Clients update cache expiry.
4. **Shorten:** same flow, any time. New `expires_at` in the past → end now.
5. Clients never extend on their own. Deleting early is safe; keeping late is not.

**Reopen:** after end the host keeps settings only (`event_template`). **Host again** creates a new event: new ID, join token, TLS key, empty gallery.

### 4.10 Quick session (MVP §5.10)
Defaults: name "<Host first name>'s session · <start time>" (renamable); no description, links or times; access `invite`; browser guests on; `ends_at = now + 12 h`; `retention_hours = 0`; storage 5 GB; `kind = quick`.
1. Verified host clicks **Quick session** (desktop: top of the Host page).
2. Fast capability check plus a **reachability test** for client isolation (method: §8.6). OK → continue. Firewall prompt → inline, continue. Isolated or likely → "This Wi-Fi may block phones from reaching you" with **Turn on hotspot** (Windows; on macOS explain Internet Sharing needs a second connection or suggest a travel router) and **Try anyway**.
3. Create with defaults, start, show the hosting window. No form.
4. Hosting window shows "Ends when you end it · auto-ends at <time>", **Rename**, **Add details**.
5. **Add details** opens the event form pre-filled; saving sets `kind = event` with the chosen end and retention. Same event ID and QR code. A "Session updated" system post if the deletion time changed.
6. **End session** = End (everything deleted at once).
7. Sweeper ends it at 12 h. One hour before: **Keep going** (adds up to 12 h; hard limit 24 h from start) and **Add details**.
8. Settings are saved to Past events only if converted.

### 4.11 One hosted event, many joined (MVP §5.11)
- The host device refuses a second event while one is `active` or `paused`. Host an event, Quick session and Host again are replaced by "You're hosting <name>" with **Open live event**.
- The phone learns its account is hosting when it joins that event as host, and shows the same state on Home.
- The host's phone has role `host`, `is_host_device = false`.

| Action | Host device | Host's phone |
|---|---|---|
| Upload, post, delete any photo or post | Yes | Yes |
| Edit details, extend or shorten, remove attendee, end | Yes | No |

- Clients keep a separate local index, outbox and cache per event. Home lists **Your events** with the role in each. An upload always targets the event whose page opened the picker.

### 4.12 Browse, sort, filter (MVP §5.12)
- Layout: pinned post above tabs → people row ("Everyone" first, then uploaders by photo count; one at a time) → section header with one sort menu ("Newest ▾"; with a Show filter: "Newest · 1 filter") → grid (3 columns on a phone, more on desktop; first photo of each group is a large tile; tiles show uploader initial and time or rank).

| Sort | Order | Grouping | Tile badge |
|---|---|---|---|
| Newest (default) | Latest first | 30-minute blocks | Time |
| Oldest | From the start | 30-minute blocks | Time |
| Most downloaded | `download_count` descending | None | ↓ count and rank |
| Hottest | `hot_score` descending | None | ↑ recent count and rank |

- Show filters, one at a time: Everything, Saved by me, Not saved yet, My photos. "Saved" filters are computed on the device from its own cache.
- Counts: each download adds 1 to `download_count` and 1 to `hot_score`; `hot_score` halves every 15 minutes (continuous decay: `score × 0.5^(Δt / 15 min)`, **DU-18**). Aggregate only; nobody sees who downloaded what. Sent in batches at most every 30 s as a versioned `counts_updated` change. What counts as a download: **DU-19**, §5.3.
- Photo time is the time the item became `AVAILABLE` (**DU-21**); capture time is not used.

---

## 5. Interfaces

The sources describe these interfaces but give no endpoints or payloads (review T26). This section fills that gap and was **approved by a person on Nov 13, 2026** (open question Q2), so it is **Decided**. Changes go through the orchestrator and update this section in the same change. `crates/proto` is the code form of this section.

### 5.1 Transport, QR URL, fingerprint
- **One port** (default 7420; next free port if taken). The listener reads the first byte: `0x16` → TLS, otherwise plain HTTP. Native apps use TLS only; plain HTTP serves the guest page and guest API only. One port keeps the firewall rule single (MVP §5.2 step 4).
- **Join URL (MVP §5.2 step 9):** `http://<host-ip>:<port>/e/<event-id>#t=<join-token>&fp=<cert-fingerprint>`. `event-id` = `evt_` + 12 hex. `t` = 128 random bits, base64url. `fp` = SHA-256 of the certificate DER, base64url without padding.
- **Certificate:** self-signed ECDSA P-256, one per event, valid for the event's maximum life. The key is stored in the event folder with owner-only permissions and deleted with the event.
- **Host code** shown for Nearby joins: first 12 hex digits of the fingerprint, uppercase, as `4F2A · 9C1E · 77B0` (screens: Join, Live).
- **Host IP changes:** the host regenerates the QR code; joined clients find the host again by mDNS, or by scanning the subnet's last known port, and accept it only if the pinned fingerprint matches.

### 5.2 Errors and reason codes
Error body: `{"error": {"code": "...", "message": "...", "retryable": bool, "retry_after_ms": int?}}`.

| Group | Codes | Source |
|---|---|---|
| Item reason codes (`item.reason_code`) | `type`, `size`, `storage_full`, `abandoned`, `stalled`, `cancelled`, `unauthorized`, `invalid_image`, `integrity` | MVP §6 |
| Added item reason | `duplicate` | DU-12 |
| Join failures | `wrong_password`, `token_invalid`, `removed`, `event_ended`, `browser_not_allowed`, `rate_limited`; client-side only: `local_network_denied`, `fingerprint_mismatch`, `unreachable`, `token_expired` | PH §5 `join_failed`, plus DU |
| General API | `session_invalid` (401), `role_denied` (403), `not_found` (404), `not_available` (409), `offset_mismatch` (409), `rate_limited` (429), `bad_request` (400), `proto_unsupported` (426) | DU |

Retryable item reasons: `abandoned`, `stalled`, `integrity`. All others are final (MVP §6).

### 5.3 Host API (all under `/e/{event_id}`)
Auth: `Authorization: Bearer <session_token>`; the guest page also gets the token as a cookie scoped to `/e/{event_id}` so image and download links work. Every request carries `X-Locale-Proto: 1`.

| Method and path | Who | Request → response |
|---|---|---|
| `GET /` (HTTP) | Anyone | Guest web page |
| `GET /meta` | Anyone | → `{event_id, name, host_name, starts_at, ends_at, expires_at, access_type, kind, status, browser_guests_allowed, proto, counts?: {photos, attendees}}`. `counts` only for open events, or invite events when `X-Join-Token` is valid; never for password events (D17) |
| `POST /join` | Anyone | `{client: {kind, app_version}, credential?, proof?, guest?: {guest_id, display_name}, join_token?, password?}` → `{session_token, session_expires_at, attendee: {id, role, is_host_device}, config}`. `kind` ∈ `ios`, `android`, `desktop`, `web` |
| `POST /session/refresh` | Attendee | → `{session_token, session_expires_at}` |
| `GET /ws` | Attendee | WebSocket, §5.4 |
| `POST /items` | uploader, host | `{submission_id, content_type, size_bytes, sha256, original_name}` → `{submission_id, item_id, state, reason_code?, retryable?, received_bytes, upload_token?, chunk_bytes?, duplicate_of?}`. Idempotent on `submission_id` |
| `HEAD /uploads/{submission_id}` | Uploader | → header `Upload-Offset` |
| `PUT /uploads/{submission_id}` | Uploader | Headers `Upload-Token`, `Upload-Offset`; body = one chunk → `{received_bytes, state}`. Wrong offset → 409 `offset_mismatch` with `received_bytes` |
| `DELETE /uploads/{submission_id}` | Uploader | Cancel → `FAILED` `cancelled` |
| `GET /submissions/{submission_id}` | Uploader | → current `{state, reason_code, retryable, received_bytes, item_id}` |
| `GET /items/{id}/preview` | Attendee | JPEG |
| `GET /items/{id}/original` · `/compatible` | Attendee | File. Supports `Range`. Headers `X-Locale-Sha256`, `Content-Disposition` with `original_name` (extension `.jpg` for compatible). `?save=1` marks a download for counting |
| `POST /items/{id}/saved` | Attendee (apps) | `{version}` → 204. Reports a save made from cache |
| `DELETE /items/{id}` | Uploader of it, host | → 204 |
| `POST /posts` · `DELETE /posts/{id}` | host role | `{text}` (1–500 chars) → post |

- **Download counting (DU-19).** A download is a request with `?save=1` or a `POST …/saved`. Viewing fetches do not count. The host increments the item's counts and writes an event-log row with actor and version but **no item ID**, so the host cannot see who saved what (MVP §5.12).
- **Host-device-only actions** (edit, extend, shorten, remove attendee, end, convert quick session) are not on the network API. The desktop shell calls host-core functions in process (MVP §5.11).
- **Rate limits:** uploads 30 per 5 min per user (MVP §5.4); joins 10 per minute per IP and password attempts 5 per minute per IP (DU).

### 5.4 Index sync messages (WebSocket, JSON text frames)
Client → host: `{"t":"hello","session_token","version"}` (first frame, within 5 s), `{"t":"resync","version"}`, `{"t":"ping"}`.

Host → client:
- `{"t":"snapshot","version","event","attendees":[…],"items":[…],"posts":[…],"config":{…}}` (`items` are `AVAILABLE` only)
- `{"t":"changes","changes":[{version,type,payload,ts}…]}` (catch-up) and `{"t":"change","version","type","payload","ts"}` (live)
- `{"t":"upload_state","submission_id","state","reason_code","retryable","received_bytes","item_id"}` (to the uploader only; not versioned; this is the completion ack of 4.4 step 18)
- `{"t":"notice","kind":"expiring_soon","expires_at"}` (not versioned)
- `{"t":"bye","reason":"removed" | "event_ended" | "session_expired"}`, `{"t":"pong"}`

Change types (MVP §5.6 step 3, plus counts from §5.12):

| `type` | Payload |
|---|---|
| `event_updated` | Changed event fields |
| `event_extended` | `{old_expires_at, new_expires_at, ends_at, retention_hours}` (also used for shortening) |
| `item_available` | Item: `{id, uploader_id, content_type, size_bytes, sha256, width, height, has_hdr_gain_map, compatible_same_as_original, original_name, available_at}` |
| `item_deleted` | `{id}` (tombstone) |
| `attendee_joined` | `{id, display_name, role, verified, client_kind, is_host_device}` |
| `attendee_removed` | `{id}` |
| `post_published` | `{id, kind: host | system, author_id, text, created_at}` |
| `post_deleted` | `{id}` |
| `counts_updated` | `[{id, download_count, hot_score, hot_score_updated_at}]` |
| `event_ended` | `{}`; then the socket closes |

Photo counts per attendee are derived on the client from items.

### 5.5 App service API
| Method and path | Request → response |
|---|---|
| `POST /v1/auth/start` | `{email}` → 204; sends a 6-digit code valid 10 min |
| `POST /v1/auth/verify` | `{email, code, display_name?, device_pubkey}` → `{account, credential, refresh_token}` |
| `POST /v1/credential/refresh` | `{refresh_token, device_pubkey}` → `{credential}` |
| `PATCH /v1/account` | `{display_name}` → `{account, credential}` |
| `POST /v1/host-link` | Emails the desktop download link to the account's address (4.2 step 1) |

**Credential (MVP §7 plus DU-31):** `v1.<base64url(payload JSON)>.<base64url(Ed25519 signature)>`, payload `{kid, account_id, display_name, role: "verified", device_pubkey, issued_at, expires_at}`. Service public keys ship in the host app as a list keyed by `kid`; rotation is an app update.

**Proof of possession (DU-31).** The credential names the device's Ed25519 public key; `proof` in `/join` is a signature over `"locale-join-v1" ‖ event_id ‖ cert_fingerprint ‖ timestamp` (accepted within ±5 min). Without it, any host that receives a credential could replay it elsewhere as that user. The sources specify a bearer credential only.

### 5.6 mDNS (DU-32)
Service type `_locale._tcp.local.`, instance name = event ID. TXT: `v=1`, `id`, `name`, `host` (host display name), `access` (`open` or `password`), `st`, `en`, `fp` (12-hex host code). Advertised only for open and password events (MVP §5.3 step 1). Counts come from `/meta`.

### 5.7 Client core API through flutter_rust_bridge (DU-33)
`sign_in_*`, `join(url | nearby)`, `list_events`, `open_event(id) → observable index`, `enqueue_upload(event_id, file)`, `outbox(event_id) → observable`, `fetch(item, version)`, `mark_saved`, `delete_item`, `create_post`, `leave`, `telemetry.track`. Observables are exposed to Dart as streams. The core owns all networking and runs on its own threads; Dart never sends protocol traffic. The Flutter app supplies: network change, app foreground, file locations, the PostHog SDK hand-off (PostHog Flutter SDK), and calls into `locale_media` for picking and saving. The desktop (Tauri) calls the same API directly in Rust.

---

## 6. Data model (MVP §7)

**App service:** `account(id, display_name, email, created_at)`. Credentials are issued, not stored. Added (DU-34): `login_code(email, code_hash, expires_at, attempts)`, `refresh_token(hash, account_id, device_pubkey, expires_at)`.

**Host event index (SQLite).** Fields from MVP §7; fields in *italics* are additions needed by the flows (**DU-34**).

| Table | Fields |
|---|---|
| `event` | id, name, description, starts_at, ends_at, links, access_type, password_hash (Argon2id), retention_hours, expires_at, extension_count, created_at, storage_limit_bytes, version, status (`active`, `paused`, `ended`), kind (`event`, `quick`), max_ends_at, *host_account_id, host_display_name, browser_guests_allowed, join_token, storage_path, port, config_version, config_json* |
| `event_template` (host app database, outside the index) | id, name, description, links, access_type, retention_hours, browser_guests_allowed, last_used_at |
| `attendee` | id, event_id, account_id or guest_id, display_name, role (`host`, `uploader`, `viewer`), is_host_device, client_kind (`ios`, `android`, `desktop`, `web`), joined_at, removed_at. Unique on (event_id, account_id or guest_id); a re-join reuses the row. `removed_at` not null = on the removed list |
| *`session`* | *token_hash, attendee_id, scope (`full`, `read`), expires_at, revoked_at* |
| `item` | id, submission_id (unique), event_id, uploader_id, state, reason_code, content_type, size_bytes, sha256, width, height, has_hdr_gain_map, file_path, preview_path, compatible_path (null until made; equals file_path for JPEG and PNG), compatible_sha256, download_count, hot_score, hot_score_updated_at, created_at, updated_at, last_progress_at, attempt_count, *claimed_sha256, original_name, received_bytes, integrity_failures, available_at* |
| `change_log` | version (primary key), event_id, change_type, payload, created_at |
| `post` | id, event_id, author_id, kind (`host`, `system`), text, created_at, deleted_at |
| `event_log` | id, ts, actor_id, client_kind, action, target_id, result, reason. Actions: joins, denials, uploads, downloads with version (target_id null, DU-19), deletes, removals, extensions |

Index on `item(state, updated_at)` for the sweeper (T19). One index database per event, inside the event folder, so deleting the folder deletes everything (4.7).

**Event folder layout (DU-35):** `<storage_path>/<event_id>/{index.sqlite, tls/, quarantine/, originals/, previews/, compatible/}`.

**Client local stores (MVP §7), one set per joined event:** `local_index` (mirror of event, attendees, `AVAILABLE` items, posts; `version`; own role; pinned fingerprint; join token; session token); `outbox(submission_id, event_id, local_file_path, sha256, size, state, attempts, created_at, last_error)`; `cache(item_id, version: original | compatible, path, preview_path)`; *`saved(item_id, version, saved_at)`* for the Saved filters.

---

## 7. Item state machine (MVP §6)

| From | To | Trigger | Who | Retryable |
|---|---|---|---|---|
| (none) | PENDING | Submit passes User Validation | Host | |
| PENDING | UPLOADING | Basic Content Precheck passes, storage available | Host | |
| PENDING | FAILED | Precheck denied (`type`, `size`, `storage_full`) | Host | No |
| PENDING | FAILED | No progress for 10 min (`abandoned`) | Sweeper | Yes |
| UPLOADING | VALIDATING | Last chunk received | Host | |
| UPLOADING | FAILED | No chunk for 2 min (`stalled`) | Sweeper | Yes |
| UPLOADING | FAILED | Uploader cancels (`cancelled`) | Host on request | No |
| UPLOADING | FAILED | Token invalid (`unauthorized`) | Host | No |
| FAILED | UPLOADING | Same submission ID resubmitted, reason retryable | Host | |
| VALIDATING | AVAILABLE | Integrity and Server Content Validation pass | Host | |
| VALIDATING | FAILED | Validation denied (`invalid_image`) | Host | No |
| VALIDATING | FAILED | Integrity retries exceeded (`integrity`) | Host | Yes |
| VALIDATING | UPLOADING | Integrity mismatch, fewer than 3 so far: bytes discarded, offset 0 (**DU-13**; MVP §5.4 step 14 requires the re-send but the table has no row for it) | Host | |
| AVAILABLE | DELETED | Uploader or host deletes, or event expires | Host | |

`DELETED` is a tombstone in the change log; the item row is removed (DU). Any other transition is illegal and must be rejected and tested.

**Invariants (MVP §6)**
1. Only the host writes item state.
2. Nothing is visible to anyone but the uploader before `AVAILABLE`.
3. The payload of an `AVAILABLE` item never changes.
4. Every state change and its change-log row commit in one transaction.
5. Every loop has a cap: transfer retries 3, integrity retries 3, outbox 10 attempts or 24 h.

---

## 8. Security

1. **Pinned TLS (D6).** Per-event self-signed certificate. Apps accept only the certificate whose SHA-256 equals `fp` from the QR code or link; no CA validation, no hostname check. Nearby joins: trust on first use after showing the host code; pin thereafter. A mismatch is a hard failure (`fingerprint_mismatch`), never a prompt to continue.
2. **Tokens.** Join token: 128 random bits per event, in the QR code. Session token: 256 random bits, opaque, stored hashed, bound to attendee and event, TTL 12 h (browser: 2 h, scope `read`). Upload token: opaque, bound to submission ID, user and maximum size, valid until the item leaves `UPLOADING` (MVP §5.4 step 10). All compared in constant time.
3. **Credentials (D7):** §5.5. Invalid or expired → treat as guest, do not reject.
4. **Decode worker (MVP §5.4 step 15).** A separate child process per job or pool: no network use, input by file handle, time limit 10 s, memory limit 1 GB, killed on overrun; a crash yields `invalid_image` and an error report with format and size bucket only (PH §8). MVP hardening level is process isolation with limits (**DU-36**); OS sandbox profiles (macOS sandbox, Windows AppContainer) are an M3 task.
5. **Metadata strip (MVP §5.4 steps 2, 15).** Remove location data only, without re-encoding: EXIF GPS IFD, XMP and IPTC location fields. Keep orientation, colour profile, capture time, HDR gain map. For HEIC, overwrite the GPS data in place so box offsets do not change (**DU-37**). If the host's strip changes bytes, it stores the new hash as `sha256` (keeping `claimed_sha256`) and returns it in the ack (**DU-20**).
6. **Reachability test (MVP §5.10 step 2).** The sources do not say how. Default (**DU-38**): the host checks its address is private and not on a known captive or isolated range, sends an mDNS query and listens for any peer reply for 2 s, and attempts a connection to its own LAN address through the default gateway where the OS allows. Result is `ok`, `isolated` or `unknown`; `unknown` shows the same warning as "likely".
7. **Passwords.** Stored as Argon2id. Sent only over pinned TLS; never accepted over plain HTTP (D12).
8. **Browser guests.** Read scope only; every write endpoint rejects `read` sessions and all plain-HTTP requests.
9. **Never remotely configurable:** TLS, validation rules, the image allowlist (PH §4).

---

## 9. UI rules (MVP §8)

- **Accent: soft indigo.** Primary fill `#A5B4FC`, hover `#818CF8`, text on primary `#1E1B4B` (light) or `#131516` (dark); links `#4F46E5` light, `#A5B4FC` dark; selected tab `#6366F1` light; badge tint `#E6E9FE` / ink `#3730A3`. Light page `#F5F3EE`, surface `#FFFFFF`, text `#16171A`, muted `#5B5F66`, line `#E2DFD8`. Dark page `#131516`, surface `#1C1F21`, text `#E9E7E2`, muted `#A9ADB2`, line `#2E3235` (screens: Accent). Mockups are light only; build both from these tokens and follow the system theme (**DU-39**).
- **Status colours are semantic only:** green, amber, red in small labels and icons (Live, Waiting, Not sent) and destructive actions, always with a word or icon. Light: ok `#DCFCE7`/`#14532D`, warn `#FEF3C7`/`#78350F`, error `#FEE4E2`/`#912018`.
- **Photos are the hero:** mixed grid with one featured tile per time group. Only a people row and one sort menu. No search bar or chip rows ("Option A").
- **Latest host post is pinned** above the tabs.
- **Nothing from inside an event before joining.** Join screens use generated cover art: indigo rings seeded from the event ID. Port the algorithm in screens → "3 · Join event" script exactly so every platform draws the same cover.
- **Destructive actions sit apart:** End event in a separated bottom section with a one-line explanation; Remove attendee and Delete settings in a row's ⋯ menu; Delete in the phone viewer at the bottom of the save sheet.
- **Verified badge** (indigo check) appears on other attendees in lists, never next to your own name in headers. Your own status shows only in Account.
- **Role labels (Decided, Q7):** "Host", "Uploader", "Guest" (the `viewer` role is always labelled "Guest"), e.g. "Join as Uploader", "Join as Guest", "Guest · browser". All labels come from one string table so a later rename needs no code change.
- Missing states (empty, loading, error, permission) follow `screen-inventory.md` → "Derived states" (**DU-40**).

---

## 10. Non-functional targets (MVP §9, provisional)

| Area | Target |
|---|---|
| Attendees connected | 50 |
| Photos per event | 1,000 |
| Max photo size | 50 MB |
| Host storage | 20 GB default (5 GB quick session) |
| Upload, 12 MP, LAN | < 5 s |
| Validation + preview, 12 MP HEIC | < 1 s on a 2020-era laptop |
| Compatible copy, 12 MP HEIC | < 2 s first request, instant after |
| Index push | < 2 s p95 after `AVAILABLE` |
| Join, scan to gallery | < 5 s with a 500-item index |
| Event lifetime | ≤ 7 days including retention |
| Host OS | macOS 13+, Windows 10/11 |
| Clients | iOS 16+, Android 9+ (API 28), current Safari, Chrome, Firefox, Edge |

**Tunable constants** (compiled defaults; PH §7 remote config within compiled bounds): chunk 1024 KB (bounds 256 KB–4 MB), stall timeout 120 s, transfer retries 3, backoff base 500 ms, outbox 10 attempts / 24 h, uploads 30 per 5 min. Fixed: pending timeout 10 min, failed cleanup 1 h, sweeper 60 s, session TTL 12 h, credential life 7 days (refresh at 2 days left), count batch 30 s, hot-score half-life 15 min, preview 1024 px, compatible JPEG quality 92.

**Observability (MVP §10).** Correlation ID = submission ID on every request, log line and state change. Event log table. Host panel counters: attendees connected, uploads in progress, failed uploads in the last hour, storage used. Debug export of the event log to a file, never sent automatically.

---

## 11. Telemetry (D16; PH §2, §4, §5, §7)

**Rules**
1. Metadata only. Never: photo bytes, previews, file names, hashes, EXIF, post text, event names or plain IDs, descriptions, links, display names, Wi-Fi names, IP addresses, GPS, exact attendee counts, exact file sizes, free text.
2. One allowlisted schema in `crates/proto`; anything not in it is dropped before sending. A test proves forbidden properties are dropped on every platform.
3. Buckets: attendees `1-5`, `6-20`, `21-50`, `50+`; sizes `<5MB`, `5-20MB`, `20-50MB`. Timings exact.
4. No location; IP capture and GeoIP off at the project.
5. `distinct_id` = random install ID. Group `locale_event` = salted hash of the event ID. Anonymous events, no person profiles.
6. Opt-in, two separate switches, both default off: usage data, crash reports. Asked once in onboarding; changeable in Account.
7. Queue offline with capture-time timestamps; send when online. On desktop the facade owns a small SQLite queue if `posthog-rs` does not persist.
8. Analytics never blocks the product. Flags fall back to compiled defaults.
9. The guest web page sends nothing; the host counts guests in `event_summary`.

**Facade.** `telemetry.track(EventName, typed props)` in the core: checks consent, applies allowlist and buckets, hashes IDs, adds `app_version`, `platform` (`ios`, `android`, `macos`, `windows`), `client_kind` (`phone`, `desktop`), then hands to the platform SDK.

**Events** (`object_action`; properties in PH §5): `onboarding_completed`, `join_attempted`, `join_succeeded`, `join_failed`, `host_check_completed`, `hotspot_offered`, `hotspot_accepted`, `hosting_started`, `quick_session_converted`, `event_extended`, `event_summary`, `host_again_used`, `upload_completed`, `upload_failed`, `outbox_expired`, `photo_saved`, `compatible_copy_generated`, `download_all_completed`, `index_push_measured` (1 in 20), `index_resync`, `host_offline_seen`.

**Flags and remote config (PH §7):** `browser_guests_available` (on), `quick_session_enabled` (on), `reachability_test_mode` (`full`), `compatible_copy_strategy` (`on_demand`), `desktop_download_all` (on), `transfer_config`, `outbox_config`, `rate_limits` (values in §10). At event start the host reads values (fresh, else cached, else compiled), freezes protocol-affecting ones into `event.config_json` with `config_version`, and sends them in `config` at join. Out-of-range values are ignored.

**Errors (PH §8):** crashes and handled exceptions with crash consent; scrub paths; tag with stage (`join`, `upload`, `validate`, `convert`, `sync`, `expiry`) and reason code. **Surveys (PH §9):** host after `event_summary` (1–5 plus one multiple choice), attendee once after first `photo_saved`; no free text; only when online after the event.

---

## 12. Register of defaults (all "Default (unconfirmed)")

| ID | Default | Where |
|---|---|---|
| DU-1 | Units and ID conventions | top |
| DU-2 | Retired: phone UI decided as Flutter (TR-1) | §3 |
| DU-3 | React + TypeScript for desktop and guest web; assets embedded. **Decided** (TR-4) | §3 |
| DU-4 | Rust crate choices | §3 |
| DU-5 | `ImageCodec` trait with per-OS back ends | §3 |
| DU-6 | App service: small Rust + SQLite service, Postmark, Fly.io. **Decided** (TR-13) | §3 |
| DU-7 | Repository layout | §3 |
| DU-8 | Email-only sign-in codes. **Decided** (TR-13) | 4.1 |
| DU-9 | Password events always require the password | 4.3 |
| DU-10 | Browser session TTL 2 h | 4.3 |
| DU-11 | `locale://join` link from the guest page | 4.3 |
| DU-12 | Same-user duplicate → `FAILED` `duplicate` with `duplicate_of` | 4.4 |
| DU-13 | `VALIDATING → UPLOADING` on integrity mismatch (first two) | §7 |
| DU-14 | Preview quality 80, orientation applied | 4.4 |
| DU-15 | Change log kept for the event's life | 4.6 |
| DU-16 | Access type and password not editable after start | 4.7 |
| DU-17 | Auto-resume a paused event on launch with the same key, token, port | 4.7 |
| DU-18 | Continuous hot-score decay | 4.12 |
| DU-19 | A download = explicit save; logged without item ID. **Decided** (TR-14) | §5.3 |
| DU-20 | "Original" = the file after the location strip; host re-hash if its strip changes bytes. **Decided** (TR-10) | §8.5 |
| DU-21 | Photo time = time it became available | 4.12 |
| DU-30 | The whole host API, sync messages, error format, single-port sniffing. **Decided** (Q2, TR-11) | §5 |
| DU-31 | Credential format and device-key proof of possession. **Decided** (Q2, TR-9) | §5.5 |
| DU-32 | mDNS service type and TXT fields. **Decided** (Q2) | §5.6 |
| DU-33 | Client core API surface. **Decided** (Q2) | §5.7 |
| DU-34 | Added tables and columns | §6 |
| DU-35 | Event folder layout | §6 |
| DU-36 | Decode worker isolation level | §8.4 |
| DU-37 | In-place GPS overwrite for HEIC | §8.5 |
| DU-38 | Reachability test method | §8.6 |
| DU-39 | Dark theme from tokens | §9 |
| DU-40 | Derived UI states for unmocked screens. **Decided** (Q8) | §9 |
| DU-42 | Riverpod for Flutter state management | §3 |
| DU-41 | Scope placed in milestones the build order does not name: extend and shorten, Past events and Host again, quick-session limits → M3; posts → M1; Download all → M2; telemetry schema → M1 | work breakdown |

---

## 13. Conflicts between sources (MVP doc followed)

| # | Conflict | Resolution |
|---|---|---|
| C1 | Older design doc states `New`, `In Progress`, `Queued`, `In Review`, `Completed`; MVP §6 uses `PENDING`, `UPLOADING`, `VALIDATING`, `AVAILABLE`, `FAILED`, `DELETED` | MVP names. No `Queued`, no `In Review` |
| C2 | Design doc scans, then strips, then encrypts at rest with server keys; MVP has no scanner and no app-level encryption (D5, D6) | MVP |
| C3 | Notes: fixed 3 s retry. Design doc and MVP: exponential | Exponential with jitter, 3 attempts |
| C4 | Design doc sends likely duplicates to manual review; MVP returns the existing item | MVP (DU-12 for mechanics) |
| C5 | MVP §5.4 step 14 asks for an integrity re-send up to 3 times; the MVP §6 table has no transition for it | DU-13 |
| C6 | MVP §5.4 step 15 has the host strip metadata; step 16 moves the original "unchanged"; SC1 says "byte-identical to the original" | DU-20; open question Q5 |
| C7 | MVP §5.12 says nothing about what a person saved is sent to the host, but counts need saves, and the mock event log shows "Leo downloaded a compatible JPEG" | DU-19: counted and logged without the item ID; open question Q6 |
| C8 | MVP §7 `event_log.client_kind` lists `ios`, `android`, `web`; `attendee.client_kind` and §5.3 add `desktop` | Four values everywhere |
| C9 | PH §5 `upload_failed.reason_code` omits `abandoned`; `join_failed.reason` uses names not in the MVP doc | Schema uses the §5.2 lists |
| C10 | MVP §5.1 says "email or phone"; no provider chosen | Email only, Postmark (TR-13) |
| C11 | Mockup "Desktop: joined event" script holds an unused chip list (Most saved, Mine); MVP §5.12 says no chip rows | No chips |
| C12 | Mockup sort hint says "Saved most by everyone"; MVP label is "Most downloaded" | Label "Most downloaded"; hint as mocked |
| C13 | Guest mockup URL `/e/rooftop` (a name slug); MVP URL uses the event ID | Event ID |
| C14 | The phone copy calls the desktop app "Locale Host", but it is also the attendee client | One desktop app; both apps are named "Locale"; phone copy says "Locale for Mac or Windows". Development identifiers `com.example.locale` until real ones are supplied (Q11) |
| C15 | Build order (MVP §11) does not place extend, Host again, posts or telemetry in a milestone | DU-41 |
| C16 | Earlier MVP D10 said the phones embed the core through UniFFI; PostHog plan lists the iOS and Android SDKs | MVP D10 now says Flutter with flutter_rust_bridge (TR-1). Telemetry uses the PostHog Flutter SDK on phones, behind the same schema facade |
