# M02-00 material fixtures

These independent experiments contain no Astrid runtime or provider integration.
They do not establish host authority, background survival, IPC safety or desktop
acceptance. See [research](../../docs/research/m02-liquid-glass.md).

## Native comparison

```sh
xcrun swiftc -warnings-as-errors -parse-as-library GlassProbe.swift -o /tmp/astrid-glass-probe -framework AppKit -framework WebKit
/tmp/astrid-glass-probe
/tmp/astrid-glass-probe --opaque-fallback
```

The window exits after 180 seconds. The force flag simulates the app-owned opaque
fallback without changing the system preference. Ordinary vibrancy is a comparison,
not Liquid Glass or the accessibility fallback. Probe labels identify the requested
comparison surface; logs identify whether the actual surface is NSGlassEffectView
or opaque NSView. No private WK key is used in the native Swift probe.

For the screenshot point samples:

```sh
xcrun swift ContrastSamples.swift native-public-webview.png > contrast-samples.json
```

Coordinates match that stored screenshot only. These samples are not an
accessibility certification or a complete text-contrast audit.

## Pinned Tauri composition fixture

```sh
cargo build --manifest-path tauri/Cargo.toml --locked --offline -j 2
cargo fmt --manifest-path tauri/Cargo.toml --check
cargo clippy --manifest-path tauri/Cargo.toml --locked --offline --all-targets -j 2 -- -D warnings
tauri/target/debug/astrid-m02-material-probe
tauri/target/debug/astrid-m02-material-probe --opaque-fallback
```

Tauri bundles static HTML/CSS directly; no Node packages, TypeScript or frontend
build pipeline exists in this fixture. No application commands/plugins/capability
grants. Its CSP rejects scripts, network, navigation forms and remote assets.
No interaction represents an actual task or policy. The window exits after 90
seconds. Whole-webview reparenting is disposable feasibility code, not a production
layout/lifecycle contract. Resize, reload, accessibility preference changes and
full keyboard traversal require additional validation before reuse.

`transparent: true` in this pinned Tauri/Wry integration uses private WK API.
NSGlassEffectView is public (macOS 26+). These are separate facts: public native
glass does not make WK transparency a wholly public API path. Older OS uses an
opaque window-background fallback; no older OS execution is claimed.
