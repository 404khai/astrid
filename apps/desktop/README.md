# Astrid desktop — M02

Tauri 2 macOS client for Astrid's headless Rust runtime. One application-lifetime
host owns the selected workspace/store and active run. Closing the window hides it;
explicit Quit cancels active work, awaits cleanup and attempts an idle save. There
is no independent daemon, active-run recovery or new OS tool authority.

From the repository root:

```sh
npm --prefix apps/desktop ci
npm --prefix apps/desktop/ui ci
npm --prefix apps/desktop/ui run typecheck
npm --prefix apps/desktop/ui test
npm --prefix apps/desktop run build -- --debug
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --manifest-path apps/desktop/src-tauri/Cargo.toml
cargo fmt --check --manifest-path apps/desktop/src-tauri/Cargo.toml
cargo clippy --locked --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings
```

The default bundle is `apps/desktop/src-tauri/target/debug/bundle/macos/Astrid.app`.
A release build omits `--debug`. Current acceptance used a debug bundle, not a
signed/notarized distribution. `CARGO_TARGET_DIR` can reuse a separate local build
cache; it changes output location, not the client/runtime boundary.

Uses existing Astrid ChatGPT credentials; explicit sign-in delegates to the existing
Rust auth implementation. `ASTRID_MODEL` selects the model (desktop default
`gpt-5.6-sol`). Notes starts read Allow/write Deny/execute Deny. Coding Ask/Auto/
Unbound retain existing meanings, including broad local-account shell authority.
Selecting a workflow affects the next run and is not a persisted grant. Retrieved
source content is sent to the configured provider. Model text and tool/file content
are rendered as inert text. Metadata tracing is currently disabled for desktop
runs; the UI does not invent token, cost or throughput metrics.

Public `NSGlassEffectView` supplies genuine native Liquid Glass on macOS 26+.
Tauri's pinned transparent WKWebView uses private `drawsBackground`; this remains
an explicit compatibility/distribution limitation. Reduced Transparency and
unsupported APIs use an opaque surface labeled as fallback. Dynamic accessibility
transitions keep WK containment stable: detaching the live WKWebView caused blank
composition on the tested OS. No CSS blur or ordinary vibrancy is claimed as glass.
See [material evidence](../../docs/research/m02-liquid-glass.md).

Projection limits: 2 KiB per text, 16 KiB history/activity batch, at most 16 history
messages/page, 256 KiB activity ring, one subscription/one outstanding batch, 10s
acknowledgement deadline, 64 KiB snapshot, 80 visible activity nodes and 32,768
UTF-16 units of provisional output. Gaps/truncation are disclosed. These are client
projection limits, not a bound on existing session/provider memory. Maximum 32
retained request IDs per application lifetime prevents replay without eviction;
quit/relaunch is required after that limit. Run ceiling is 20 model calls.

The private desktop selection preference stores only a versioned folder path.
`ASTRID_DESKTOP_FOLDER` is a trusted local launch override useful for synthetic
acceptance; it does not replace the saved preference. No renderer command accepts
a raw workspace path, principal or permission policy. Windows/Linux, overlays,
login-browser roundtrip and release distribution need separate actual evidence.
