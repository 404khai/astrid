# Astrid mobile concept

An isolated iOS-first Expo SDK 57 concept on `experiment/mobile`. All app files
and generated native builds live in this folder. The Phase 4 statement from the original experiment is historical; current runtime direction is recorded in `../docs/CURRENT.md`.

## Native simulator app

```sh
cd mobile
npm ci
npm run ios -- --device "iPhone 17" --port 8085
```

This generates `ios/`, builds Astrid with Xcode, and installs it as its own simulator
app (`dev.astrid.mobile`). It does not use Expo Go. The debug build uses Metro for
live updates; keep the development server running. A bundled build that launches
without Metro can be created with `npm run ios -- --configuration Release`.
The supplied app icon is configured in `app.json` and copied to `assets/app-icon.png`.
Generated `ios/` is intentionally ignored; Expo prebuild reproduces it from config.

## UI

- Expo UI SwiftUI `TabView`: Home, Settings, Search.
- SwiftUI `NavigationStack`, native large/inline navigation titles and toolbar.
- SwiftUI scrolling applies `scrollEdgeEffectStyle('soft', 'top')`, matching
  `.scrollEdgeEffectStyle(.soft, for: .top)` on iOS 26+.
- Logo starts in the principal toolbar position and moves to the leading position
  when scrolling collapses the header.
- Expandable project/session list, working filter, archive empty state and search.
- Composer uses the same sheet adapter structure and presentation as MonoCode's
  `NativeSheet.ios.tsx`: zero-size Host, BottomSheet, large detent, visible drag
  indicator, solid `#171717` canvas and RNHostView content.
- Composer card follows the supplied prompt reference: checkout strip, prompt,
  command/reference hint, model selector, paperclip and square send action.

MonoCode's current checkout contains mobile build remnants. The sheet source was
found in its existing worktree at
`/Users/admin/Developer/monocode-worktrees/mc-orch-f0722b6395a6/mobile/src/shared/ui/NativeSheet.ios.tsx`.
That worktree was read only; Astrid uses its own branch in the original checkout.

This is local demo data. Sessions reset when the app restarts. No runtime connection,
model execution, permissions, slash-command parsing, or attachment upload is included.
The paperclip is disabled until attachment support exists.

## Checks

```sh
npm run typecheck
npx expo install --check
npm run export:ios
```

SwiftUI API reference: https://docs.expo.dev/versions/latest/sdk/ui/swift-ui/

Xcode 27 / iOS 27 scene lifecycle support is enabled through
`expo-build-properties` (`ios.enableSceneSupport: true`), following Expo
SDK 57's supported opt-in:
https://github.com/expo/fyi/blob/main/ios-scene-lifecycle.md

Validated on 2026-10-10: TypeScript, Expo dependency compatibility, iOS Hermes
export, and native Xcode Debug build passed. The scene-enabled build installed
and launched on the iPhone 17 / iOS 27 simulator with zero build errors/warnings.
The running app showed the Home list and native tab bar. The large native sheet
opened, prompt entry enabled submit, and closing returned to Home. The simulator
UI tool could not reliably deliver a scroll gesture; logo relocation still needs
a manual swipe check.


## Current integration direction — 2026-10-18

Reuse this existing app for iOS **and Android**, following the
[implementation plan](../docs/plans/remote-mobile-cli.md#mobile-and-minimum-astrid-host-proposal).
The current SwiftUI imports are iOS-specific, not evidence of Android support.
MO-01 introduces shared React Native Meister conversation components and platform
adapters; real streaming, permissions and InstaBox cards wait for the authenticated
Host contract. Runtime and provider/InstaBox credentials stay on the Host.
No networking or native app change was made in the planning/recovery slice.
