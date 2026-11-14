# Locale MVP: screen inventory

Nov 13, 2026 · Companion to `spec-digest.md` ("digest"; DU-n = a Default (unconfirmed) recorded there). Drafted by a Sonnet subagent, reviewed and corrected by the orchestrator. Section D gives the default design for every state and screen that has no mockup. **Role labels are decided (Q7):** wherever this file says the role names are a TODO, use "Host", "Uploader" and "Guest" (never "Viewer" in the UI).

Sources: `docs/Locale MVP Flows.md` ("MVP", primary, wins on conflict) and `docs/agentic/source/screens.md` ("screens.md", mockup outlines). Citations: "MVP §8" = section 8 tables; "MVP §5.x step n"; "Dn" = decision; "screens.md: <title>" = mockup section.

Coverage meaning: Full = every item in the MVP §8 "Contents" cell is on a mockup (states may still be missing, listed per entry). Partial = some contents or a distinct sub-screen is missing. None = no mockup.

Decision records (no detail needed):
- Accent: soft indigo chosen (screens.md: Accent). Primary fill #A5B4FC, text on it #1E1B4B in both themes; light links/tabs #4F46E5 (dark: #A5B4FC); tint #E6E9FE (dark #2A2D5C). Status colours (ok/warn/err) only on small labels. Matches MVP §8 UI rules.
- Event page: Option A chosen (screens.md: EventOptions). People row is the only visible filter, sort moves into the section header, no search bar or chips. Matches MVP §5.12. Options B and C are rejected.

Mockup file to canvas number: Main=1 Welcome, Home=2, Join=3, Event=4, Viewer=5, Uploads=6, HostCreate=7, QuickCheck=7b, HostLive=8, HostPast=9, GuestJoin=10, GuestPhoto=11, DeskEvents=12, DeskEventView=13. All files are `<Name>.dc.html`. Phone mockups are 390x844; desktop 1280 wide.

---

## A. Coverage table

| Surface | Screen (MVP §8) | Mockup | Coverage | What is missing |
|---|---|---|---|---|
| Desktop | Sign in | none | None | Whole screen (sign in / sign up, one-time code entry) |
| Desktop | Events | DeskEvents | Full | Join steps after tapping Join (password prompt, host code, cover art) have no desktop screen |
| Desktop | Joined event | DeskEventView | Partial | Sort menu open state; Posts and Attendees tab content; download-all progress; role variants |
| Desktop | Create event | HostCreate | Partial | Password field (shown only as a radio); hotspot fix; folder picker; password vs browsers-checkbox rule; Edit-details form |
| Desktop | Quick session check | QuickCheck | Full | Only the "likely blocked" state; no all-good, firewall-prompt or hotspot-failed state |
| Desktop | Hosting window | HostLive | Full | Extend "Custom…" dialog, End confirm, expiry notification, host-offline, Keep going, Add details form |
| Desktop | Past events | HostPast | Full | Row ⋯ menu contents; Delete settings confirm; empty state |
| Desktop | Event view (your hosted event) | none | None | Photos/Posts/Attendees tabs for the host's own event; Edit event form (HostLive has only a button) |
| Desktop | Attendees | HostLive (Attendees table) | Partial | ⋯ menu contents (Remove, delete uploads option); "View all 23" target; client kind has no "desktop" sample |
| Desktop | Event log | HostLive (Event log) | Partial | Export flow; "View all"; log has no "download version" or removal entries beyond samples |
| Phone | Welcome | Main | Full | Sign up / sign in forms are not mocked |
| Phone | Home | Home | Full | Guest variant (sign-up prompt on Host an event); empty state; host-offline; M5 warning dialog |
| Phone | Event page, host's phone | Event (viewer = host phone) | Full | Delete affordance on grid tiles; Write a post composer |
| Phone | Join | Join | Full | Wrong-password error; removed/ended/offline; QR-vs-Nearby difference for host code |
| Phone | Event page | Event | Full | Ended, expiring-soon, empty gallery, loading, error states; header links |
| Phone | Photo viewer | Viewer | Full | Role variants for Delete; Android/older-device default; download error |
| Phone | Upload picker | none | None | Whole screen (system picker, inline precheck errors) |
| Phone | Uploads | Uploads | Full | Empty state; "Expired" label naming |
| Phone | Account | none | None | Whole screen (display name, verified status, credential expiry) |
| Guest web | Join | GuestJoin | Full | Error states (event ended, host unreachable, browsers not allowed) |
| Guest web | Gallery | none | None | Photos/Posts/Attendees read-only tabs |
| Guest web | Photo view | GuestPhoto | Partial | Safari variant (original shown); JPEG/PNG original (single download); loading and error |

Counts: Full 12, Partial 5, None 5 (22 screens).

Mockups with no row in MVP §8: AccentCompare and EventOptions (decision records only). Also, no §8 row exists for these mocked sub-views: desktop Join screen (needed after DeskEvents Join), QR "Show full screen" view, Add details / Edit event forms, and the phone Event page tabs as separate rows (covered in the Event page row).

---

## B. Screen entries

### Desktop app (host and attendee)

Common to all desktop pages with a nav bar (DeskEvents, DeskEventView, HostCreate, HostPast; HostLive has none in outline): nav links Events (DeskEvents), Host (HostCreate), Past events (HostPast); account button "SR" (aria "Account: Sam R."), destination not mocked (screens.md: Events, Joined event, Create, Past). MVP §8: Navigation: Events, Host, Past events.

#### D-01 Sign in
- Purpose: sign in or sign up; verified account needed to host or upload; viewer join works without (MVP §8).
- Mockup: none.
- Elements: per MVP §8 only: sign in, sign up. Phone Welcome (P-01) is the closest visual reference.
- States: none mocked.
- Actions and navigation: Sign up = email or phone plus one-time code, needs internet (MVP §5.1 step 3). Result: signed credential (§5.1 step 4). Desktop uses the same sign-in (§5.1 step 8). Continue as guest is a phone/app concept (§5.1 step 1); whether desktop offers it is unspecified.
- Data from host: none (app service only, D1, D7). Local: credential, guest profile.
- Flows: 5.1; D7.
- Gaps: whole screen; code-entry UI; error states (bad code, no internet); when it appears (first launch? on Host click? on Upload?) unspecified; credential-expired fallback to guest (5.1 step 6).

#### D-02 Events
- Purpose: list joined events plus hosted one, join by link, join from Nearby.
- Mockup: DeskEvents (tweak `hosting` true/false).
- Elements: nav; "Events" title and subtitle; if hosting: Hosting banner "You're hosting <name> on this computer" with note that joining others is allowed, link "Open live event"; "Your events" rows (name, role pill, meta "Live on this computer · 23 here · 48 photos", action link; hosted row action "Open live event", joined row "Open"); "Nearby" list headed `On "<network>" · open and password events` (name, lock icon if password, meta "Open/Password · n here · hosted by X", Join); note "Invite-only events don't show here. Ask for the link."; "Join with a link" input plus "Join event"; "On this computer you can" three bullets (drag to add, save originals/compatible to folder, download all originals).
- States: hosting vs not hosting mocked. Not mocked: empty Your events, empty Nearby, loading/scanning, network lost, host-offline row, permission-denied (local network).
- Actions and navigation: Open live event → HostLive (D-06). Open (joined) → DeskEventView (D-03). Join (nearby) and Join event (link) → DeskEventView directly in mockup. MVP requires an access check first (password, host code, §5.3 steps 2-4); that screen is missing.
- Data from host: for joined events: event name, host name, role, live count, photo count (from local index). For Nearby: mDNS advert (name, access type, host name, here-count); no photo count for password events (D17).
- Flows: 5.3 (Desktop app join), 5.11; D12, D15, D17.
- Gaps: no desktop Join/password/host-code screen; link-paste error state; "Nearby" locked-row password flow; Nearby fingerprint trust-on-first-use display (§5.3 step 2). MVP §8 lists "Your events (joined, plus the one you host marked Hosting)" - matches.

#### D-03 Joined event
- Purpose: browse, save and add photos in an event joined from this computer.
- Mockup: DeskEventView (tweak `platform` macOS/Windows).
- Elements: nav; link "All events"; title, role pill ("Uploader"), meta "Hosted by X · times · photos delete <time>"; buttons "Download all originals (62)" and "Add photos"; pinned post card "Pinned · Priya (host) · 7:40 PM" with link "All posts (3)"; tab buttons Photos 62, Posts 3, Attendees 14; people pills with counts (Everyone first); section header "7:00 – 7:30 PM · 18 photos" and sort button "Newest"; grid of tiles (initial, time, saved check, one featured tile, one selected tile); drop-zone hint "Drop photos anywhere to add them to this event. JPEG, HEIC or PNG."; photo panel: format badge, "12 MP · HDR", "Photo by Priya · Thu 7:12 PM", "Saved by 6 people", "Save as" radio group (Original · HEIC with size; Compatible · JPEG with note), "Save to folder", folder path with "Change".
- States: macOS vs Windows default mocked (macOS Original, Windows Compatible; Windows folder `Downloads\Locale\<event>`). Not mocked: sort menu open, ranked (Most downloaded, Hottest) layout, Posts tab, Attendees tab, uploading tile, host offline, event ended, viewer role (no Add photos), loading, error, empty. Script also defines chips (All, Most saved, Not saved yet, Mine, Newest first) but the outline shows only pills plus the sort button; treat chips as unused (MVP §5.12: no chip rows).
- Actions and navigation: All events → D-02. Tabs switch content. Pill filters by one person. Sort button opens the sort menu (not mocked; options per MVP §5.12: Newest, Oldest, Most downloaded, Hottest; Show: Everything, Saved by me, Not saved yet, My photos). Add photos / drop → upload flow 5.4 (outbox on this computer). Save to folder → saves chosen version to folder. Change → folder picker. Download all originals → resumable, skips saved files (MVP §5.3 Desktop step 4). All posts → Posts tab.
- Data from host: event record (name, host, times, expires_at), role, pinned/latest post, posts, roster (with photo counts), items (uploader, time, format, dimensions, HDR flag, size of original and compatible, saved-by count, download counts), previews, original and compatible bytes on demand.
- Flows: 5.3, 5.4, 5.5, 5.6, 5.12; D11, D12.
- Gaps: Compatible size note shown even if original is JPEG/PNG (MVP §5.5 step 7: only one choice then); "default by device" per OS only mocked for macOS/Windows; Download-all progress UI; the two radios both appear checked in the outline but are exclusive in the script; no sort menu; Posts and Attendees content reuse phone designs (P-05) but not mocked at desktop size.

#### D-04 Create event
- Purpose: start a Quick session in one click or fill the full event form and host.
- Mockup: HostCreate (tweak `alreadyHosting`).
- Elements: nav; "Host an event"; "Photos are stored on this computer and deleted when the event expires."; if hosting: "Live", "You're hosting <name>" explanation, link "Open live event"; else: Quick session card ("Share right now…", tags: Invite link only, Ends when you end it · 12 h max, Deleted at the end, Photos only, link "Start quick session"); "Or set up a full event"; Event details (Name, Description, Starts, Ends, Links optional); Who can join (Access: Open "Shows in Nearby", Invite link only "QR code or link" [default], Password "App only, no browsers"; checkbox "Let browsers join as viewers. People without the app can view and download, but not upload."); Storage and deletion (Delete photos: 24 hours after end [default], When the event ends, 3 days after end; "You can extend later, up to 7 days total."; Space to use 20/10/50 GB; folder `~/Locale/Events`); "Ready to host?" checks (Network, Firewall with "Allow connections" fix, Space, Power) with ✓ or !; "No shared Wi-Fi?" hotspot/travel-router hint; "Start hosting" and "Fix 1 item to start." (button blocked while any check fails).
- States: not hosting and already hosting mocked; capability check with one failure (Firewall) mocked. Not mocked: all checks pass (enabled button), hotspot-needed, low battery/space failures, field validation errors, password entry field, loading while starting.
- Actions and navigation: Start quick session → QuickCheck (D-05). Allow connections → OS firewall rule (Windows prompt, macOS accept prompt, MVP §5.2 step 4). Start hosting → starts server and shows HostLive (D-06) (target not linked in mockup). Open live event → HostLive.
- Data from host: local only (own computer: network, free space, power, firewall); signed-in account for host name.
- Flows: 5.2, 5.10, 5.11; D14, D15, D17.
- Gaps: no password field although access type Password exists (MVP §5.2 step 2); MVP says browser-guest option is not offered for password events, mockup keeps checkbox visible (conflict, MVP wins: hide or disable when Password); hotspot offer (§5.2 step 5) only as text hint; same form is reused for "Add details" conversion and Edit event (§5.10 step 5, §5.7) but pre-filled variant not mocked; storage-folder picker not mocked; battery rule ("above 50%") vs mock "Plugged in" wording; external links input is one text field with sample "Playlist · Building entry" (format unspecified); date/time pickers not mocked (text fields).

#### D-05 Quick session check
- Purpose: reachability and capability check before a quick session starts (shown only when needed).
- Mockup: QuickCheck.
- Elements: title "Starting your quick session"; four checks: Network ✓, "Can phones reach this computer?" ! ("A test from this network could not reach your computer…"), Firewall ✓, "Power and space" ✓ (battery 64%, 5 GB reserved); warning "Phones probably can't reach you on this Wi-Fi" with hotspot advice; buttons Cancel, "Try anyway", "Turn on hotspot and start"; footer summarising defaults ("Invite link only · ends when you end it, 12 hours at most · photos deleted at the end · photos only. You can add details later without changing the QR code.").
- States: only the isolated-network warning mocked. Not mocked: all good (MVP: continue without showing), firewall prompt inline, hotspot start failure, Mac variant (Internet Sharing needs a second connection, otherwise travel router).
- Actions and navigation: Cancel → back (target not linked). Try anyway → HostLive (D-06). Turn on hotspot and start → HostLive (mockup).
- Data from host: local only (reachability test result, network name/IP, battery, space).
- Flows: 5.10 step 2; D14.
- Gaps: macOS variant of hotspot action; what Cancel returns to; no loading state while test runs.

#### D-06 Hosting window
- Purpose: run and control the live event or quick session; the only place to end it.
- Mockup: HostLive (tweaks `mode` event/quick session, `expiringSoon`).
- Elements: name (quick: "Sam's session · 3:40 PM" plus "Rename"); pill "Live" or "Live · Quick session"; buttons "Add photos", "Edit details"; "Join with this code": QR, "Scan with the Locale app, or any phone camera to view in a browser.", "Show full screen", "Print"; host code `4F2A · 9C1E · 77B0` (for Nearby joins) and `192.168.1.24:7420 · Rooftop-WiFi`; counters: Here now 23 (11 iPhone · 9 Android · 3 browser), Photos 48 (2 uploading now), Failed in last hour 1 (Unsupported file type), Storage 3.8 of 20 GB; if event: "Photos delete in <countdown>", expiry date "up to 7 days total (until Thu, Nov 20)", buttons +1 hour, +6 hours, +1 day, Custom…, "Everyone is told when you extend or shorten."; if quick: "Ends when you end it", "Auto-ends at 3:40 AM (12 h max). Photos are deleted as soon as it ends.", "Add details" ("Turns it into a full event… Same QR code."); Photos: "Open gallery (48)", drop zone with "choose files" ("JPEG, HEIC or PNG up to 50 MB. Copies are added to the event; your originals stay where they are."), "Latest photos (yours first)" with Checking/percent tiles, note "Checked the same way as everyone's photos before they appear."; Attendees table (Name, Device, Role, Photos, Actions ⋯) with "View all 23", verified badge, note "Verified account · use ⋯ to remove someone"; New post (input "Post to everyone…", Post; "Shows in everyone's Posts tab."); Event log with Export; separated danger section "End this event" / "End this session" with explanation and button "End event" / "End session".
- States: event vs quick session; expiring soon (countdown 52 minutes, amber box; no other change). Not mocked: host offline/paused (MVP §5.7 step 4), lost network, storage near full or full, extension rejected (cap reached), session ended, empty attendee list, loading.
- Actions and navigation: Add photos / drop / choose files → host upload through loopback (MVP §5.4 Host uploads), copies file. Edit details → event form (not mocked). Show full screen / Print → QR view (not mocked). +1h/+6h/+1d → extend (§5.9); Custom… → custom end time and retention (not mocked). Add details → converts quick session to event, same QR (§5.10 step 5). Rename → quick only. Open gallery → anchor #gallery (host's own event view, D-08; no mockup). View all 23 → anchor #all (D-09). Row ⋯ → remove attendee (not mocked). Post → publishes to Posts tab. Export → event log export. End event/session → deletes everything (§5.7), no confirm mocked.
- Data from host: event record (name, kind, ends_at, retention, expires_at, extension_count), join URL/IP/port/network name, QR payload (URL with token and fingerprint), host code, attendee roster (name, client kind, role, verified, photo count), counters (here now by client kind, photos, uploading, failures last hour, storage used/allocation), items (latest photos, state: Checking/uploading %), event log rows (time, text, denial flag).
- Flows: 5.2 step 9-10, 5.4 (Host uploads), 5.7, 5.8 step 6-7, 5.9, 5.10, 5.11, 5.12; D13, D14, D15.
- Gaps: no Keep going / "expiring soon" host notification (§5.8 steps 6-7, §5.9 step 1) beyond an amber countdown; Posts: host can post but delete-post not mocked; Attendee roles show "Host/Uploader/Viewer" (role-name TODO; Event page uses "Guest"); "up to 7 days total" cap is from event creation (§5.9 step 2) while the mock text only gives a date; Event log is the MVP §8 "Event log" screen (D-10), Attendees table the "Attendees" screen (D-09); no nav bar in the outline although §8 has one; the host window does not mention retention shortening UI beyond Custom…; MVP §5.2 step 10 says the host keeps the computer from sleeping (shown in D-04 Power check only).

#### D-07 Past events
- Purpose: show settings of ended events and start Host again.
- Mockup: HostPast.
- Elements: nav; "Past events" with note "Only settings are kept. Photos, attendees and the event index were deleted when each event ended."; link "New event"; per event: name, "Ended <date> · photos deleted <date>", access, retention, browsers; buttons "More actions for <name>, including delete settings" and "Host again"; footer note explaining Host again gives a new empty event with new QR code and everyone joins again.
- States: populated list mocked (3 samples). Not mocked: empty, host-again disabled while an event is live (MVP §5.11 step 3: replaced by "You're hosting <name>"), delete confirm.
- Actions and navigation: New event → HostCreate (D-04). Host again → new event from saved settings (§5.9); target not linked. ⋯ → Delete settings (menu not mocked).
- Data from host: local only (saved settings per ended event: name, description, links, access, retention, browser setting, ended date, deleted date).
- Flows: 5.9 (Reopen), 5.11; D13, D15.
- Gaps: quick sessions are not saved here unless converted (§5.10 step 8); live-event variant not mocked; Host again flow step (does it open the form pre-filled or start immediately?) unspecified. Mock rows omit description and links although §5.9 says they are kept.

#### D-08 Event view (your hosted event)
- Purpose: browse own event with the same tabs as the phone app; edit event.
- Mockup: none (HostLive links "Open gallery (48)" to an unmocked anchor; Edit details button exists).
- Elements: per MVP §8: Photos, Posts, Attendees tabs same as phone app (see P-05); edit event.
- States: none mocked.
- Actions and navigation: Edit event → form (D-04 pre-filled, not mocked).
- Data from host: same as D-03/P-05 (host's own data, via loopback).
- Flows: 5.6, 5.7 (edit), 5.12; D15.
- Gaps: entire screen; relation to D-03 (likely the same layout with host powers: delete any photo/post, post) not specified; navigation from HostLive undefined.

#### D-09 Attendees
- Purpose: roster with verified badges, client kind, Remove.
- Mockup: HostLive "Attendees" table (partial).
- Elements: columns Name, Device, Role, Photos, Actions; verified icon; ⋯ per row ("More actions for <name>"); sample rows: Mac/iPhone/Android/Browser; roles Host, Uploader, Viewer.
- States: populated list only.
- Actions and navigation: ⋯ → Remove attendee (revokes token, closes socket, adds to removed list, optional delete of their uploads: MVP §5.7). Menu not mocked. View all 23 → #all.
- Data from host: roster (name, client kind iOS/Android/web/desktop, role, verified flag, photo count).
- Flows: 5.7, 5.11; D15.
- Gaps: no dedicated full-list screen; remove confirmation and "delete their uploads" option; "desktop" client kind (§5.3) not in sample. Default: the host's own rows (host device and host's phone) have no Remove.

#### D-10 Event log
- Purpose: recent joins, denials, uploads, deletes; export.
- Mockup: HostLive "Event log" section.
- Elements: rows (time, text; denials in red e.g. "Denied: unsupported file type (Alex, .mov)"); Export button.
- States: populated only.
- Actions and navigation: Export → file export (format, location unspecified).
- Data from host: event log rows.
- Flows: 5.5 step 9 (download entries), 5.7, 5.9 step 3, 5.8; D16 not needed.
- Gaps: export format and destination; log retention and size; "view all" for long logs; per-photo download version entry ("Leo downloaded a compatible JPEG") sits close to §5.12 "nobody sees who downloaded a photo". Resolved by digest DU-19: the log row names the person and the version but never the photo. Export: JSON Lines file through a save dialog (default).

---

### Phone app (iOS, Android)

#### P-01 Welcome
- Purpose: entry; choose guest, sign up or sign in.
- Mockup: Main (canvas 1).
- Elements: logo "Locale"; collage of 4 photos with badge "Original · 3.4 MB"; headline "Share full-quality photos with the people around you."; "Photos stay on devices at the event, not in the cloud. Everything is deleted when the event ends."; buttons Sign up, Sign in, Continue as guest; note "Guests can view and download. Uploading and hosting need a verified account."
- States: single state. Not mocked: loading, offline (sign up needs internet, MVP §5.1 step 3).
- Actions and navigation: Sign up / Sign in → account flows (not mocked, one-time code). Continue as guest → creates local guest profile (§5.1 step 2) then Home (P-02); no link in mockup.
- Data from host: none (app service for sign up; local for guest).
- Flows: 5.1; D7.
- Gaps: sign up and sign in screens (email/phone, code); display-name entry for guest (profile has display name, §5.1 step 2); error and no-internet states.

#### P-02 Home
- Purpose: list joined events, scan QR, host entry, nearby events.
- Mockup: Home (tweaks `you` attendee/hosting, `hosting` desktop (MVP)/phone (M5)).
- Elements: logo; account button (initials, aria "Account"); "Your events" rows (thumbnail, name, role badge, meta "Live · 23 here · 48 photos" or "Live · 23 here · hosted from your MacBook"); button "Scan QR code to join"; if not hosting: "Quick session" and "Host an event" with caption (MVP: "In this version, sessions and events are hosted from Locale Host on a Mac or Windows computer. We'll send you the link."; M5: "Host from this phone. Keep Locale open while hosting. One hosted event at a time."); if hosting: "You're hosting <name> from Sam's MacBook. You can still join other events. End this one there to host something new."; "Nearby: On this Wi-Fi" rows (initial, name, meta "Open · 12 here · Hosted by X" or "Password · …" with lock icon).
- States: attendee, hosting; MVP-copy vs M5 copy mocked. Not mocked: guest (not signed in) variant, empty Your events, empty Nearby, nothing on Wi-Fi/permission denied, host offline row, send-link confirmation.
- Actions and navigation: event row → Event page (P-05). Account → Account (P-09). Scan QR → camera scan (not mocked) → Join (P-04). Quick session / Host an event → MVP: explains hosting runs on Locale Host and offers to send the download link by email (§5.2 step 1); guests asked to sign up first. Nearby row → Join (P-04).
- Data from host: for joined events: from local index per event (name, role, here-count, photo count, live/offline). Nearby: mDNS advert (name, host name, access type, here-count; password events show no counts after join? mock shows "8 here" for a password event on Nearby).
- Flows: 5.3, 5.10, 5.11; D14, D15.
- Gaps: Nearby password row shows "8 here" while MVP §5.3/§D17 says counts only for open and invite-link events (conflict; MVP wins); role badge copy TODO (MVP §8 role rename); send-link dialog; guest prompt; Nearby never lists invite-link or quick sessions (matches MVP).

#### P-03 Event page, host's phone
- Purpose: host's phone view of own event; same as everyone's with host extras.
- Mockup: Event (tweak `viewer` = host phone).
- Elements: all P-05 elements plus strip "Host / Post and delete here · end or extend from Sam's MacBook" (MVP wording: a "You're hosting" strip); Posts tab: "Write a post" button; posts note "Everyone sees your posts here. They are deleted with the event."
- States: as P-05; viewer toggle only.
- Actions and navigation: Write a post → composer (not mocked). Delete on any photo → via Viewer (P-07). End/extend/edit/remove: not available (MVP §5.11 table).
- Data from host: as P-05; host role flag.
- Flows: 5.11 (steps 6-7), 5.7; D15.
- Gaps: strip copy differs from MVP ("You're hosting"); composer; post delete UI; no tile-level Delete; Viewer (P-07) does not show its host variant.

#### P-04 Join
- Purpose: pre-join screen with generated cover, access check and Join.
- Mockup: Join (tweaks `access` password/invite link, `eventId` drives cover art).
- Elements: Back; live label ("Live now" or "Live now · 62 photos · 8 here" for invite); event name; times ("Today 7:00 – 10:00 PM"); host avatar and "Hosted by Priya"; access line ("Password required · counts appear after you join" / "Invite link · photos delete 24 h after it ends"); generated indigo ring cover (seeded from event ID); if password: lock icon, "Event password" field with Show toggle; "First time joining this host. Check this code matches the host's screen:" with `4F2A · 9C1E · 77B0`; "Photos, posts and who's here stay hidden until you've joined."; button "Join as Uploader"; note "You can view, download and add photos. Your phone needs Local Network access to reach the host."
- States: password, invite mocked. Not mocked: open event variant (counts shown), guest role ("Join as Viewer"), wrong password, rate limited, removed, event ended, host offline, local-network permission denied, joining spinner.
- Actions and navigation: Back → Home. Show → toggles password visibility. Join as Uploader → access check (§5.3 steps 2-5) then Event page (P-05). No link in mockup.
- Data from host (before join, D17): event name, host name, times, access type, event ID (cover), and for open/invite events photo count and attendee count; host-cert fingerprint (host code) for Nearby; nothing from inside the event.
- Flows: 5.3 ("What the join screen shows"); D6, D12, D17.
- Gaps: host code block is shown in both variants; MVP §5.3 step 2 shows it only for Nearby first-time joins (QR/invite joins use the fingerprint in the link) - mock conflicts; "Join as <role>" label depends on role names TODO (MVP §8); "Uploader" label only (guest: Viewer); password events are app-only so browser join not offered; Local Network note is text only, no permission prompt or denied screen.

#### P-05 Event page
- Purpose: main event view; browse photos, posts, attendees.
- Mockup: Event (tweaks `viewer`, `state` live/extended/hostOffline, `tab`, `sort`, `menuOpen`, `progress`).
- Elements: Back to home; Uploads link with badge (e.g. 3) → Uploads; "Event details and menu" button; title; meta "Fri 7 PM – 1 AM · Hosted by Sam · deletes Sat 2 PM"; if host offline: banner "Host offline." + "Photos saved on this phone still open. New uploads will send when the host is back."; if not on Posts tab: pinned latest post (label "PINNED · SAM · 8:12 PM" or "LOCALE · 7:30 PM" for system notice) with "All posts"; tabs Photos 48 / Posts n / Attendees 23; Photos tab: people row (Everyone first, initials, one selected), section header (e.g. "9:30 – 10:00 PM" + count) and sort button ("Newest" or "… · 1 filter"), sort menu (SORT BY: Newest, Oldest, Most downloaded, Hottest, each with hint; SHOW: Everything, Saved by me, Not saved yet, My photos), mixed grid (featured tile first; badges: "HEIC · HDR", rank ↓n / ↑n; own uploading tile with percent and "Only you see this"), link "Add photos" → Uploads; Posts tab: posts (author, time, text; system notices in tint), note "Only the host can post here. Posts are deleted with the event."; Attendees tab: rows (initial, name, indigo check if verified, meta e.g. "Uploader · 14 photos", "Host · Mac", "Guest · browser"), note "Verified accounts can upload. Guests can view and download."
- States: live, extended (system post "The host extended the event. Photos now delete Sun, Nov 16 at 2:00 PM." pinned and in Posts; header deletion time changes), hostOffline mocked; all tabs and four sorts mocked; menu open. Not mocked: event ended (event ended change, MVP §5.7), expiring soon (one-hour notice, §5.8 step 6), empty gallery, loading/first sync, error, removed from event, reconnecting; viewer (guest) role variant (no Add photos).
- Actions and navigation: Back → Home. Uploads badge → Uploads (P-08). Menu → event details/menu (contents not specified; not mocked). Tab buttons switch tabs. People pill filters one person. Sort button opens menu; picking closes it. Tile → Viewer (P-07). Add photos → Uploads in mockup; MVP §5.4: opens picker from this page (P-08/Upload picker).
- Data from host (via local index, §5.6): event record (name, times, host, links, expires_at), latest post and all posts (including system notices), roster (name, role, verified, photo count, client kind), items (uploader, time, format, HDR flag, preview, download_count, hot_score, own pending uploads), saved state is local only ("Saved by me", "Not saved yet" are computed on device, §5.12), counters (photos, posts, attendees), connection state.
- Flows: 5.5, 5.6, 5.9 (notice), 5.12, 5.8; D13, D17.
- Gaps: header omits links (MVP §5.6 header: links); expiring-soon and ended states; meta menu contents; role label copy ("Host", "Uploader", "Guest" vs "Viewer" in host table); one-person filter only; "Posts 4" count in tab hides pinned dup; "Everyone" initial text "All"; Event options B/C are rejected, do not build.

#### P-06 Upload picker
(Numbering: IDs are P-01..P-09 in the order of these entries, which differs slightly from the §8 table order: Upload picker is P-06 and Photo viewer is P-07.)
- Purpose: multi-select photos from the camera roll with inline precheck errors.
- Mockup: none (Uploads row "Screen Recording.mov" shows the "Not sent" result of a failed precheck).
- Elements: per MVP §8: multi-select from camera roll; precheck errors inline. System pickers (iOS `PHPickerViewController` with `preferredAssetRepresentationMode = .current`; Android photo picker, MVP §5.4 step 1).
- States: none mocked.
- Actions and navigation: select → Client Content Precheck (JPEG, HEIC/HEIF, PNG, under 50 MB, strip location) → outbox → Uploads. Opened from an event page (MVP §5.11 step 9).
- Data from host: local only (device photo library). Host has the event's allocation check later.
- Flows: 5.4 (client side), 5.11; D3, D11.
- Gaps: whole screen; photo-library permission (limited access on iOS) and denied state; error copy for wrong type or over 50 MB (Uploads sample: "Only photos (JPEG, HEIC, PNG) can be shared"); Live Photo behaviour (still only); no copy for oversize.

#### P-07 Photo viewer
- Purpose: full photo, metadata, save in Original or Compatible, delete.
- Mockup: Viewer (tweak `format` HEIC/JPEG).
- Elements: Close; uploader avatar and name ("Jordan"), "Fri 9:41 PM · 7 of 48"; format badge, "12 MP", "HDR" (HEIC only), "Saved by 12"; sheet "Save photo"; if HEIC: radio group "Version to save": "Original · HEIC Best for this phone, The exact file Jordan shared, with HDR · 3.4 MB" (default) and "Compatible · JPEG Opens in any app. Slightly lower quality, no HDR · 5.1 MB"; if JPEG: single "Original · JPEG … Opens everywhere · 4.2 MB"; button "Save to Photos"; note "Saved photos stay after the event ends. Everything else is deleted."; set-apart button "Delete this photo".
- States: HEIC and JPEG mocked. Not mocked: loading (preview then full image), download error/hash mismatch (MVP §5.5 step 6), saved confirmation, Photos permission denied, Compatible default on devices that cannot decode HEIC, role variants for Delete (shown unconditionally), PNG, compatible copy being generated.
- Actions and navigation: Close → Event page. Save to Photos → downloads chosen version, saves to camera roll (§5.5 steps 7-8). Delete this photo → tombstone (§5.7); only uploader or host should see it; no confirm mocked.
- Data from host: item (uploader, time, index "7 of 48", format, dimensions, HDR flag, saved-by count (aggregate), original size and compatible size and hash), preview, full image (original or compatible). "Saved" status is local.
- Flows: 5.5, 5.7, 5.12; D11.
- Gaps: Delete visibility by role not mocked; copy "Saved by 12" derives from download_count; compatible size only known after it is generated (MVP §5.5 step 4: made on first request; size may be unknown); no swipe between photos specified (the "7 of 48" implies it).

#### P-08 Uploads
- Purpose: outbox list with state, progress and Retry.
- Mockup: Uploads.
- Elements: Back to event; title "Uploads", event name; summary "3 sending / 1 waiting / 2 need attention"; rows (thumb, filename, badge, optional progress bar, detail, optional action): "62%" "2.1 of 3.4 MB" [Cancel]; "18%" "Resumed after connection dropped"; "Checking" "Received by host, being checked"; "Waiting" "Will send when the host is reachable"; "Not sent" "Only photos (JPEG, HEIC, PNG) can be shared" [Remove]; "Stopped" "Stopped after 10 tries" [Retry]; "Shared" "Visible to everyone · 9:41 PM"; footer "Uploads keep going if you leave this screen, and resume where they stopped if the connection drops. Location data is removed before sending."
- States: mixed list mocked. Not mocked: empty, all done, host offline (banner), event expired (MVP §5.8 step 2 "Expired"), storage full on host (FAILED reason), per-item detail.
- Actions and navigation: Back → Event page. Cancel (active upload) → removes from outbox (not in MVP, mockup-only). Remove (rejected file) → removes row. Retry → resubmits with same submission ID (§5.8 step 3). Rows persist across app kill (D3).
- Data from host: per-item state echoes (PENDING/UPLOADING/VALIDATING/AVAILABLE/FAILED reason), but list is local outbox; host returns state on resubmit.
- Flows: 5.4, 5.8, 6 (item state model); D3.
- Gaps: MVP outbox states are Waiting for connection, Uploading (n%), Failed (reason), Expired; mock uses Checking, Stopped, Shared, Not sent, Cancel, Remove, which are not in MVP §5.8 step 5. Use the mockup labels with this mapping (default): percent = Uploading; "Checking" = host state VALIDATING; "Waiting" = Waiting for connection; "Not sent" = Failed with a final reason (precheck or host reason code), action Remove; "Stopped" = Expired (10 tries or 24 h, or event ended), action Retry when the event is still live; "Shared" = AVAILABLE, kept in the list until the screen is left. Cancel maps to `DELETE /uploads/{submission_id}` (reason `cancelled`, MVP §6); "My uploads" link from extension notice (§5.9 step 5) not mocked; mock has no host-offline banner here.

#### P-09 Account
- Purpose: display name, verified status, credential expiry; only place your own verified status shows.
- Mockup: none (Home has an account button with aria "Account").
- Elements: per MVP §8 only: display name, verified status, credential expiry.
- States: none mocked.
- Actions and navigation: unspecified (sign out? refresh credential? edit name?). Opened from Home account button (screens.md: Home).
- Data from host: none (local credential).
- Flows: 5.1; D7; MVP §8 UI rule "Verified badges belong to attendees".
- Gaps: whole screen; actions; offline-expired credential state (drops to guest, §5.1 step 6); sign out; PostHog opt-in toggle location (D16) unspecified.

---

### Guest web page (served by the host over HTTP, view only)

Both mockups draw a browser address-bar strip ("Not secure", `192.168.1.24:7420/e/rooftop`); this is browser chrome, not part of the page.

#### W-01 Join
- Purpose: browser guest enters a display name and joins (open or invite-link events only).
- Mockup: GuestJoin.
- Elements: "Locale"; "You're invited to" + event name; "Hosted by Sam · 48 photos · 23 here"; "Photos and who's here appear after you join."; "Your name" input (sample "Leo") with note "Shown to the host in the attendee list."; button "View photos"; "View only in the browser" with "You can browse and download photos. To add photos and save originals straight to Photos, use the app."; link "Get the Locale app" (https://example.com/get-locale, placeholder); footer "This page comes straight from the host's computer on this Wi-Fi, not from the internet. Photos are deleted Sat, Nov 15 at 2:00 PM."
- States: default only. Not mocked: loading, error (event ended, host unreachable, browsers not allowed, password event), generated cover art (MVP §8 and D17 require it; mock has none).
- Actions and navigation: View photos → sends join token (URL fragment) plus name, gets read-only session, opens Gallery (W-02) (§5.3 Browser guest steps 2-4). Get the Locale app → external link.
- Data from host (pre-join, D17): event name, host name, times, counts (open/invite only), event ID for cover, expires time.
- Flows: 5.3 (Browser guest join, what the join screen shows), 5.1 step 7; D6, D12, D17.
- Gaps: cover art missing in mock; MVP banner text "View only. Get the app to upload and save at full quality." appears after join (not on the join page) while the mock has different copy; times not shown (D17: times); access type not shown; random guest ID stored in browser (§5.1 step 7), no UI.

#### W-02 Gallery
- Purpose: read-only Photos, Posts and Attendees tabs for browser guests.
- Mockup: none (GuestPhoto's "All photos" link targets GuestJoin, which is wrong).
- Elements: per MVP §8: Photos, Posts and Attendees tabs (read only), photo grid from previews; banner "View only. Get the app to upload and save at full quality." (MVP §5.3 Browser guest step 5).
- States: none mocked.
- Actions and navigation: tile → Photo view (W-03). All photos link in W-03 should return here.
- Data from host: snapshot via WebSocket (§5.6 step 7): event record, posts, roster, items with previews; counts.
- Flows: 5.3, 5.5, 5.6, 5.12 (pinned post, people row, sort assumed same as phone but not stated for web); D12.
- Gaps: whole screen; whether the people row and sort menu appear on web is unstated (MVP §5.12 says "phone and desktop"). Default: reuse the phone Event page layout (P-05) read-only, with pinned post, people row and sort menu, without Show filters that need a save history, without Add photos; host-offline and event-ended handling in the browser (page reload fetches a fresh snapshot, §5.6 step 7); no Uploads link or Add photos.

#### W-03 Photo view
- Purpose: show a photo and offer Download Original or Compatible.
- Mockup: GuestPhoto.
- Elements: link "All photos"; badge "View only"; "Showing JPEG"; "Photo by Jordan · Fri 9:41 PM"; explainer "The original is HEIC, which this browser can't display, so you're seeing a compatible copy."; download links: "Download compatible · JPEG Opens anywhere · 5.1 MB" and "Download original · HEIC Exact file, with HDR · 3.4 MB · opens on iPhone, Mac, most Android"; note "On iPhone, downloads go to Files. Press and hold the photo to save it to Photos."; "Want to add your own photos?" + "Get the app".
- States: Chrome-like browser showing compatible copy only. Not mocked: Safari (original shown, MVP §8), JPEG/PNG original (single download, §5.5 step 7), loading, download error, compatible-copy-generating wait.
- Actions and navigation: All photos → mock goes to GuestJoin; should go to Gallery (W-02). Download compatible / original → file download, no byte-range resume for browsers (§5.5 step 5 "app only"). Get the app → external link.
- Data from host: item (uploader, time, format, original and compatible size), full image (original on Safari, compatible elsewhere), downloads (host logs version, §5.5 step 9).
- Flows: 5.5; D11, D12.
- Gaps: Safari variant; single-version case; navigation fix above; default highlighted option not specified for web.

---

## C. Cross-cutting gaps

Missing or unspecified everywhere, with the MVP section that requires it:

1. Local-network permission (iOS Local Network, Android nearby-Wi-Fi): only a text note on Join (screens.md: Join). No prompt-priming or denied-state screen. MVP §5.3, D8.
2. Camera permission and QR scanner: "Scan QR code to join" button on phone Home has no scanner screen or denied state. MVP §5.3 step 1, §5.2 step 9. (Desktop webcam scan is out of scope, §5.3.)
3. Photo-library permission (iOS limited access, Android picker) and Save-to-Photos permission: no mockup. MVP §5.4 step 1, §5.5 step 8.
4. Expiry warning and notifications: host "one hour before expires_at" notice with Extend, attendee "expiring soon" notice, quick session "Keep going" / "Add details" warning. Only an amber countdown on HostLive. MVP §5.8 steps 6-7, §5.9, §5.10 step 7.
5. Event ended and host-offline states: phone Event has a host-offline banner only; no ended screen, no paused indicator on host side, no offline state on Home, Join, Desktop or Guest web. MVP §5.7 step 4, §5.6 step 6.
6. Host capability-check failures: only the Firewall failure is mocked (D-04, D-05). Missing: no network, hotspot offer, low space, low battery, Windows firewall prompt, macOS prompt, Mac Internet Sharing case. MVP §5.2 steps 4-5, §5.10 step 2.
7. Sign-in, sign-up, one-time code entry, credential expiry/refresh, guest display name: no screens (D-01, P-01, P-09). MVP §5.1.
8. Error and denial states: wrong password, rate limit, removed attendee, invalid or expired token, session auto re-join, event ID unknown, hash mismatch on download, host out of storage allocation, bad file type (only the Uploads row), network loss mid-upload (only the Uploads detail text). MVP §5.3 step 4-5, §5.4 steps 9 and 14, §5.5 step 6.
9. Host management forms and confirms: Edit event details (pre-filled form), Add details conversion, Extend Custom… dialog, End event/session confirm, Remove attendee (with optional delete uploads), Delete settings, Host again, QR full-screen and Print, event log Export. MVP §5.7, §5.9, §5.10 step 5, §8.
10. Desktop join screen: password prompt, host code on first Nearby join, cover art after DeskEvents Join (mock jumps straight into the event). MVP §5.3 Desktop app join step 2, D17. No §8 row.
11. Role names: all mocks use Host, Uploader, Viewer, Guest placeholders. MVP §8 TODO: rename user-facing labels (badges, "Join as ...", attendee lists); internal `host`, `uploader`, `viewer` stay. Affects P-02, P-04, P-05, D-06.
12. "My uploads" link from the extension notice (MVP §5.9 step 5) has no screen; Uploads (P-08) is the outbox, not a list of shared photos.
13. Telemetry opt-in consent and post-event survey (PostHog): no UI. MVP D16.
14. Dark theme: tokens exist in screens.md: Accent, but every screen mockup is light. Build both from the tokens (digest DU-39). MVP §8 UI rules ("both themes").
15. Conflicts where the MVP wins (recap): host code shown on non-Nearby Join (P-04); counts shown for a password event in Nearby (P-02); browsers checkbox visible for Password events (D-04); "Leo downloaded" log entry versus aggregate-only counts (D-10); Uploads state names (P-08); GuestJoin banner copy and missing cover art (W-01).

---

## D. Derived states and screens (Default (unconfirmed), digest DU-40)

No mockup exists for the items below. Build them from the existing components and tokens (digest §9) using these rules, so that every implementer makes the same choice. Decided Nov 13, 2026 (Q8): agents build these as written; screenshots are reviewed at the M1 and M2 exits.

**General rules**
- Empty state: one muted line of text plus the primary action for that screen, centred in the content area. No illustrations.
- Loading: skeleton tiles or rows in the surface colour; a spinner only inside buttons. First sync shows the header from `/meta` at once and skeletons for the tabs.
- Error: inline, next to what failed, in the error status colours with a word and an icon, plus one recovery action (Try again, Scan again, Open settings). Never a blocking alert for a recoverable error.
- Confirmation for destructive actions: a dialog that names what is deleted, with the destructive button on the right and Cancel as default focus.
- Banners (host offline, viewing only, expiring soon, view only) sit under the header, above the pinned post, one at a time, in this priority: ended > host offline > expiring soon > viewing only.

| Item | Default design | Source of the requirement |
|---|---|---|
| Sign up / Sign in (phone and desktop, D-01) | Two steps: email field → 6-digit code field with Resend (after 30 s). Sign up adds a display-name field. Errors inline: wrong code, expired code, no internet. Desktop shows it as a dialog opened from the account button, or when Host or Add photos needs a verified account; desktop works signed out as a viewer | MVP §5.1; digest 4.1, DU-8 |
| Guest name | "Continue as guest" asks for a display name once (one field, Continue) | MVP §5.1 step 2 |
| Telemetry consent | One onboarding step after sign-in or guest name: two switches, both off: "Help improve Locale by sharing anonymous usage data", "Send crash reports"; Continue. Same two switches in Account | PH §2 rule 6 |
| Account (P-09, and desktop account menu) | Display name (editable when signed in), "Verified" with credential expiry date or "Guest", the two telemetry switches, Sign out. Expired credential: amber line "Viewing only until you're back online" | MVP §8; §5.1 step 6; §12 |
| QR scanner (phone) | Full-screen camera with a square guide and Cancel. Camera denied: text plus Open Settings. Not a Locale code: inline "This isn't a Locale event code" | MVP §5.3 step 1 |
| Local Network permission (iOS) | Before the first join, one priming sheet: "Locale needs Local Network access to reach the host", Continue → system prompt. Denied: Join shows an error with Open Settings (`local_network_denied`) | MVP §12 |
| Photo library and Save permissions | System picker needs no library permission. Save to Photos asks for add-only access; denied → error with Open Settings | MVP §5.4 step 1, §5.5 step 8 |
| Upload picker (P-06) | System picker. Precheck failures appear as "Not sent" rows in Uploads with the reason: "Only photos (JPEG, HEIC, PNG) can be shared", "Photos over 50 MB can't be shared" | MVP §5.4 step 2 |
| Join errors (P-04, W-01, desktop) | Wrong password: inline under the field. Removed: "The host removed you from this event". Ended: "This event has ended". Unreachable: "Can't reach the host. Check you're on the same Wi-Fi" with Try again. Fingerprint mismatch: "This isn't the host you scanned. Scan the code again", no way to continue. Browser not allowed or password event in a browser: "This event needs the Locale app" with the app link | MVP §5.3; digest §5.2, §8.1 |
| Join variants (P-04) | Host code block only on a first Nearby join. Button "Join as Uploader" (verified) or "Join as Viewer" (guest). Open events show counts like invite events | MVP §5.3 step 2, D17 |
| Desktop join dialog | Clicking Join in Nearby or Join event opens a dialog with the P-04 content (cover, name, times, host, access line, password field, host code on first Nearby join), then opens D-03 | MVP §5.3 Desktop app join step 2 |
| Phone "Host an event" / "Quick session" (MVP) | Sheet with the mockup caption and a button "Email me the link" (verified) or "Sign up to host" (guest); success line "Sent to <email>" | MVP §5.2 step 1 |
| Event ended (all clients) | The event page is replaced by "This event has ended. Photos you saved are still on your device." with Back to Home; the event leaves Your events. Web: same text, no actions | MVP §5.7 |
| Host offline | Phone banner as mocked; the same banner on desktop D-03 and web W-02; Home and Events rows show "Host offline" in place of "Live" | MVP §5.7 step 4 |
| Expiring soon (attendees) | Banner "Photos delete in under an hour" from the `expiring_soon` notice | MVP §5.8 step 5 |
| Expiry warning (host) | Desktop notification plus the amber expiry card as mocked, with the Extend buttons. Quick session: amber card "Auto-ends in under an hour" with Keep going and Add details | MVP §5.8 steps 6–7, §5.10 step 7 |
| Paused (host side) | After relaunch, D-06 shows a one-time line "Hosting resumed. Guests reconnect on their own." If the network is lost: amber pill "Offline" in place of "Live" | MVP §5.7 step 4; digest DU-17 |
| Removed while in the event | Event page closes with "The host removed you from this event" | MVP §5.7 |
| Viewing only (guest or expired credential) | No Add photos; in its place a muted line "Sign up to add photos" (guest) or the Account expiry line | MVP §5.1 step 6 |
| Event view of your hosted event (D-08) | The D-03 layout opened from "Open gallery", with host powers: Delete on any photo in the panel, Delete in each post's ⋯ menu, no Download-all change. Back link "Live event" | MVP §8, §5.11 |
| Desktop Posts and Attendees tabs | The phone designs (P-05) in a single centred column, 640 px max | MVP §8 |
| Desktop sort menu | The phone menu (P-05) as a dropdown under the sort button | MVP §5.12 |
| Edit details / Add details | The D-04 form pre-filled, as a dialog, without the capability checks; access type and password read-only (digest DU-16); buttons Save, Cancel | MVP §5.7, §5.10 step 5 |
| Extend Custom… | Dialog with end date-time and the "Delete photos" choice; shows the resulting deletion time and the 7-day limit; error inline when past the cap | MVP §5.9 |
| End event / End session confirm | Dialog repeating the danger-section text; button "End and delete" | MVP §5.7; §8 rule on destructive actions |
| Remove attendee | Row ⋯ menu → Remove… → dialog with a checkbox "Also delete their photos (n)"; button Remove | MVP §5.7 |
| Post composer (host's phone) and post delete | Sheet with a text field (500 characters) and Post. Each of the host's posts has a ⋯ menu with Delete | MVP §5.11 table |
| Create form details | Password field appears under the Password radio (required, 4+ characters). Choosing Password hides the browsers checkbox. Links: up to 5 rows of label + URL. Starts and Ends use native date-time inputs. Start hosting is enabled only when all checks pass | MVP §5.2 steps 2–4 |
| Capability-check failures | Same check rows as mocked, one fix button each: Network "Turn on hotspot" (Windows) or instructions (macOS); Space "Choose a smaller size"; Power "Plug in to continue" | MVP §5.2 steps 4–5 |
| QR full screen and Print | Full screen: white page with the QR code, event name and "Scan with the Locale app or your camera". Print: the same page through the system print dialog | MVP §5.2 step 9 |
| Past events details | ⋯ menu holds only "Delete settings…" with a confirm. Host again opens the D-04 form pre-filled. While hosting, Host again is disabled with "You're hosting <name>" | MVP §5.9, §5.11 step 3 |
| Download all (desktop) | The button becomes a progress line "Saving 14 of 62…" with Cancel; ends with "62 saved to <folder>" and Show in folder | MVP §5.3 Desktop app join step 4 |
| Photo viewer details (P-07) | Swipe between photos in the current sort. Delete shown only to the uploader and the host role. Compatible size shown as "JPEG" without a size until the copy exists. Preview shown first, with a thin progress bar while the full image loads | MVP §5.5 |
| Guest web join (W-01) | Add the generated cover art and the event times above the name | D17, MVP §8 |
| Guest web photo (W-03) | Safari: shows the original, with Original listed first. JPEG or PNG original: one Download button. "All photos" returns to W-02 | MVP §5.5 steps 2, 7 |
| "My uploads" link in the extension notice | Opens the Photos tab with the Show filter set to My photos | MVP §5.9 step 5 |
| Event details menu (phone header button) | Sheet with description, links, times, deletion time, host, and Leave event | MVP §5.6 header |
| Survey prompts | One card on Home after the event, when online; dismissible; no free text | PH §9 |
