# Locale: design tradeoffs

Nov 13, 2026 · Living record · Companion to `Locale MVP Flows.md`

## Purpose

This document records every major design decision where real alternatives were weighed: what the options were, how they compare, what was chosen, and what would make us change our minds. The MVP doc's decisions table (D1–D17) says *what* was decided in one line each. This document says *why this option over the others*, so a later reader, or an agent, does not reopen a settled question without new information.

**How to add an entry.** Copy the template at the end. Give it the next TR number. Keep the comparison to the options someone seriously considered. When a decision changes, do not delete the old entry: mark it **Superseded by TR-n** and add the new one.

**Statuses**
- **Decided:** a person has chosen. Build on it.
- **Proposed:** a default the build will use unless a person overrules it (marked "Default (unconfirmed)" in `docs/agentic/spec-digest.md`).
- **Open:** not yet chosen; the entry gives a recommendation.
- **Superseded:** replaced by a later entry.

## Index

| ID | Decision | Choice | Status | Related |
|---|---|---|---|---|
| TR-1 | Phone app UI framework | Flutter, one codebase, over the Rust core | Decided, Nov 13, 2026 | D10, D12; digest DU-2 |
| TR-2 | How phone hosting runs inside a Flutter app (M5) | Rust host core owned by a native foreground service, not by the Flutter UI | Decided, Nov 13, 2026 | D10, M5 |
| TR-3 | Language of the shared core | Rust | Decided (D10) | D10 |
| TR-4 | Desktop app shell | Tauri with a React UI (reconfirmed after TR-1) | Decided, Nov 13, 2026 | D10, TR-1 |
| TR-5 | First host form factor | Desktop first, phone in M5 | Decided (D10) | D1, D10 |
| TR-6 | Browser access for people without the app | View-only page over plain HTTP, open and invite-link events only | Decided (D6, D12) | D6, D12, D17 |
| TR-7 | When to make the compatible JPEG | On first request, cached | Decided (D11) | D11 |
| TR-8 | Product analytics | PostHog EU cloud, opt-in, metadata only | Decided (D16) | D16 |
| TR-9 | Credential shape | Signed credential bound to a device key | Decided, Nov 13, 2026 | D7; digest DU-31 |
| TR-10 | What "original" means after stripping location | The file after a location-only strip | Decided, Nov 13, 2026 | SC1; digest DU-20 |
| TR-11 | Network ports for TLS and HTTP | One port, protocol chosen by the first byte | Decided, Nov 13, 2026 | D6; digest DU-30 |
| TR-12 | How the Windows host is built and tested | macOS first; Windows on GitHub Actions runners | Decided, Nov 13, 2026 | D10 |
| TR-13 | Sign-in channel, code provider, service hosting | Email codes, Postmark, one Fly.io machine (EU) | Decided, Nov 13, 2026 | D7, MVP §5.1; DU-6, DU-8 |
| TR-14 | Counting saves without tracking who saved what | Save reports counted per photo; log names the person, never the photo | Decided, Nov 13, 2026 | MVP §5.12; DU-19 |
| TR-15 | HEIC decoding on Windows hosts | Bundled libheif, internal builds until licensing advice | Decided, Nov 13, 2026 | D11, MVP §12 |

---

## TR-1. Phone app UI framework

**Status:** Decided, Nov 13, 2026. Replaces the earlier default of native SwiftUI and Compose (digest DU-2).

**Context.** The phone apps are the main client: join, browse, upload and save photos, and from M5 host events. All protocol and data logic (pinned TLS, sync, outbox, uploads, cache) lives in the shared Rust client core. The UI layer only draws screens and calls the core. The question is which toolkit draws the screens, and how much platform-specific code the choice adds.

**Options**

| | Native: SwiftUI + Jetpack Compose | **Flutter** | React Native | Kotlin Multiplatform / Compose Multiplatform |
|---|---|---|---|---|
| UI codebases | 2 (Swift, Kotlin) | **1 (Dart)** | 1 (TypeScript) | 1 shared, or 2 native |
| Look and feel | Exactly the platform's | Draws its own widgets; very close to native | Uses platform widgets | Native, or Compose drawn everywhere |
| Calling the Rust core | UniFFI (generated Swift and Kotlin) | **`flutter_rust_bridge` (generated Dart, async and streams)** | Possible, less mature | Overlaps with what the Rust core already does |
| OS features (picker, Save to Photos, permissions) | Direct calls | Plugins, or our own small Swift and Kotlin plugins | Same as Flutter | Direct calls |
| Testing by agents | XCTest and Android instrumented tests; both need a simulator or emulator | **Widget and golden tests run headless, no simulator; integration tests on a simulator** | Jest headless; Detox on simulators | Mixed |
| Share code with the desktop and web UIs | No | No (unless the desktop also moves to Flutter, see TR-4) | Yes, with a React desktop and guest page | No |
| Work in the build plan | About 12 phone UI tasks duplicated across iOS and Android | **About half of that** | Similar to Flutter | Similar to native |

**Decision: Flutter**, calling the Rust client core through `flutter_rust_bridge`, with a small set of native plugins we own.

**Why**
- One UI codebase for iOS and Android halves the phone UI work, and halves the number of places the two phones can drift apart in behaviour or copy.
- Widget and golden tests run without a simulator. That makes agent verification faster and more reliable, which matters for a hands-off build. Golden tests also suit the generated cover art, which must look identical on every device.
- Moving the UI to Flutter changes nothing in the core. TLS, sync, the outbox and the image pipeline stay in Rust either way.
- Phone hosting (M5) is as possible in Flutter as in native code (see TR-2).

**Costs we accept**
- **Platform features still need native code.** Each one goes in a small plugin we own, so it can be tested and fixed directly:
  - `locale_media`. iOS: `PHPickerViewController` with `preferredAssetRepresentationMode = .current`, so HEIC files arrive unconverted (MVP §5.4 step 1), and add-only Save to Photos. Android: the system photo picker and saving through MediaStore.
  - `locale_net`. iOS: priming for the Local Network permission and detecting a denial. Android: a multicast lock so mDNS discovery receives packets.
  - `locale_host_service` (M5): the Android foreground service and the iOS keep-awake hosting mode (TR-2).
- **We don't rely on general-purpose picker plugins** for the original file. Some convert HEIC or strip data by default, and that would silently break "original quality". This is a deliberate rule, not a temporary gap.
- **A third UI language.** The project now has Rust (core), TypeScript (desktop and guest page) and Dart (phones).
- **Close to native, not identical.** Flutter draws its own widgets. With the custom soft-indigo design (MVP §8) this barely matters, because the app doesn't use stock platform styling anyway.
- **Slightly larger apps and a little more memory** for the Flutter engine. The M0b battery measurement covers this.

**What would make us revisit**
- The picker or Save to Photos plugins prove unreliable for byte-exact originals.
- M0b or M5 finds hosting needs so much platform code that the UI is a small share of the phone work.
- `flutter_rust_bridge` cannot keep up with the core's API (streams, cancellation, background threads).

**Consequences for the build**
- Digest §3 stack table, §5.7 (core API through `flutter_rust_bridge`) and the repository layout (`apps/mobile`, `packages/locale_*`).
- In the work breakdown, the separate iOS and Android UI tasks merge into Flutter tasks, with per-platform plugin tasks. Verification uses `flutter test` and `flutter test integration_test` on a simulator and an emulator.
- The M0a and M0b test apps are Flutter apps, so the spikes measure the real stack.

---

## TR-2. How phone hosting runs inside a Flutter app (M5)

**Status:** Decided, Nov 13, 2026.

**Context.** From M5 a phone can host an event: it runs the same Rust host core the desktop runs (D10). The OS must keep that process alive and its network server reachable for hours.

**Options**
1. Start the host core from the Flutter UI code.
2. **Start the host core from a native component that outlives the UI:** an Android foreground service, and on iOS a hosting mode that keeps the app in the foreground with the screen awake. Flutter only shows status and controls.

**Decision: option 2.**

**Why.** Android can destroy the Flutter UI (its activity and engine) while a foreground service keeps running. If the server belongs to the UI, the event drops offline with it. If the service owns the core, hosting continues and the Flutter screen reconnects to the running core when the user returns. The Rust core runs on its own threads and needs no Dart code to serve requests.

**What Flutter does not change.** The real limits come from the OS and are identical in native code:
- iOS suspends a background app's network server whatever the framework. Hosting needs the app in the foreground (MVP §12). M0b measures how this behaves.
- Apps cannot turn on the hotspot on iOS, and Android restricts it. Users turn it on in Settings.
- Recent Android versions require a declared foreground-service type, and some types have runtime limits. M0b must confirm which type fits a long-running local server.

**What would make us revisit.** M0b shows iOS foreground hosting is not viable. Phone hosting would then be Android-only, or wait for Phase 2 or 3 transports. That is independent of the UI framework.

---

## TR-3. Language of the shared core

**Status:** Decided (D10).

| | **Rust** | Go (gomobile) | Kotlin Multiplatform | C++ |
|---|---|---|---|---|
| Runs on macOS, Windows, iOS, Android | Yes | Yes; iOS and Android through gomobile | Yes; desktop through the JVM or native | Yes |
| Memory safety when parsing untrusted files | Yes, without a garbage collector | Yes, with a garbage collector | Yes | No |
| Bindings to Flutter, Swift, Kotlin, desktop | `flutter_rust_bridge`, UniFFI, direct in Tauri | gomobile (Java and Objective-C only); weaker for Flutter | Native on Android; Swift interop improving | Hand-written |
| Fit for an embedded phone host | Small runtime, predictable memory | Larger runtime, garbage-collection pauses | JVM on desktop is heavy | Good, but unsafe |

**Why Rust.** One library has to run inside a desktop app, inside two phone apps and in tests, while parsing photos from strangers. Rust gives memory safety without a runtime, and it has first-class bindings to both Tauri (natively) and Flutter. Go is the documented fallback (D10), but its mobile bindings don't reach Flutter cleanly.

**Revisit if** a needed crate is missing for something essential (for example an mDNS or HEIC component) and a Go or platform library is far ahead.

---

## TR-4. Desktop app shell

**Status:** Decided (D10); reconfirmed Nov 13, 2026 after TR-1.

| | **Tauri + React** | Electron + React | Flutter desktop | Native (SwiftUI + WinUI) |
|---|---|---|---|---|
| Calling the Rust core | Direct; the backend *is* Rust | Through a Node add-on | `flutter_rust_bridge` | UniFFI plus Windows bindings |
| App size and memory | Small; uses the OS web view | Large; ships Chromium and Node | Medium | Small |
| Shares UI code with | The guest web page (React) | The guest web page | The phone app (after TR-1) | Nothing |
| Platforms | macOS, Windows | macOS, Windows | macOS, Windows | Two separate apps |

**Why Tauri.** The host core is Rust, and Tauri's backend is Rust, so the desktop app calls the core with no bridge. The guest web page has to be a plain, small web page served from the laptop with no internet, so React is needed for the guest page regardless. Tauri lets the desktop reuse those components.

**Follow-up after TR-1: Flutter desktop instead? Decided Nov 13, 2026: no, keep Tauri.** With phones on Flutter, the desktop's joined-event screens (gallery, viewer, posts, attendees) could have shared Dart code with the phones instead of React code with the guest page. Rejected because:
- The desktop's own screens (create event, capability checks, hosting window, past events) exist only on the desktop, so sharing saves less than it seems.
- The guest page needs React regardless; Flutter's web output is too heavy for an offline page served from a laptop.
- Tauri calls the Rust host core with no bridge.

**Cost accepted:** the joined-event screens are built twice, in Dart for phones and in React for desktop and the guest page. The shared logic (filters, sorts, grouping) lives in the Rust client core, so only drawing code is duplicated.

**Revisit if** desktop and phone UIs drift apart in ways users notice, or the duplicated screens become a large share of UI work.

---

## TR-5. First host form factor

**Status:** Decided (D10, rev 2 of the MVP doc).

| | **Desktop host first** | Phone host first |
|---|---|---|
| Proves the pipeline and sync without OS limits | Yes | No: iOS suspension, battery and storage limits apply from day one |
| Fits parties, hikes, festivals | Poorly; someone must bring a laptop | Well |
| Fits classrooms, work sessions, vendors | Well | Well |
| Risk to the product target | Phone hosting stays unproven until M5 | Answered first |

**Why desktop first.** It removes the hardest platform limits while the pipeline, sync and UX are proven. Phone hosting remains the product target, and the risk is handled by an early, parallel phone-host spike (M0b) on the same core, so M5 is a wrapper, not a rewrite.

**Revisit if** M0b shows phone hosting is easy on both platforms; M5 could then move earlier.

---

## TR-6. Browser access for people without the app

**Status:** Decided (D6, D12, D17).

| | No browser access | **View-only page over plain HTTP** | Full browser client over HTTPS |
|---|---|---|---|
| "Any device" without installing | No | Yes | Yes |
| Security | Best | Traffic is visible on the local network; limited to view and download | Browsers can't trust or pin a self-signed certificate; needs a real certificate, so a domain and internet |
| Uploads at original quality | — | Not offered: iOS Safari may convert HEIC; uploads stop when the tab is backgrounded; no durable outbox | Same browser limits |

**Why.** A view-only page makes "any device" real at almost no cost. Its weaker transport is contained: read-only sessions, open and invite-link events only (passwords never cross plain HTTP), and a host switch to turn browsers off. Uploads stay app-only because browsers can't keep the original-quality promise.

**Revisit when** browser background uploads and HEIC handling improve, or if local HTTPS without a public certificate becomes practical.

---

## TR-7. When to make the compatible JPEG

**Status:** Decided (D11). Switchable by the `compatible_copy_strategy` flag (PostHog plan §7).

| | **On first request** | At upload (eager) |
|---|---|---|
| Host CPU and storage | Only for photos someone needs as JPEG | For every HEIC photo |
| First view in Chrome or on Windows | Up to 2 s wait the first time | Instant |
| Upload completion time | Unchanged | Longer |

**Why on demand.** Most viewers (iOS, macOS, Android 9+, Safari) can show the original, so most photos never need a JPEG. **Revisit** if field data shows first-request latency is a common complaint; the flag allows switching without a release.

---

## TR-8. Product analytics

**Status:** Decided (D16). Full plan: `Locale PostHog Plan.md`.

| | No analytics | **PostHog EU cloud** | Self-hosted PostHog | Several vendors (analytics, crash, flags) |
|---|---|---|---|---|
| Can measure the success criteria in the field | No | Yes | Yes | Yes |
| Privacy story for a "no cloud" product | Strongest | Acceptable with opt-in, metadata only, IP capture off | Strong | Weaker; more parties |
| Operating cost | None | Low (free tier) | Servers and upkeep | Several bills and SDKs |
| SDKs for iOS, Android and Rust | — | Yes | Yes | Varies |

**Why.** The provisional targets need field numbers. One vendor covers analytics, errors, flags and surveys on every platform. Strict rules (an allowlisted schema, opt-in, buckets, no content) keep the brand promise. **Revisit** self-hosting in Phase 2 if the privacy story needs "no third-party analytics at all".

---

## TR-9. Credential shape

**Status:** Decided, Nov 13, 2026 (open question Q2, digest DU-31).

| | Bearer credential (MVP doc as written) | **Credential bound to a device key, plus a signed join proof** |
|---|---|---|
| Offline check by the host | Yes | Yes |
| A malicious host can replay a visitor's credential at another event, including as that user's `host` role | Yes | No: the proof names the event and certificate and is signed by a key that never leaves the device |
| Work | Smallest | One more key pair per device and one signature per join; no user-visible step |

**Why.** Hosts are other users' laptops, not trusted servers, so whatever a visitor hands a host must be useless anywhere else. **Revisit** never in the weaker direction. A stronger option (per-event credentials from the service) needs internet at join time, which events often lack.

---

## TR-10. What "original" means after stripping location

**Status:** Decided, Nov 13, 2026 (open question Q5, digest DU-20).

| | Keep all metadata | **Strip location only, otherwise byte-exact** | Re-encode to strip everything |
|---|---|---|---|
| "Byte-identical" (SC1) | Yes, to the camera file | Yes, between uploader's upload and every download; pixels identical to the camera file | No |
| Location privacy | GPS leaks to everyone at the event | Protected | Protected |
| Quality | Full | Full, with HDR and colour profile kept | Lossy |

**Why.** It keeps both promises that matter, full quality and no location leaks, and makes SC1 testable. The product copy says "location removed, nothing else changed".

---

## TR-11. Network ports for TLS and HTTP

**Status:** Decided, Nov 13, 2026, with the interface contract (Q2, digest DU-30).

| | Two ports (TLS for apps, HTTP for browsers) | **One port; protocol chosen by the first byte** |
|---|---|---|
| Firewall rules the host must allow | Two | One |
| QR URL | Must carry or imply both ports | `http://ip:port/...` works for both |
| Complexity | Simple listeners | A small detection step on each connection |

**Why.** The host's firewall prompt is one of the most common setup failures (MVP §12). One port means one rule, and the QR URL in MVP §5.2 needs no change.

---

## TR-12. How the Windows host is built and tested

**Status:** Decided, Nov 13, 2026 (open question Q3).

**Context.** Windows 10/11 is a required host platform (MVP §9), but development happens on a Mac. The riskiest Windows parts (bundled libheif, firewall rule, Mobile Hotspot, mDNS, worker sandbox) cannot run on macOS.

| | Windows machine or VM for agents | **GitHub Actions Windows runner** | Defer all Windows work to before M4 |
|---|---|---|---|
| Unattended verification | Yes, if agents get shell access | Yes, on every pushed `ci/*` branch | No |
| Setup | A machine or VM licence, plus remote access | Workflow files; Actions on the existing GitHub repo | None |
| Catches Windows breakage early | Yes | Yes | No; a late surprise |
| OS dialogs (firewall prompt, hotspot) | Partly | No | No |

**Decision:** macOS first, with Windows tests on the GitHub Actions runner from task S6. One manual pass on a real Windows machine for firewall, hotspot and an isolated network (M3-11).

**Why.** It's cheapest to set up, runs without a person once agents may push `ci/*` branches, and keeps Windows regressions visible from M0a. Windows-only code sits behind two traits (`ImageCodec`, OS helpers), so macOS work never waits.

**Costs we accept.** Agents need permission to push branches (never `main`). CI runners can't show OS dialogs, so those checks stay manual.

**What would make us revisit.** Runner time becomes costly, or Windows failures can't be reproduced on CI. Then add a Windows VM.

---

## TR-13. Sign-in channel, code provider, service hosting

**Status:** Decided, Nov 13, 2026 (open question Q4).

**Context.** MVP §5.1 asks for "email or phone with a one-time code" from the app service. The service only issues credentials (D1, D7), so it is small, but it must be online, send codes reliably and protect the signing key.

| Channel | Email | SMS | Both |
|---|---|---|---|
| Cost per code | Near zero | Per message, varies by country | Both |
| Fraud and rules | Low | SMS pumping fraud; per-country sender rules | Both |
| Reach | Everyone with email | Everyone with a phone | Both |

| Provider | **Postmark** | Resend | AWS SES |
|---|---|---|---|
| Focus | Transactional email only | Developer-focused, transactional and marketing | General-purpose |
| Setup | Domain verification, HTTP API | Domain verification, HTTP API | AWS account, sandbox exit request, more configuration |

| Hosting | **Fly.io, one machine with a volume** | Managed container platforms (Render, Railway) | A VPS |
|---|---|---|---|
| SQLite on persistent disk | Yes | Varies | Yes |
| Secrets for the signing key | Yes | Yes | Manual |
| EU region | Yes | Varies | Yes |
| Upkeep | Low | Low | OS patching is ours |

**Decision:** email codes only, sent through Postmark, from one small Fly.io machine in an EU region with a SQLite volume and daily snapshots. The production signing key lives only as a Fly secret.

**Why.** Email avoids SMS cost and fraud entirely. Postmark is built only for transactional mail, which is all we send. Fly gives a persistent disk for SQLite, secrets and an EU region, matching the PostHog choice, with little upkeep. The sender is behind a trait, so changing provider is one file.

**Costs we accept.** Sign-up needs an email address. A single machine means brief downtime during deploys. That's acceptable because hosts verify credentials offline, so events keep working.

**What would make us revisit.** Users ask for phone sign-in, deliverability problems, or more than one region needed.

---

## TR-14. Counting saves without tracking who saved what

**Status:** Decided, Nov 13, 2026 (open question Q6, digest DU-19).

**Context.** "Most downloaded", "Hottest" and "Saved by 12" need counts per photo. MVP §5.12 says nobody, including the host, sees who downloaded a photo, and that the "Saved by me" filters are computed on the device. An app that already cached a photo can save it without a new request.

| | Count only full-file fetches | **Apps report saves; host counts per photo, logs the person without the photo** | Host records who saved which photo |
|---|---|---|---|
| Counts mean "saved" | No: views and saves mixed | Yes | Yes |
| Host learns who saved which photo | No | No | Yes; breaks MVP §5.12 |
| Event log still shows activity per person | No | Yes ("Leo downloaded a compatible JPEG") | Yes |

**Decision:** the middle option.

**Why.** It makes the sorts mean what their labels say, keeps the event log useful for debugging, and never stores a person-to-photo link.

**Costs we accept.** A host who watches the log live and correlates timing could guess. That's accepted for the MVP; the log is local to the host and deleted with the event.

**What would make us revisit.** Users or reviewers find the log entries too revealing. Then drop the actor from download log rows.

---

## TR-15. HEIC decoding on Windows hosts

**Status:** Decided, Nov 13, 2026 (open question Q10).

**Context.** Windows hosts must decode iPhone HEIC photos to validate them and make previews and compatible copies. Windows only decodes HEIC with the Microsoft HEVC extension, which users must install. Bundling a decoder raises HEVC patent licensing questions (MVP §12).

| | **Bundle libheif** | Require the Microsoft HEVC extension | Reject HEIC on Windows hosts |
|---|---|---|---|
| Works out of the box | Yes | No; an extra install, sometimes paid | Yes, but breaks success criterion 2 |
| Licensing exposure | Needs advice before commercial distribution | None for us | None |
| Consistency with the macOS decoder | Good | Good | — |

**Decision:** bundle libheif. Windows builds are marked internal-only until licensing advice is received.

**Why.** Development and a private field test are not blocked, and hosting works without asking users to install codecs. Legal advice is only needed before public distribution.

**Fallback, ready if advice is negative:** require the Microsoft HEVC extension. The capability check detects it and blocks hosting with install instructions. The `ImageCodec` trait makes this a backend swap.

**What would make us revisit.** The licensing advice, or a royalty-free path becoming clear.

---

## Template

```
## TR-n. <Decision in a few words>

**Status:** Decided | Proposed | Open | Superseded by TR-m. <date>

**Context.** What problem forced a choice, in two or three sentences.

**Options**
| | Option A | Option B | Option C |
|---|---|---|---|
| <criterion that mattered> | | | |

**Decision:** <the option>.

**Why.** The deciding reasons, most important first.

**Costs we accept.** What the choice makes worse.

**What would make us revisit.** Concrete signals, not "if things change".

**Consequences.** Which docs, tasks or code this changes.
```
