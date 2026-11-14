<!-- Generated from design/mvp-ui/project by extract_mockups.py. Do not edit; re-run the script. -->

# Locale MVP UI: screen outlines

Each screen lists its tweaks (switches between alternate states), an outline of what is on screen (▸ if = shown only in some states; ▸ for each = repeated from data), and its renderVals() script, which holds the sample data and state logic. `'<scene>'` = a placeholder photo.

## Canvas notes

```
Reference mockups for the Locale MVP (docs/Locale MVP Flows.md, rev 2).

Use these as the comparison baseline while building: each screen maps to a flow in section 5 and a row in section 8.

1 Welcome → 5.1
2 Home → 5.3 (nearby), 5.10 (quick session, M5), 5.11 (your events)
3 Join → 5.3
4 Event page → 5.6, 5.9, 5.11, 5.12 (browse, sort, filter)
5 Viewer → 5.5 (Original / Compatible)
6 Uploads → 5.4, 5.8
7 Create → 5.2, 5.10, 5.11 (one event at a time)
7b Quick session check → 5.10
8 Live → 5.2, 5.4 (host uploads), 5.7, 5.9
9 Past → 5.9 (Host again)
10–11 Guest → 5.3, 5.5
12 Desktop Events → 5.3 (desktop app join), 5.11
13 Desktop joined event → 5.3, 5.4, 5.5

Tweaks: Home (attendee vs hosting; host on desktop vs phone), Event page (attendee vs host phone, session state, starting tab, sort, menu open), Join (password vs invite, event ID for cover art), Create (already hosting), Desktop Events (hosting or not), Desktop joined event (macOS vs Windows save default), Viewer (photo format), Live host (event vs quick session, expiring soon).
```

```
Comparison checklist (success criteria, section 2)

• iPhone and Android upload and download byte-identical originals (5, 6)
• HEIC viewable on Android and in Chrome; Original or Compatible offered (5, 11)
• Guests view and download, never upload (10, 11)
• New photos appear for everyone in under 2 s (4)
• Interrupted uploads resume from the outbox without re-picking (6)
• Expiry is always visible; extensions are announced (4, 8)
• Ending the event deletes everything; only settings survive (9)
```

## Screens

| File | Title on canvas | Size |
|---|---|---|
| AccentCompare.dc.html | Accent: soft indigo (chosen) | 1280×1000 |
| EventOptions.dc.html | Event page options (A chosen) | 1370×1000 |
| Main.dc.html | 1 · Welcome | 390×844 |
| Home.dc.html | 2 · Home and nearby | 390×844 |
| Join.dc.html | 3 · Join event | 390×844 |
| Event.dc.html | 4 · Event page (tabs, people and sort menu work in Play) | 390×844 |
| Viewer.dc.html | 5 · Photo viewer and save | 390×844 |
| Uploads.dc.html | 6 · Uploads (outbox) | 390×844 |
| HostCreate.dc.html | 7 · Host: create event or quick session | 1280×1080 (page) |
| QuickCheck.dc.html | 7b · Host: quick session Wi-Fi check | 1280×800 (page) |
| HostLive.dc.html | 8 · Host: live event / quick session | 1280×1340 (page) |
| HostPast.dc.html | 9 · Host: past events | 1280×800 (page) |
| GuestJoin.dc.html | 10 · Browser guest: join | 390×844 |
| GuestPhoto.dc.html | 11 · Browser guest: photo | 390×844 |
| DeskEvents.dc.html | 12 · Desktop: Events (join and nearby) | 1280×860 (page) |
| DeskEventView.dc.html | 13 · Desktop: joined event | 1280×1080 (page) |

---

## Accent: soft indigo (chosen)

File: `AccentCompare.dc.html` · Page title: Accent comparison: soft indigo

Outline:

```
▸ for each o in options:
  ## {{o.name}}
  {{o.note}}
  ▸ for each s in o.swatches:
    {{s.label}}
    {{s.hex}} · {{s.ratio}}
  ▸ for each t in o.themes:
    {{t.label}}
    Manage my uploads
    Rooftop birthday
    Host
    ● Live
    Waiting
    Not sent
    Photos 48
    Posts 4
    Attendees 23
    [button: Scan QR code to join]
    [button: Host an event]
```

Script (data and state):

```js
class Component extends DCLogic {
  renderVals() {
    const status = {
      light: { okBg: '#DCFCE7', okInk: '#14532D', warnBg: '#FEF3C7', warnInk: '#78350F', errBg: '#FEE4E2', errInk: '#912018' },
      dark: { okBg: '#163A24', okInk: '#86EFAC', warnBg: '#3A2C14', warnInk: '#FCD34D', errBg: '#3F1B18', errInk: '#FCA5A5' }
    };
    const light = (a) => Object.assign({ label: 'Light', bg: '#F5F3EE', fg: '#16171A', muted: '#5B5F66', surface: '#FFFFFF', line: '#E2DFD8', outline: '#16171A' }, status.light, a);
    const dark = (a) => Object.assign({ label: 'Dark', bg: '#131516', fg: '#E9E7E2', muted: '#A9ADB2', surface: '#1C1F21', line: '#2E3235', outline: '#E9E7E2' }, status.dark, a);
    return {
      options: [
        {
          name: 'Soft indigo',
          note: 'Same periwinkle in both themes · dark text on soft buttons',
          swatches: [
            { label: 'Primary fill', hex: '#A5B4FC', ratio: '8.0:1 with ink text' },
            { label: 'Hover / pressed', hex: '#818CF8', ratio: '' },
            { label: 'Tint (badges)', hex: '#E6E9FE', ratio: 'light mode' },
            { label: 'Text and links', hex: '#4F46E5', ratio: '5.7:1 on page' }
          ],
          themes: [
            light({ primary: '#A5B4FC', onPrimary: '#1E1B4B', link: '#4F46E5', tab: '#6366F1', badgeBg: '#E6E9FE', badgeInk: '#3730A3' }),
            dark({ primary: '#A5B4FC', onPrimary: '#131516', link: '#A5B4FC', tab: '#A5B4FC', badgeBg: '#2A2D5C', badgeInk: '#C7D2FE' })
          ]
        },
        {
          name: 'For reference',
          note: 'The earlier light-mode versions you compared',
          swatches: [
            { label: 'Deep indigo', hex: '#4338CA', ratio: 'white text 7.9:1' },
            { label: 'Deep teal-cyan', hex: '#0E7490', ratio: 'white text 5.4:1' },
            { label: 'Soft indigo', hex: '#A5B4FC', ratio: 'ink text 8.0:1' },
            { label: 'Page ground', hex: '#F5F3EE', ratio: '' }
          ],
          themes: [
            light({ label: 'Light · deep indigo (before)', primary: '#4338CA', onPrimary: '#FFFFFF', link: '#4338CA', tab: '#4338CA', badgeBg: '#E0E7FF', badgeInk: '#3730A3' }),
            light({ label: 'Light · deep teal-cyan (before)', primary: '#0E7490', onPrimary: '#FFFFFF', link: '#0E7490', tab: '#0E7490', badgeBg: '#CFFAFE', badgeInk: '#155E75' })
          ]
        }
      ]
    };
  }
}
```

---

## Event page options (A chosen)

File: `EventOptions.dc.html` · Page title: Event page options

Outline:

```
Option A · Recommended
People row only
Keeps the person filter you liked as the single visible control. Sort moves into the section header. No search bar, no chips.
Rooftop birthday
Fri 7 PM – 1 AM · Hosted by Sam · deletes Sat 2 PM
PINNED · SAM · 8:12 PM
Cake at 9 on the terrace. Group photo right after!
All posts
Photos 48
Posts 4
Attendees 23
▸ for each pp in people:
  {{pp.initial}}
  {{pp.name}}
9:30 – 10:00 PM
· 14
Newest
▸ for each t in tilesA:
Option B
One search bar, filters in a sheet
The cleanest. Person, saved and time filters open from one button; the active filter shows as a single removable chip.
Rooftop birthday
Fri 7 PM – 1 AM · Hosted by Sam · deletes Sat 2 PM
Sam:
Cake at 9 on the terrace. Group photo right after!
Photos 48
Posts 4
Attendees 23
Search people or times
1
Jordan
14 photos
▸ for each t in tilesB:
Filter photos (sheet, when tapped)
PEOPLE
▸ for each pp in people:
  {{pp.initial}}
Also: Most saved · Saved by me · Time range
Option C
Photo hero + two dropdowns
The most visual. The pinned post sits on the event's cover photo; filtering is two small dropdowns (person, sort) on the section header.
Rooftop birthday
Fri 7 PM – 1 AM · Hosted by Sam · deletes Sat 2 PM
Sam · 8:12 PM
Cake at 9 on the terrace. Group photo right after!
Photos 48
Posts 4
Attendees 23
9:30 – 10:00 PM
· 14
Everyone
Newest
▸ for each t in tilesC:
```

Script (data and state):

```js
class Component extends DCLogic {
  renderVals() {
    const S = {
      dusk: 'linear-gradient(180deg,#2E3168 0%,#6E4C8A 38%,#E79B7A 62%,#3B2A33 63%,#1C1620 100%)',
      lights: '<scene>',
      cake: '<scene>',
      terrace: '<scene>',
      portrait: '<scene>',
      city: '<scene>',
      group: '<scene>',
      drinks: '<scene>',
      confetti: '<scene>',
      sky: 'linear-gradient(180deg,#F7C59F 0%,#EF8A7E 40%,#8C5A8F 75%,#3A2F55 100%)'
    };
    const t = (arr) => arr.map((s) => ({ scene: s }));
    return {
      feature: S.dusk,
      cover: S.group,
      jordan: S.portrait,
      people: [
        { initial: 'All', name: 'Everyone', scene: '#16171A', ring: '4px', weight: '700' },
        { initial: 'J', name: 'Jordan', scene: S.portrait, ring: '0px', weight: '500' },
        { initial: 'P', name: 'Priya', scene: S.terrace, ring: '0px', weight: '500' },
        { initial: 'A', name: 'Alex', scene: S.city, ring: '0px', weight: '500' },
        { initial: 'S', name: 'Sam', scene: S.lights, ring: '0px', weight: '500' },
        { initial: 'M', name: 'You', scene: S.confetti, ring: '0px', weight: '500' }
      ],
      tilesA: t([S.cake, S.group, S.lights, S.drinks, S.terrace, S.city, S.sky]),
      tilesB: t([S.portrait, S.city, S.drinks, S.sky, S.cake]),
      tilesC: t([S.dusk, S.cake, S.lights, S.drinks, S.terrace, S.city, S.sky, S.portrait, S.confetti, S.group, S.dusk, S.cake, S.lights, S.drinks, S.terrace])
    };
  }
}
```

---

## 1 · Welcome

File: `Main.dc.html` · Page title: Locale · Welcome

Outline:

```
Locale
▸ for each c in collage:
Original · 3.4 MB
# Share full-quality photos with the people around you.
Photos stay on devices at the event, not in the cloud. Everything is deleted when the event ends.
[button: Sign up]
[button: Sign in]
[button: Continue as guest]
Guests can view and download. Uploading and hosting need a verified account.
```

Script (data and state):

```js
class Component extends DCLogic {
  renderVals() {
    return {
      collage: [
        { x: '8px', y: '14px', w: '146px', h: '184px', r: '-7deg', scene: 'linear-gradient(180deg,#2E3168 0%,#6E4C8A 38%,#E79B7A 62%,#3B2A33 63%,#1C1620 100%)' },
        { x: '178px', y: '6px', w: '156px', h: '116px', r: '5deg', scene: '<scene>' },
        { x: '196px', y: '118px', w: '118px', h: '118px', r: '-4deg', scene: '<scene>' },
        { x: '62px', y: '160px', w: '118px', h: '88px', r: '6deg', scene: '<scene>' }
      ]
    };
  }
}
```

---

## 2 · Home and nearby

File: `Home.dc.html` · Page title: Locale · Home

Tweaks:
- `you` (enum): attendee | hosting — default `attendee`
- `hosting` (enum): desktop (MVP) | phone (M5) — default `desktop (MVP)`

Outline:

```
Locale
[button: {{initials}}] (aria: Account)
## Your events
▸ for each ev in mine:
  [link → Event.dc.html: {{ev.name}} {{ev.role}} {{ev.meta}}]
[button: Scan QR code to join]
▸ if notHosting:
  [button: Quick session]
  [button: Host an event]
  {{hostCaption}}
▸ if isHosting:
  You're hosting Rooftop birthday
  from Sam's MacBook. You can still join other events. End this one there to host something new.
## Nearby
On this Wi-Fi
▸ for each ev in nearby:
  ▸ if ev.locked:
  [button: {{ev.initial}} {{ev.name}} {{ev.meta}} [icon: Password required]]
```

Script (data and state):

```js
class Component extends DCLogic {
  renderVals() {
    const isHosting = (this.props.you ?? 'attendee') === 'hosting';
    const phone = (this.props.hosting ?? 'desktop (MVP)') === 'phone (M5)';
    const dark = { bg: '#16171A', ink: '#FFFFFF', sub: '#D4D4D8' };
    const light = { bg: '#FFFFFF', ink: '#16171A', sub: '#5B5F66' };
    const uploader = { role: 'Uploader', roleBg: '#F1EFEA', roleInk: '#3F4349' };
    const uploaderDark = { role: 'Uploader', roleBg: '#3F3F46', roleInk: '#F4F4F5' };
    const host = { role: 'Host', roleBg: '#A5B4FC', roleInk: '#1E1B4B' };
    return {
      isHosting,
      notHosting: !isHosting,
      initials: isHosting ? 'SR' : 'MR',
      hostCaption: phone
        ? 'Host from this phone. Keep Locale open while hosting. One hosted event at a time.'
        : 'In this version, sessions and events are hosted from Locale Host on a Mac or Windows computer. We’ll send you the link.',
      mine: [
        Object.assign({ thumbA: 'linear-gradient(180deg,#2E3168 0%,#6E4C8A 38%,#E79B7A 62%,#3B2A33 63%,#1C1620 100%)', thumbB: '<scene>', name: 'Rooftop birthday', meta: isHosting ? 'Live · 23 here · hosted from your MacBook' : 'Live · 23 here · 48 photos' }, dark, isHosting ? host : uploaderDark),
        Object.assign({ thumbA: '<scene>', thumbB: 'linear-gradient(135deg,#1F3A4A 0%,#3E7C8C 50%,#C9E4DF 100%)', name: 'Climbing gym meetup', meta: 'Live · 9 here · 31 photos' }, light, uploader)
      ],
      nearby: [
        { initial: 'T', name: 'Saturday trivia', meta: 'Open · 12 here · Hosted by Bar laptop', locked: false },
        { initial: 'D', name: 'Design club meetup', meta: 'Password · 8 here · Hosted by Priya', locked: true }
      ]
    };
  }
}
```

---

## 3 · Join event

File: `Join.dc.html` · Page title: Locale · Join event

Tweaks:
- `access` (enum): password | invite link — default `password`
- `eventId` (text):  — default `evt_7f3a91`

Outline:

```
[button: Back]
{{liveLabel}}
# Design club meetup
Today 7:00 – 10:00 PM
P
Hosted by Priya
{{accessLine}}
▸ if isPassword:
  [icon: Password protected]
▸ if isPassword:
  label: Event password
  [input:password value="••••••••"]
  [button: Show]
First time joining this host. Check this code matches the host's screen:
4F2A · 9C1E · 77B0
Photos, posts and who's here stay hidden until you've joined.
[button: Join as Uploader]
You can view, download and add photos. Your phone needs Local Network access to reach the host.
```

Script (data and state):

```js
class Component extends DCLogic {
  renderVals() {
    const isPassword = (this.props.access ?? 'password') === 'password';
    const id = this.props.eventId ?? 'evt_7f3a91';
    let h = 2166136261;
    for (let i = 0; i < id.length; i++) { h ^= id.charCodeAt(i); h = Math.imul(h, 16777619) >>> 0; }
    const r = (n) => { h = Math.imul(h ^ (h >>> 13), 1274126177) >>> 0; return h % n; };
    const ax = 15 + r(30), ay = 20 + r(40), bx = 60 + r(30), by = 45 + r(40);
    const hue = ['#312E81', '#3730A3', '#1E1B4B'][r(3)];
    const nodes = [0, 1, 2, 3, 4].map(() => 'radial-gradient(circle at ' + (8 + r(84)) + '% ' + (10 + r(70)) + '%, rgba(199,210,254,0.9) 0 3px, transparent 4px)').join(',');
    const cover = nodes + ',' +
      'repeating-radial-gradient(circle at ' + ax + '% ' + ay + '%, rgba(165,180,252,0.30) 0 2px, transparent 2px 26px),' +
      'repeating-radial-gradient(circle at ' + bx + '% ' + by + '%, rgba(165,180,252,0.18) 0 1.5px, transparent 1.5px 20px),' +
      'radial-gradient(circle at ' + ax + '% ' + ay + '%, rgba(129,140,248,0.55) 0%, transparent 45%),' +
      'linear-gradient(160deg, ' + hue + ' 0%, #4338CA 55%, #6366F1 100%)';
    return {
      isPassword,
      cover,
      liveLabel: isPassword ? 'Live now' : 'Live now · 62 photos · 8 here',
      accessLine: isPassword ? 'Password required · counts appear after you join' : 'Invite link · photos delete 24 h after it ends'
    };
  }
}
```

---

## 4 · Event page (tabs, people and sort menu work in Play)

File: `Event.dc.html` · Page title: Locale · Event page

Tweaks:
- `viewer` (enum): attendee | host phone — default `attendee`
- `state` (enum): live | extended | hostOffline — default `live`
- `tab` (enum): photos | posts | attendees — default `photos`
- `sort` (enum): newest | oldest | most downloaded | hottest — default `newest`
- `menuOpen` (boolean):  — default `False`
- `progress` (range):  — default `62`

Outline:

```
[button: Back to home]
[link → Uploads.dc.html: 3]
[button: Event details and menu]
# Rooftop birthday
{{metaLine}}
▸ if isOffline:
  Host offline.
  Photos saved on this phone still open. New uploads will send when the host is back.
▸ if isHostPhone:
  Host
  Post and delete here · end or extend from Sam's MacBook
▸ if notPosts:
  {{pinnedLabel}}
  {{pinnedText}}
  [button: All posts] → openPosts
<div “Event sections”>
▸ for each t in tabs:
  [button: {{t.label}}] → t.pick
▸ if showPhotos:
  <div “Show photos by”>
  ▸ for each pp in people:
    [button: {{pp.initial}} {{pp.name}}] (aria: {{pp.aria}}) → pp.pick
  {{sectionTitle}}
  · {{sectionCount}}
  [button: {{sortLabel}}] → toggleMenu
  ▸ if menuOpen:
    <div “Sort and show”>
    SORT BY
    ▸ for each s in sorts:
      ▸ if s.on:
      [button: {{s.label}} {{s.hint}}] → s.pick
    SHOW
    ▸ for each w in shows:
      ▸ if w.on:
      [button: {{w.label}}] → w.pick
  ▸ for each p in tiles:
    ▸ if p.uploading:
    ▸ if p.showBadge:
    [link → Viewer.dc.html: {{progress}}% Only you see this {{p.badge}} {{p.initial}} {{p.caption}}]
  [link → Uploads.dc.html: Add photos]
▸ if showPosts:
  ▸ if isHostPhone:
    [button: Write a post]
  ▸ for each post in posts:
    {{post.who}}
    {{post.time}}
    {{post.text}}
  {{postsNote}}
▸ if showPeople:
  ▸ for each person in roster:
    {{person.initial}}
    {{person.name}}
    ▸ if person.verified:
      [icon: Verified account]
    {{person.meta}}
  Verified accounts can upload. Guests can view and download.
```

Script (data and state):

```js
class Component extends DCLogic {
  renderVals() {
    const S = {
      dusk: 'linear-gradient(180deg,#2E3168 0%,#6E4C8A 38%,#E79B7A 62%,#3B2A33 63%,#1C1620 100%)',
      lights: '<scene>',
      cake: '<scene>',
      terrace: '<scene>',
      portrait: '<scene>',
      city: '<scene>',
      group: '<scene>',
      drinks: '<scene>',
      confetti: '<scene>',
      sky: 'linear-gradient(180deg,#F7C59F 0%,#EF8A7E 40%,#8C5A8F 75%,#3A2F55 100%)'
    };
    const st = this.state || {};
    const state = this.props.state ?? 'live';
    const isHostPhone = (this.props.viewer ?? 'attendee') === 'host phone';
    const tab = st.tab || this.props.tab || 'photos';
    const sort = st.sort || this.props.sort || 'newest';
    const show = st.show || 'everything';
    const person = st.person || 'all';
    const menuOpen = st.menuOpen !== undefined ? st.menuOpen : (this.props.menuOpen ?? false);
    const set = (o) => () => this.setState(o);

    const hostPost = (time, text) => ({ who: 'Sam · host', whoInk: '#3730A3', time, text, bg: '#FFFFFF', border: '#E2DFD8' });
    const posts = [
      hostPost('8:12 PM', 'Cake at 9 on the terrace. Group photo right after!'),
      hostPost('7:48 PM', 'Drinks are in the cooler by the stairs. Please take empties down.'),
      { who: 'Locale', whoInk: '#3730A3', time: '7:30 PM', text: 'The host extended the event. Photos now delete Sun, Nov 16 at 2:00 PM.', bg: '#E6E9FE', border: '#E6E9FE' },
      hostPost('7:05 PM', 'Buzz 4B at the front door, then take the elevator to R.'),
      hostPost('7:00 PM', 'Welcome! Add your photos here so everyone gets the full-quality versions.')
    ];
    if (state !== 'extended') posts.splice(2, 1);
    const pinned = state === 'extended'
      ? { label: 'LOCALE · 7:30 PM', text: 'The host extended the event. Photos now delete Sun, Nov 16 at 2:00 PM.' }
      : { label: 'PINNED · SAM · 8:12 PM', text: 'Cake at 9 on the terrace. Group photo right after!' };

    const sortDefs = [
      ['newest', 'Newest', 'Latest first, grouped by time'],
      ['oldest', 'Oldest', 'From the start of the event'],
      ['most downloaded', 'Most downloaded', 'Saved most by everyone'],
      ['hottest', 'Hottest', 'Most saved in the last 30 minutes']
    ];
    const showDefs = [['everything', 'Everything'], ['saved', 'Saved by me'], ['unsaved', 'Not saved yet'], ['mine', 'My photos']];
    const ranked = sort === 'most downloaded' || sort === 'hottest';
    const sortLabel = (sortDefs.find((d) => d[0] === sort) || sortDefs[0])[1] + (show !== 'everything' ? ' · 1 filter' : '');

    const uploaders = [['all', 'All', 'Everyone', '#16171A'], ['jordan', 'J', 'Jordan', S.portrait], ['priya', 'P', 'Priya', S.terrace], ['alex', 'A', 'Alex', S.city], ['sam', 'S', 'Sam', S.lights], ['you', 'M', 'You', S.confetti]];

    const timeTiles = [
      { scene: S.dusk, initial: 'J', caption: 'Jordan · 9:41', span: 'span 2', uploading: false, showBadge: true, badge: 'HEIC · HDR' },
      { scene: S.confetti, initial: 'M', caption: 'You', span: 'span 1', uploading: true, showBadge: false, badge: '' },
      { scene: S.cake, initial: 'P', caption: '9:38', span: 'span 1', uploading: false, showBadge: false, badge: '' },
      { scene: S.group, initial: 'A', caption: '9:36', span: 'span 1', uploading: false, showBadge: false, badge: '' },
      { scene: S.lights, initial: 'S', caption: '9:35', span: 'span 1', uploading: false, showBadge: false, badge: '' },
      { scene: S.drinks, initial: 'J', caption: '9:33', span: 'span 1', uploading: false, showBadge: false, badge: '' },
      { scene: S.terrace, initial: 'P', caption: '9:31', span: 'span 1', uploading: false, showBadge: false, badge: '' },
      { scene: S.city, initial: 'A', caption: '9:30', span: 'span 1', uploading: false, showBadge: false, badge: '' }
    ];
    const rankScenes = sort === 'hottest'
      ? [[S.cake, 'P', '14'], [S.group, 'A', '11'], [S.dusk, 'J', '9'], [S.lights, 'S', '7'], [S.sky, 'M', '6'], [S.drinks, 'J', '5'], [S.terrace, 'P', '4'], [S.city, 'A', '3'], [S.portrait, 'J', '2']]
      : [[S.dusk, 'J', '18'], [S.group, 'A', '16'], [S.cake, 'P', '15'], [S.terrace, 'P', '12'], [S.lights, 'S', '11'], [S.portrait, 'J', '9'], [S.city, 'A', '8'], [S.drinks, 'J', '7'], [S.sky, 'M', '6']];
    const rankTiles = rankScenes.map(([sc, i, n], k) => ({
      scene: sc, initial: i, caption: '#' + (k + 1), span: k === 0 ? 'span 2' : 'span 1', uploading: false, showBadge: true,
      badge: (sort === 'hottest' ? '↑ ' : '↓ ') + n
    }));
    const tiles = (ranked ? rankTiles : (sort === 'oldest' ? timeTiles.slice().reverse().filter((t) => !t.uploading) : timeTiles))
      .map((t, k) => Object.assign({ label: 'Photo by ' + t.caption + ', ' + (k + 1) + ' of 48' }, t));

    return {
      isHostPhone,
      isOffline: state === 'hostOffline',
      metaLine: state === 'extended' ? 'Fri 7 PM – 1 AM · Hosted by Sam · deletes Sun 2 PM' : 'Fri 7 PM – 1 AM · Hosted by Sam · deletes Sat 2 PM',
      pinnedLabel: pinned.label, pinnedText: pinned.text,
      postsNote: isHostPhone ? 'Everyone sees your posts here. They are deleted with the event.' : 'Only the host can post here. Posts are deleted with the event.',
      progress: this.props.progress ?? 62,
      showPhotos: tab === 'photos', showPosts: tab === 'posts', showPeople: tab === 'attendees', notPosts: tab !== 'posts',
      openPosts: set({ tab: 'posts' }),
      tabs: [['photos', 'Photos 48'], ['posts', 'Posts ' + posts.length], ['attendees', 'Attendees 23']].map(([id, label]) => ({
        label, sel: tab === id ? 'true' : 'false', line: tab === id ? '#6366F1' : 'transparent',
        weight: tab === id ? '700' : '500', ink: tab === id ? '#16171A' : '#5B5F66', pick: set({ tab: id })
      })),
      people: uploaders.map(([id, initial, name, scene]) => ({
        initial, name, scene, sel: person === id ? 'true' : 'false', ring: person === id ? '4px' : '0px', weight: person === id ? '700' : '500',
        aria: id === 'all' ? 'Everyone’s photos' : 'Photos by ' + name, pick: set({ person: id })
      })),
      sectionTitle: ranked ? (sort === 'hottest' ? 'Hottest right now' : 'Most downloaded') : (sort === 'oldest' ? '7:00 – 7:30 PM' : '9:30 – 10:00 PM'),
      sectionCount: ranked ? '48 photos' : '14',
      sortLabel,
      sortBg: menuOpen ? '#E6E9FE' : '#FFFFFF', sortBorder: menuOpen ? '#A5B4FC' : '#E2DFD8',
      menuOpen, menuExpanded: menuOpen ? 'true' : 'false',
      toggleMenu: set({ menuOpen: !menuOpen }),
      sorts: sortDefs.map(([id, label, hint]) => ({ label, hint, on: sort === id, sel: sort === id ? 'true' : 'false', bg: sort === id ? '#F5F6FF' : 'transparent', pick: set({ sort: id, menuOpen: false }) })),
      shows: showDefs.map(([id, label]) => ({ label, on: show === id, sel: show === id ? 'true' : 'false', bg: show === id ? '#F5F6FF' : 'transparent', pick: set({ show: id, menuOpen: false }) })),
      tiles,
      posts,
      roster: [
        { initial: 'S', name: 'Sam', meta: 'Host · Mac', scene: S.lights, verified: true },
        { initial: 'J', name: 'Jordan', meta: 'Uploader · 14 photos', scene: S.portrait, verified: true },
        { initial: 'P', name: 'Priya', meta: 'Uploader · 11 photos', scene: S.terrace, verified: true },
        { initial: 'A', name: 'Alex', meta: 'Uploader · 9 photos', scene: S.city, verified: true },
        { initial: 'M', name: 'Maya (you)', meta: 'Uploader · 3 photos', scene: S.confetti, verified: true },
        { initial: 'L', name: 'Leo', meta: 'Guest · browser', scene: '#8A8780', verified: false }
      ]
    };
  }
}
```

---

## 5 · Photo viewer and save

File: `Viewer.dc.html` · Page title: Locale · Photo viewer

Tweaks:
- `format` (enum): HEIC | JPEG — default `HEIC`

Outline:

```
[button: Close]
J
Jordan
Fri 9:41 PM · 7 of 48
{{formatLabel}}
12 MP
▸ if isHeic:
  HDR
Saved by 12
## Save photo
▸ if isHeic:
  legend: Version to save
  label: [input:radio checked] Original · HEIC Best for this phone The exact file Jordan shared, with HDR · 3.4 MB
  label: [input:radio] Compatible · JPEG Opens in any app. Slightly lower quality, no HDR · 5.1 MB
▸ if isJpeg:
  Original · JPEG
  The exact file Jordan shared. Opens everywhere · 4.2 MB
[button: Save to Photos]
Saved photos stay after the event ends. Everything else is deleted.
[button: Delete this photo]
```

Script (data and state):

```js
class Component extends DCLogic {
  renderVals() {
    const f = this.props.format ?? 'HEIC';
    return {
      isHeic: f === 'HEIC', isJpeg: f === 'JPEG', formatLabel: f,
      scene: 'linear-gradient(180deg,#2E3168 0%,#6E4C8A 38%,#E79B7A 62%,#3B2A33 63%,#1C1620 100%)'
    };
  }
}
```

---

## 6 · Uploads (outbox)

File: `Uploads.dc.html` · Page title: Locale · Uploads

Outline:

```
[button: Back to event]
Uploads
Rooftop birthday
3 sending
1 waiting
2 need attention
▸ for each r in rows:
  {{r.name}}
  {{r.badge}}
  ▸ if r.bar:
  {{r.detail}}
  ▸ if r.hasAction:
    [button: {{r.action}}]
Uploads keep going if you leave this screen, and resume where they stopped if the connection drops. Location data is removed before sending.
```

Script (data and state):

```js
class Component extends DCLogic {
  renderVals() {
    const blue = '#4F46E5', amber = '#92400E', red = '#B42318', green = '#067647', grey = '#5B5F66';
    return {
      rows: [
        { thumb: 'linear-gradient(180deg,#F7C59F 0%,#EF8A7E 40%,#8C5A8F 75%,#3A2F55 100%)', name: 'IMG_4821.HEIC', badge: '62%', tone: blue, bar: true, pct: '62%', detail: '2.1 of 3.4 MB', hasAction: true, action: 'Cancel' },
        { thumb: '<scene>', name: 'IMG_4822.HEIC', badge: '18%', tone: blue, bar: true, pct: '18%', detail: 'Resumed after connection dropped', hasAction: false, action: '' },
        { thumb: '<scene>', name: 'IMG_4819.HEIC', badge: 'Checking', tone: blue, bar: false, pct: '0%', detail: 'Received by host, being checked', hasAction: false, action: '' },
        { thumb: '<scene>', name: 'IMG_4823.JPG', badge: 'Waiting', tone: amber, bar: false, pct: '0%', detail: 'Will send when the host is reachable', hasAction: false, action: '' },
        { thumb: 'repeating-linear-gradient(135deg,#3F4349 0 6px,#4A4E55 6px 12px)', name: 'Screen Recording.mov', badge: 'Not sent', tone: red, bar: false, pct: '0%', detail: 'Only photos (JPEG, HEIC, PNG) can be shared', hasAction: true, action: 'Remove' },
        { thumb: '<scene>', name: 'IMG_4790.HEIC', badge: 'Stopped', tone: red, bar: false, pct: '0%', detail: 'Stopped after 10 tries', hasAction: true, action: 'Retry' },
        { thumb: 'linear-gradient(180deg,#2E3168 0%,#6E4C8A 38%,#E79B7A 62%,#3B2A33 63%,#1C1620 100%)', name: 'IMG_4815.HEIC', badge: 'Shared', tone: green, bar: false, pct: '100%', detail: 'Visible to everyone · 9:41 PM', hasAction: false, action: '' }
      ]
    };
  }
}
```

---

## 7 · Host: create event or quick session

File: `HostCreate.dc.html` · Page title: Locale Host · Create event

Tweaks:
- `alreadyHosting` (boolean):  — default `False`

Outline:

```
Locale
<nav “Main”>
[link → DeskEvents.dc.html: Events]
[link → HostCreate.dc.html: Host]
[link → HostPast.dc.html: Past events]
[button: SR] (aria: Account: Sam R.)
# Host an event
Photos are stored on this computer and deleted when the event expires.
▸ if isHosting:
  Live
  ## You're hosting Rooftop birthday
  You can host one event or quick session at a time. End this one, or let it expire, to start something new. You can still join other people's events from Events or from your phone.
  [link → HostLive.dc.html: Open live event]
▸ if notHosting:
  ## Quick session
  Share right now at a cafe, a work session or a hangout. No form: you get a QR code in one click.
  Invite link only
  Ends when you end it · 12 h max
  Deleted at the end
  Photos only
  [link → QuickCheck.dc.html: Start quick session]
  ## Or set up a full event
  ## Event details
  label: Name
  [input:text value="Rooftop birthday"]
  label: Description
  [textarea: Sam turns 30. Drop your photos here so everyone gets them in full quality.]
  label: Starts
  [input:text value="Fri, Nov 14 · 7:00 PM"]
  label: Ends
  [input:text value="Sat, Nov 15 · 1:00 AM"]
  label: Links (optional)
  [input:text value="Playlist · Building entry"]
  ## Who can join
  legend: Access type
  label: [input:radio] Open Shows in Nearby
  label: [input:radio checked] Invite link only QR code or link
  label: [input:radio] Password App only, no browsers
  label: [input:checkbox checked] Let browsers join as viewers. People without the app can view and download, but not upload.
  ## Storage and deletion
  label: Delete photos
  24 hours after the event ends
  When the event ends
  3 days after the event ends
  You can extend later, up to 7 days total.
  label: Space to use
  20 GB
  10 GB
  50 GB
  ~/Locale/Events
  ## Ready to host?
  ▸ for each c in checks:
    {{c.mark}}
    {{c.title}}
    {{c.detail}}
    ▸ if c.fix:
      [button: {{c.fixLabel}}]
  No shared Wi-Fi?
  Turn on this computer's hotspot, or plug in a travel router. Venue Wi-Fi sometimes blocks phones from seeing each other.
  [button: Start hosting]
  Fix 1 item to start.
```

Script (data and state):

```js
class Component extends DCLogic {
  renderVals() {
    const ok = '#067647', warn = '#B54708';
    const hosting = this.props.alreadyHosting ?? false;
    return {
      isHosting: hosting,
      notHosting: !hosting,
      checks: [
        { dot: ok, mark: '✓', title: 'Network', detail: 'Connected to "Rooftop-WiFi" at 192.168.1.24', fix: false, fixLabel: '' },
        { dot: warn, mark: '!', title: 'Firewall', detail: 'Incoming connections on port 7420 are blocked, so phones cannot join.', fix: true, fixLabel: 'Allow connections' },
        { dot: ok, mark: '✓', title: 'Space', detail: '212 GB free, 20 GB reserved for this event', fix: false, fixLabel: '' },
        { dot: ok, mark: '✓', title: 'Power', detail: 'Plugged in. Sleep is paused while hosting.', fix: false, fixLabel: '' }
      ]
    };
  }
}
```

---

## 7b · Host: quick session Wi-Fi check

File: `QuickCheck.dc.html` · Page title: Locale Host · Quick session check

Outline:

```
# Starting your quick session
▸ for each c in checks:
  {{c.mark}}
  {{c.title}}
  {{c.detail}}
Phones probably can't reach you on this Wi-Fi
Cafe and hotel networks often block devices from seeing each other. Turn on this computer's hotspot and ask people to join it before scanning.
[button: Cancel]
[link → HostLive.dc.html: Try anyway]
[link → HostLive.dc.html: Turn on hotspot and start]
Invite link only · ends when you end it, 12 hours at most · photos deleted at the end · photos only. You can add details later without changing the QR code.
```

Script (data and state):

```js
class Component extends DCLogic {
  renderVals() {
    const ok = '#067647', warn = '#B54708';
    return {
      checks: [
        { dot: ok, mark: '✓', title: 'Network', detail: 'Connected to "Bean There Guest" at 10.0.4.17' },
        { dot: warn, mark: '!', title: 'Can phones reach this computer?', detail: 'A test from this network could not reach your computer. The Wi-Fi is likely blocking devices from each other.' },
        { dot: ok, mark: '✓', title: 'Firewall', detail: 'Incoming connections allowed on port 7420' },
        { dot: ok, mark: '✓', title: 'Power and space', detail: 'Battery 64%, sleep paused while hosting · 5 GB reserved' }
      ]
    };
  }
}
```

---

## 8 · Host: live event / quick session

File: `HostLive.dc.html` · Page title: Locale Host · Live event

Tweaks:
- `mode` (enum): event | quick session — default `event`
- `expiringSoon` (boolean):  — default `False`

Outline:

```
{{eventName}}
▸ if isQuick:
  [button: Rename]
{{pillLabel}}
[button: Add photos]
[button: Edit details]
## Join with this code
<div “Event QR code”>
▸ for each q in qr:
Scan with the Locale app, or any phone camera to view in a browser.
[button: Show full screen]
[button: Print]
Host code (for Nearby joins)
4F2A · 9C1E · 77B0
192.168.1.24:7420 · Rooftop-WiFi
Here now
23
11 iPhone · 9 Android · 3 browser
Photos
48
2 uploading now
Failed in last hour
1
Unsupported file type
Storage
3.8
of 20 GB
▸ if isEvent:
  Photos delete in
  {{remaining}}
  {{expiryDate}} · up to 7 days total (until Thu, Nov 20)
  [button: +1 hour]
  [button: +6 hours]
  [button: +1 day]
  [button: Custom…]
  Everyone is told when you extend or shorten.
▸ if isQuick:
  Quick session
  Ends when you end it
  Auto-ends at 3:40 AM (12 h max). Photos are deleted as soon as it ends.
  [button: Add details]
  Turns it into a full event with a description, schedule and deletion time. Same QR code.
## Photos
[link → #gallery: Open gallery (48)]
Drop photos here, or
[button: choose files]
JPEG, HEIC or PNG up to 50 MB. Copies are added to the event; your originals stay where they are.
Latest photos (yours first)
▸ for each m in mine:
  ▸ if m.busy:
    {{m.label}}
Checked the same way as everyone's photos before they appear.
## Attendees
[link → #all: View all 23]
th: Name
th: Device
th: Role
th: Photos
th: Actions
▸ for each p in people:
  {{p.name}}
  ▸ if p.verified:
    [icon: Verified account]
  {{p.device}}
  {{p.role}}
  {{p.photos}}
  [button: {{p.menuLabel}}]
Verified account · use ⋯ to remove someone
label: New post
Shows in everyone's Posts tab.
[input:text placeholder="Post to everyone…"]
[button: Post]
## Event log
[button: Export]
▸ for each l in log:
  {{l.t}}
  {{l.text}}
## {{dangerTitle}}
{{dangerText}}
[button: {{endLabel}}]
```

Script (data and state):

```js
class Component extends DCLogic {
  renderVals() {
    const soon = this.props.expiringSoon ?? false;
    const quick = (this.props.mode ?? 'event') === 'quick session';
    const N = 25;
    const finder = (r, c, r0, c0) => {
      const y = r - r0, x = c - c0;
      if (y < 0 || x < 0 || y > 6 || x > 6) return null;
      if (y === 0 || y === 6 || x === 0 || x === 6) return true;
      if (y >= 2 && y <= 4 && x >= 2 && x <= 4) return true;
      return false;
    };
    const qr = [];
    for (let r = 0; r < N; r++) {
      for (let c = 0; c < N; c++) {
        let on = finder(r, c, 0, 0);
        if (on === null) on = finder(r, c, 0, N - 7);
        if (on === null) on = finder(r, c, N - 7, 0);
        if (on === null) {
          const nearFinder = (r < 8 && c < 8) || (r < 8 && c > N - 9) || (r > N - 9 && c < 8);
          on = nearFinder ? false : (((r * 7 + c * 13 + r * c) % 5) < 2);
        }
        qr.push({ bg: on ? '#16171A' : '#FFFFFF' });
      }
    }
    return {
      qr,
      isQuick: quick,
      isEvent: !quick,
      eventName: quick ? 'Sam’s session · 3:40 PM' : 'Rooftop birthday',
      pillLabel: quick ? 'Live · Quick session' : 'Live',
      endLabel: quick ? 'End session' : 'End event',
      dangerTitle: quick ? 'End this session' : 'End this event',
      dangerText: quick ? 'Stops hosting now and deletes every photo from this computer and from everyone’s app. Photos people saved to their own devices stay.' : 'Stops hosting now and deletes every photo and post from this computer and from everyone’s app, instead of waiting for Sat, Nov 15 at 2:00 PM. Photos people saved to their own devices stay.',
      remaining: soon ? '52 minutes' : '17 h 42 m',
      expiryDate: 'Sat, Nov 15 at 2:00 PM',
      expiryBg: soon ? '#FFF4E5' : '#FFFFFF',
      expiryBorder: soon ? '#F5B97A' : '#E2DFD8',
      expiryInk: soon ? '#92400E' : '#16171A',
      people: [
        { name: 'Sam R. (you)', device: 'Mac', role: 'Host', photos: 6, verified: true, menuLabel: 'More actions for Sam R.' },
        { name: 'Jordan', device: 'iPhone', role: 'Uploader', photos: 14, verified: true, menuLabel: 'More actions for Jordan' },
        { name: 'Alex', device: 'Android', role: 'Uploader', photos: 9, verified: true, menuLabel: 'More actions for Alex' },
        { name: 'Priya', device: 'iPhone', role: 'Uploader', photos: 11, verified: true, menuLabel: 'More actions for Priya' },
        { name: 'Leo (guest)', device: 'Browser', role: 'Viewer', photos: 0, verified: false, menuLabel: 'More actions for Leo' }
      ],
      log: [
        { t: '9:44 PM', text: 'Sam (host) added 3 photos from this computer', ink: '#16171A' },
        { t: '9:41 PM', text: 'Jordan shared IMG_4815.HEIC', ink: '#16171A' },
        { t: '9:40 PM', text: 'Leo joined from a browser (viewer)', ink: '#16171A' },
        { t: '9:38 PM', text: 'Denied: unsupported file type (Alex, .mov)', ink: '#B42318' },
        { t: '9:35 PM', text: 'Leo downloaded a compatible JPEG', ink: '#16171A' },
        { t: '9:30 PM', text: 'Priya re-joined after her session expired', ink: '#16171A' }
      ],
      mine: [
        { scene: '<scene>', busy: true, label: 'Checking' },
        { scene: '<scene>', busy: true, label: '40%' },
        { scene: 'linear-gradient(180deg,#2E3168 0%,#6E4C8A 38%,#E79B7A 62%,#3B2A33 63%,#1C1620 100%)', busy: false, label: '' },
        { scene: '<scene>', busy: false, label: '' },
        { scene: '<scene>', busy: false, label: '' },
        { scene: '<scene>', busy: false, label: '' }
      ]
    };
  }
}
```

---

## 9 · Host: past events

File: `HostPast.dc.html` · Page title: Locale Host · Past events

Outline:

```
Locale
<nav “Main”>
[link → DeskEvents.dc.html: Events]
[link → HostCreate.dc.html: Host]
[link → HostPast.dc.html: Past events]
[button: SR] (aria: Account: Sam R.)
# Past events
Only settings are kept. Photos, attendees and the event index were deleted when each event ended.
[link → HostCreate.dc.html: New event]
▸ for each e in events:
  ## {{e.name}}
  {{e.ended}}
  {{e.access}}
  {{e.retention}}
  {{e.browsers}}
  [button: More actions for {{e.name}}, including delete settings]
  [button: Host again]
Host again
starts a new, empty event with these settings. It gets a new QR code, so old codes and links stop working and everyone joins again.
```

Script (data and state):

```js
class Component extends DCLogic {
  renderVals() {
    return {
      events: [
        { name: 'Saturday trivia', ended: 'Ended Sat, Nov 8 · photos deleted Sun, Nov 9', access: 'Open', retention: 'Delete 24 h after end', browsers: 'Browsers allowed' },
        { name: 'Design club meetup', ended: 'Ended Sat, Nov 1 · photos deleted at end', access: 'Password', retention: 'Delete at end', browsers: 'App only' },
        { name: 'Studio open house', ended: 'Ended Sun, Oct 26 · photos deleted Wed, Oct 29', access: 'Invite link', retention: 'Delete 3 days after end', browsers: 'Browsers allowed' }
      ]
    };
  }
}
```

---

## 10 · Browser guest: join

File: `GuestJoin.dc.html` · Page title: Locale guest · Join

Outline:

```
Not secure
192.168.1.24:7420/e/rooftop
Locale
You're invited to
# Rooftop birthday
Hosted by Sam · 48 photos · 23 here
Photos and who's here appear after you join.
label: Your name
[input:text value="Leo"]
Shown to the host in the attendee list.
[button: View photos]
View only in the browser
You can browse and download photos.
To add photos and save originals straight to Photos, use the app.
[link → https://example.com/get-locale: Get the Locale app]
This page comes straight from the host's computer on this Wi-Fi, not from the internet. Photos are deleted Sat, Nov 15 at 2:00 PM.
```

Script (data and state):

```js
class Component extends DCLogic {
  renderVals() {
    return {};
  }
}
```

---

## 11 · Browser guest: photo

File: `GuestPhoto.dc.html` · Page title: Locale guest · Photo

Outline:

```
Not secure
192.168.1.24:7420/e/rooftop
[link → GuestJoin.dc.html: All photos]
View only
Showing JPEG
Photo by Jordan · Fri 9:41 PM
The original is HEIC, which this browser can't display, so you're seeing a compatible copy.
[link → #download-compatible: Download compatible · JPEG Opens anywhere · 5.1 MB]
[link → #download-original: Download original · HEIC Exact file, with HDR · 3.4 MB · opens on iPhone, Mac, most Android]
On iPhone, downloads go to Files. Press and hold the photo to save it to Photos.
Want to add your own photos?
[link → https://example.com/get-locale: Get the app]
```

Script (data and state):

```js
class Component extends DCLogic {
  renderVals() {
    return {};
  }
}
```

---

## 12 · Desktop: Events (join and nearby)

File: `DeskEvents.dc.html` · Page title: Locale desktop · Events

Tweaks:
- `hosting` (boolean):  — default `True`

Outline:

```
Locale
<nav “Main”>
[link → DeskEvents.dc.html: Events]
[link → HostCreate.dc.html: Host]
[link → HostPast.dc.html: Past events]
[button: SR] (aria: Account: Sam R.)
# Events
Join events near you to view, add and save photos from this computer.
▸ if hosting:
  [icon: Hosting]
  You're hosting Rooftop birthday on this computer
  You can still join events on this network. Your event keeps priority, and this computer won't switch networks.
  [link → HostLive.dc.html: Open live event]
## Your events
▸ for each ev in mine:
  {{ev.name}}
  {{ev.role}}
  {{ev.meta}}
  [link → {{ev.href}}: {{ev.action}}]
## Nearby
On "Studio-5G" · open and password events
▸ for each n in nearby:
  {{n.name}}
  ▸ if n.locked:
    [icon: Password required]
  {{n.meta}}
  [link → DeskEventView.dc.html: Join]
Don't see it? Invite-only events don't show here. Ask for the link.
label: Join with a link
Paste the invite link from the host's QR code or a message.
[input:text value="http://192.168.1.31:7420/e/crit#t=…"]
[link → DeskEventView.dc.html: Join event]
## On this computer you can
Add photos by dragging them in
Save originals or compatible JPEGs to a folder
Download every original from an event at once
```

Script (data and state):

```js
class Component extends DCLogic {
  renderVals() {
    const hosting = this.props.hosting ?? true;
    const mine = [];
    if (hosting) {
      mine.push({ name: 'Rooftop birthday', role: 'Hosting', roleBg: '#A5B4FC', roleInk: '#1E1B4B', meta: 'Live on this computer · 23 here · 48 photos', action: 'Open live event', href: 'HostLive.dc.html', thumb: 'linear-gradient(180deg,#2E3168 0%,#6E4C8A 38%,#E79B7A 62%,#3B2A33 63%,#1C1620 100%)' });
    }
    mine.push({ name: 'Studio crit night', role: 'Uploader', roleBg: '#F1EFEA', roleInk: '#3F4349', meta: 'Live · hosted by Priya · 14 here · 62 photos', action: 'Open', href: 'DeskEventView.dc.html', thumb: '<scene>' });
    return {
      hosting,
      mine,
      nearby: [
        { name: 'Saturday trivia', meta: 'Open · 12 here · hosted by Bar laptop', locked: false },
        { name: 'Design club meetup', meta: 'Password · 8 here · hosted by Priya', locked: true },
        { name: 'Pottery open studio', meta: 'Open · 5 here · hosted by Kiln room iPad', locked: false }
      ]
    };
  }
}
```

---

## 13 · Desktop: joined event

File: `DeskEventView.dc.html` · Page title: Locale desktop · Joined event

Tweaks:
- `platform` (enum): macOS | Windows — default `macOS`

Outline:

```
Locale
<nav “Main”>
[link → DeskEvents.dc.html: Events]
[link → HostCreate.dc.html: Host]
[link → HostPast.dc.html: Past events]
[button: SR] (aria: Account: Sam R.)
[link → DeskEvents.dc.html: All events]
# Studio crit night
Uploader
Hosted by Priya · Thu 6:00 – 9:00 PM · photos delete Fri 9:00 PM
[button: Download all originals (62)]
[button: Add photos]
<section “Latest post from the host”>
Pinned · Priya (host) · 7:40 PM
Second round of crits starts at 8. Upload your work-in-progress shots before then so we can pull them up on the big screen.
[link → #posts: All posts (3)]
<div “Event sections”>
[button: Photos 62]
[button: Posts 3]
[button: Attendees 14]
<div “Show photos by”>
▸ for each pp in people:
  [button: {{pp.initial}} {{pp.name}} {{pp.count}}] (aria: {{pp.aria}})
## 7:00 – 7:30 PM · 18 photos
[button: Newest]
▸ for each p in photos:
  ▸ if p.saved:
  [button: {{p.initial}} {{p.time}} ✓] (aria: {{p.label}})
Drop photos anywhere
to add them to this event. JPEG, HEIC or PNG.
HEIC
12 MP · HDR
Photo by Priya · Thu 7:12 PM
Saved by 6 people
legend: Save as
label: [input:radio checked] Original · HEIC Exact file, with HDR · 3.4 MB
label: [input:radio checked] Compatible · JPEG {{compNote}}
[button: Save to folder]
{{folder}}
[button: Change]
```

Script (data and state):

```js
class Component extends DCLogic {
  renderVals() {
    const S = {
      dusk: 'linear-gradient(180deg,#2E3168 0%,#6E4C8A 38%,#E79B7A 62%,#3B2A33 63%,#1C1620 100%)',
      lights: '<scene>',
      cake: '<scene>',
      terrace: '<scene>',
      portrait: '<scene>',
      city: '<scene>',
      group: '<scene>',
      drinks: '<scene>',
      confetti: '<scene>',
      sky: 'linear-gradient(180deg,#F7C59F 0%,#EF8A7E 40%,#8C5A8F 75%,#3A2F55 100%)'
    };
    const win = (this.props.platform ?? 'macOS') === 'Windows';
    const list = [
      [S.portrait, 'P', '7:12', true, 'span 2', true], [S.city, 'J', '7:14', false, 'span 1', false], [S.drinks, 'K', '7:15', true, 'span 1', false],
      [S.group, 'P', '7:18', false, 'span 1', false], [S.lights, 'M', '7:19', false, 'span 1', false], [S.terrace, 'J', '7:21', false, 'span 1', false],
      [S.sky, 'K', '7:22', false, 'span 1', false], [S.cake, 'P', '7:24', false, 'span 1', false], [S.confetti, 'M', '7:26', false, 'span 1', false],
      [S.dusk, 'J', '7:28', false, 'span 1', false]
    ];
    const sel = (on) => on ? '2px solid #6366F1' : '1px solid #D4D4D8';
    return {
      selected: S.portrait,
      origChecked: !win, compChecked: win,
      origBorder: sel(!win), compBorder: sel(win),
      origBg: !win ? '#F5F6FF' : '#FFFFFF', compBg: win ? '#F5F6FF' : '#FFFFFF',
      compNote: win ? 'Opens in Windows Photos without extra codecs · 5.1 MB' : 'Opens anywhere, no HDR · 5.1 MB',
      folder: win ? 'Downloads\\Locale\\Studio crit night' : '~/Downloads/Locale/Studio crit night',
      chips: [
        { label: 'All', bg: '#16171A', ink: '#FFFFFF', border: '#16171A' },
        { label: 'Most saved', bg: '#FFFFFF', ink: '#16171A', border: '#E2DFD8' },
        { label: 'Not saved yet', bg: '#FFFFFF', ink: '#16171A', border: '#E2DFD8' },
        { label: 'Mine', bg: '#FFFFFF', ink: '#16171A', border: '#E2DFD8' },
        { label: 'Newest first', bg: '#FFFFFF', ink: '#16171A', border: '#E2DFD8' }
      ],
      people: [
        { initial: 'All', name: 'Everyone', count: '62', scene: '#16171A', border: '2px solid #6366F1', bg: '#F5F6FF', sel: 'true', aria: 'Everyone’s photos' },
        { initial: 'P', name: 'Priya', count: '21', scene: S.terrace, border: '1px solid #E2DFD8', bg: '#FFFFFF', sel: 'false', aria: 'Show photos by Priya' },
        { initial: 'J', name: 'Jordan', count: '17', scene: S.portrait, border: '1px solid #E2DFD8', bg: '#FFFFFF', sel: 'false', aria: 'Show photos by Jordan' },
        { initial: 'K', name: 'Kai', count: '12', scene: S.drinks, border: '1px solid #E2DFD8', bg: '#FFFFFF', sel: 'false', aria: 'Show photos by Kai' },
        { initial: 'M', name: 'Mo', count: '8', scene: S.confetti, border: '1px solid #E2DFD8', bg: '#FFFFFF', sel: 'false', aria: 'Show photos by Mo' },
        { initial: 'S', name: 'You', count: '4', scene: S.dusk, border: '1px solid #E2DFD8', bg: '#FFFFFF', sel: 'false', aria: 'Show your photos' }
      ],
      photos: list.map(([sc, i, t, saved, span, selectedTile], k) => ({
        scene: sc, initial: i, time: t, saved, span,
        border: selectedTile ? '3px solid #6366F1' : '0',
        label: 'Photo ' + (k + 1) + ' of 62, ' + t + ' PM' + (saved ? ', saved' : '')
      }))
    };
  }
}
```

