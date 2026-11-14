# ADR 0039 — Phone companion: Android overlay and QR-paired link to the Mac

Date: 2026-11-14
Status: **Accepted 2026-11-14.** The maintainer asked for V2-06 "for my phone.
it should be an overlay, when I first activate it, it should also use the
distortion wave along the edges of my phone screen ... I want to be able to load
it onto my android phone ... should we add qr code connection to my desktop app?
with syncing of messages. it should also be able to use the spatial weaver engine
(canvas on my phone)". Asked how the phone should connect, given that it needs a
network listener on the Mac, the maintainer chose **"QR pairing + sync
(Recommended)"** as proposed: pairing only while the setting is on, a LAN-only
WebSocket, a two-minute one-time QR secret, one revocable key per phone, every
frame sealed with AES-GCM, chats synced through the Mac's store, chats kept in
memory on the phone, and no mail for phone asks.
Related: ADRs 0026 (presence, intro wave amendment), 0033 (ask surface), 0036
(live canvas), 0037 (desktop app, chat store); [v2 gameplan](../plans/v2-gameplan.md)
V2-06; the HO-01 trust baseline in [remote-mobile-cli](../plans/remote-mobile-cli.md).

## Context

V2-06 is the phone companion: phone asks routed to the Mac runtime, plus an
Android overlay prototype. The phone can't run Astrid's runtime or hold model
credentials (thin-client principle; credentials stay host-side). So phone asks
have to reach the Mac, and that needs a listener on the Mac. That's the
cross-device trust decision the remote-mobile plan held back until there was a
concrete contract. This ADR is that contract, for one paired phone on the same
local network as a running desktop app. It isn't a general Host API.

## Decision

### Stack

- **The phone app** is the existing Expo/React Native app in `mobile/`, now with
  Android added (`dev.astrid.mobile`). `App.android.tsx` is the Android app; the
  iOS concept (`App.tsx`, SwiftUI) is untouched.
- **The overlay** is a local Expo module in Kotlin, `mobile/modules/astrid-overlay`:
  - **Bubble.** A foreground service (type `specialUse`, with a notification
    that has Stop) keeps Astrid's mark as a draggable bubble over other apps.
    Drawing over other apps needs the user to allow it in system settings.
  - **Ask panel.** Tapping the bubble opens it in a see-through activity
    (`AskActivity`) running the app's `AstridAsk` component on the same React
    runtime. The panel and the app share one link.
  - **Wave.** The first time the overlay is switched on, the Distortion Waves
    band (the same vendored model as presence's intro, ADR 0026 amendment) rises
    from the bottom middle to the top and leaves its colour on the screen's
    rounded edges. On Android 13+ it's an AGSL `RuntimeShader` ported from the
    Metal shader; older phones get a plain glow round the edges.
  - **What it can't do.** The wave and bubble windows take no focus, and the
    wave takes no touches. It's skipped when animations are off. Neither reads
    the screen.
- **The canvas** is the live canvas page (`web/canvas.html`, ADR 0036), built
  into one HTML file (`web/vite.phone.config.ts`, `mobile/scripts/build-canvas.mjs`)
  and loaded from the APK's assets in a WebView. It gets the same validated
  shapes, through the same `window.astrid.receive` bridge. It doesn't load
  anything else, and navigation away is refused. Two-finger pinch zoom was added
  to the shared page for touch screens.

### Link

- **The listener.** The desktop app (`PhoneLink.swift`) runs a WebSocket server
  on TCP port 47821 (`Network.framework`):
  - **When.** Only while Settings › Phone › "Let paired phones connect" is on
    and the app is running. Quitting the app ends the link.
  - **Who.** Peers outside loopback, private IPv4, link-local or IPv6 unique-local
    ranges are dropped, and cellular interfaces are excluded.
  - **Limits.** At most four connections. A connection that hasn't proved a key
    within 10 s is closed.
- **Pairing.** "Pair Phone" shows a QR code:
  `astrid://pair?v=1&h=<LAN addresses>&p=47821&s=<32-byte secret>&n=<Mac name>`.
  - **The secret.** It works for 120 s and is used up by the first phone that
    proves it, so a wrong guess doesn't use it up.
  - **The key.** The Mac then creates a device ID and a random 32-byte device
    key, sends both sealed under the pairing key, and closes.
  - **Where keys live.** The phone keeps them in `expo-secure-store` (Android
    Keystore-backed). The Mac keeps them in `~/Library/Application
    Support/Astrid/Devices.json` (0600 in a 0700 folder, next to the chats it
    already protects that way).
  - **Limit.** At most eight phones are paired. Revoking one in Settings
    deletes its key and closes its connections.
- **Handshake and sealing** (`LinkSession`, `LinkCrypto`, mirrored in
  `mobile/src/link`):
  1. Text hellos carry the version, the device ID (or `null` to pair) and a
     32-byte nonce from each side.
  2. The connection key is HKDF-SHA256 over the device key (or the pairing
     secret), salted with both nonces, with info `astrid-link/1 <device|pair>`.
  3. Every later frame is binary AES-256-GCM. The 12-byte nonce is a 4-byte
     direction (phone 0, Mac 1) followed by an 8-byte counter, and the receiver
     expects exactly the next count. Replayed, reordered, reflected or tampered
     frames close the connection, and so does a text frame after the hello.
  4. Inbound frames are capped at 64 KiB. Outbound frames are capped at one
     chat file (8 MiB) plus overhead.
  5. A fixed vector is checked by both the Swift and the JS tests.
- **No trust on first use.** The QR code is the out-of-band channel: knowing
  the secret proves the phone saw this Mac's screen.

### What a phone can do

These are the only requests (`PhoneRequest`); anything else is refused:

- `sync`: list the chats.
- `open`: open one chat and get its updates.
- `ask`: send text to a chat or start a new one.
- `cancel`: stop an ask this phone started.
- `ping`.

How asks behave:

- **No mail.** Phone asks run through the desktop's chat model, but in a
  second `astrid presence-ask` child with no mail source. A phone ask can't
  reach mail, and phones can't answer approvals, since mail approvals are
  answered on the Mac. Otherwise they're the same read-only presence asks
  (ADR 0033), with drawing (ADR 0036).
- **One at a time.** One ask runs at a time, phone or desktop.

How chats sync:

- **One store.** A phone ask's chat and reply are saved in the desktop's chat
  store, and the desktop shows them as they stream.
- **Both directions.** The phone gets the chat list on every save, its open
  chat again whenever it changes, and the events of the running ask, whichever
  side started it. So a desktop ask also shows on the phone.
- **Nothing on the phone's disk.** The phone keeps chats in memory only; only
  the pairing is stored.

## Alternatives considered

- **Pairing without sync.** This option was offered and declined. The maintainer
  wanted their messages synced.
- **No link yet**, overlay and demo only. Declined: the phone couldn't ask anything.
- **TLS with a self-signed, pinned certificate.** It's equivalent once the QR
  carries the fingerprint, but it needs certificate generation on the Mac and
  custom trust on Android. Sealing frames under a QR-delivered key gets the same
  guarantees (confidentiality, integrity, peer authentication) without either.
  It's revisitable if the link ever leaves the LAN.
- **A cloud relay, or exposing the runtime beyond the LAN.** Out of scope: no
  public bind, port forwarding or new hosted service.
- **A native Kotlin/Compose app.** Rejected: the existing Expo app is the mobile
  direction, and native code is kept to what Android requires (overlay, service
  and shader).

## Consequences

- **Asks need the Mac.** The phone only works while the desktop app is open
  with the link on, on the same network. The Mac's addresses are in the QR, so
  if its address changes, pair again. There's no discovery yet.
- **The link isn't the runtime.** It belongs to the desktop app, so the
  runtime, presence and the CLI don't change. Background execution still ends
  when the desktop app quits; this is not the daemon.
- **Pairing a phone is a new authority.** The phone can read every desktop chat
  and start asks that use the Mac's model credentials, but it can't use mail or
  approvals, and every ask is still the bounded read-only presence ask.
- **iOS** has no cross-app overlay. The iOS concept app is unchanged here.
- **Verification limits.** Live model asks from the phone, a physical Android
  phone, and the Mac's local-network/firewall prompts need the maintainer.

## Amendment 2026-11-14: chat styling, diagrams on the phone, redesign

The maintainer asked to rename chats and change their icon and colour on both
apps, a Diagrams tab on the phone, a Monocode-style phone layout, Settings with
About on both apps, and the Astrid mark in the middle of the pairing QR.

- **Chat style.** Chats gain optional `icon` and `color` fields, chosen from
  fixed name lists (`ChatStyle`) that each app maps to its own glyphs. Older
  chat files load unchanged. A rename keeps a chat's place in the list.
- **Two more link requests:**
  - `style` (rename, or set or reset the icon or colour of one chat);
  - `diagrams` (the 40 newest drawings with their shapes).
  Both are validated on the Mac. Neither reaches a tool, and a phone can't
  delete a chat.
- **The pairing code** is generated with high error correction (30%), so the
  mark over its middle (about 5% of the code) doesn't stop it scanning. It now
  opens in a modal with a countdown, a new-code button and a copy-link button.
  The copied `astrid://pair` link opens the phone app and pairs it. The link
  carries the same one-time, two-minute secret as the code, so it's only for
  sending to your own phone.
