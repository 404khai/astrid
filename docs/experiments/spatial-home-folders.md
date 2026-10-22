# Bot home and spatial folders

Follow-up scope: mock presentation only; retain GPUI 0.2.2 and the existing AppKit glass, composer, segmented control, search field, and sidebar button.

Reference decisions:
- `floating_ui_home.png`: scenic full-window canvas, a quiet centered Astrid greeting and floating composer, translucent collapsible left navigation, bot avatars at the bottom. Open here by default.
- `spatial_folders.png`: a separate folder overview, overlapping colored folders with readable identity tabs, front folder identity, and floating controls.
- `Folder Stack Svg.svg`: adopt the rounded folder silhouette and raised, offset tabs. Each folder is its own interactive surface; the supplied image is not used as a static substitute.

Milestones:
1. Home view, bot selection, native sidebar toggle, keyboard equivalents.
2. Horizontal trackpad bot switching with gesture locking; preserve session carousel and pinch navigation.
3. Folder stack with interruptible hover lift, reduced motion, keyboard focus/open, and routes to existing spatial cards.
4. Build, deterministic interaction checks, native launch/visual inspection, checkpoint.

Bot identities use the five existing offline avatar fixtures: Meister, Shelly, Nora, Corbin, and Brief. Switching bots changes presentation selection only. Manually placed session cards remain untouched. No runtime connection, persistence, new dependencies, or Host contract.

## Checkpoint — 2026-10-22

Implemented milestones 1–3. Home is the default screen, with offline scenic artwork, centered Astrid identity, floating native composer, mock disclosure, five selectable bots and recent mock sessions. Native AppKit sidebar button and ⌘B collapse/restore navigation. The existing native segmented control selects folders/home. Spatial canvas remains available through home navigation and the command palette. Hugeicons remain the control icon source; avatars remain bundled Blobatar assets.

Horizontal scrolling uses GPUI's AppKit-backed scroll events. A horizontal threshold, dominant-axis check, gesture-start reset, consumed latch and idle-gap fallback prevent momentum from repeatedly switching bots. Arrow keys provide an alternative. Existing instance-local AppKit magnification still opens the spatial canvas and zooms toward the pointer; no global event capture or permissions were introduced.

Folders are five separate cached artwork plates derived from the supplied rounded/tab silhouette, with independent GPUI hit regions, labels, hover lift and click-to-focus. Hover artwork moves inside a stationary hit region. Lift reverses from its current value; reduced motion snaps. Keyboard selection is named in the heading, raises the selected folder, and Return opens its session. Card world placements and pinning remain separate and unchanged by bot/folder navigation.

Verification: 21 deterministic tests pass, including swipe direction/locking/vertical rejection, bot wrapping, hover reversal/reduced motion, offline folder rasterization and the previous canvas/native delegate checks. Locked debug build, formatting and clippy with warnings denied pass. Upstream `block 0.1.6` retains its existing future-compatibility warning.

Launched the real updated native application. Observed opening home, collapsed sidebar after native button activation, colored folder stack after native segmented selection, keyboard folder opening into session carousel, and a horizontal scroll changing the home avatar/bot. Capture service provided Stage Manager thumbnails rather than full-resolution front-window images; full-resolution visual acceptance remains pending. CUA has no pointer-hover-only API or physical pinch gesture, so end-to-end hover/pinch feel was not claimed as verified.

Files: `src/home.rs` (presentation navigation/swipe/lift math), `src/home_ui.rs` (home/sidebar/folders), `src/main.rs` (routing/layout/input), `src/native_chrome.rs` (native action routing), `src/art.rs` (folder plates), README and this plan. No runtime, root Cargo, security, persistence or unrelated checkout edits.

Next incomplete task: physical trackpad pinch/swipe and folder-hover acceptance at full window resolution, followed by resize/accessibility/IME QA and a fresh post-AppKit frame-pacing benchmark. No new performance claim is made from this follow-up. Folder stack is a 2D layered treatment, without perspective transforms. Scenic artwork is the existing offline illustrated scene, not the photograph in the reference. Custom GPUI sidebar/folders still need richer accessibility semantics; AppKit supplies actual native controls and editing.
