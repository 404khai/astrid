//! Narrow native presentation adapter; no model/runtime/tool authority.
use serde::Serialize;
use tauri::WebviewWindow;
#[derive(Clone, Serialize)]
pub struct Material {
    pub mode: &'static str,
}
pub async fn update(window: &WebviewWindow) -> Result<Material, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    window
        .with_webview(move |platform| {
            #[cfg(target_os = "macos")]
            let result = unsafe { native(platform.inner().cast(), platform.ns_window().cast()) };
            #[cfg(not(target_os = "macos"))]
            let result = Material {
                mode: "opaque_no_api",
            };
            let _ = tx.send(result);
        })
        .map_err(|_| "Native material adapter unavailable".to_string())?;
    rx.await
        .map_err(|_| "Native material inspection unavailable".into())
}
#[cfg(target_os = "macos")]
unsafe fn native(
    web: *mut objc2_app_kit::NSView,
    window: *mut objc2_app_kit::NSWindow,
) -> Material {
    use objc2::{
        ClassType, MainThreadMarker, MainThreadOnly,
        rc::Retained,
        runtime::{AnyClass, NSObjectProtocol},
    };
    use objc2_app_kit::{
        NSAutoresizingMaskOptions, NSColor, NSGlassEffectView, NSGlassEffectViewStyle, NSView,
        NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState,
        NSVisualEffectView, NSWindow, NSWorkspace,
    };
    let Some(mtm) = MainThreadMarker::new() else {
        return Material {
            mode: "opaque_adapter_error",
        };
    };
    // Public availability check precedes every 26+ class/selector use.
    let available = AnyClass::get(c"NSGlassEffectView").is_some();
    let reduced = NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceTransparency();
    let forced = std::env::var_os("ASTRID_DESKTOP_OPAQUE").is_some();
    unsafe {
        let web: &NSView = &*web;
        let window: &NSWindow = &*window;
        let Some(root) = window.contentView() else {
            return Material {
                mode: "opaque_adapter_error",
            };
        };
        let mask = NSAutoresizingMaskOptions::ViewWidthSizable
            | NSAutoresizingMaskOptions::ViewHeightSizable;
        let glass = if available {
            root.subviews()
                .iter()
                .find(|v| v.isKindOfClass(NSGlassEffectView::class()))
        } else {
            None
        };
        if !available || reduced || forced {
            // Keep WKWebView containment stable. Detaching it from NSGlassEffectView
            // loses visible WebKit composition on the tested macOS 27 build even
            // though AX content remains present. The renderer covers the material
            // with a fully opaque surface for this mode; do not label it glass.
            window.setBackgroundColor(Some(&NSColor::windowBackgroundColor()));
            return Material {
                mode: if reduced {
                    "opaque_reduced"
                } else if forced {
                    "opaque_requested"
                } else {
                    "opaque_no_api"
                },
            };
        }
        window.setBackgroundColor(Some(&NSColor::clearColor()));
        if glass.is_none() {
            let Some(retained) = Retained::retain(web as *const NSView as *mut NSView) else {
                return Material {
                    mode: "opaque_adapter_error",
                };
            };
            let effect =
                NSGlassEffectView::initWithFrame(NSGlassEffectView::alloc(mtm), root.bounds());
            // Glass embeds WK content; the separate public behind-window material
            // supplies a desktop backdrop to that glass. This base is ordinary
            // vibrancy, not the native Liquid Glass implementation itself.
            let backdrop =
                NSVisualEffectView::initWithFrame(NSVisualEffectView::alloc(mtm), root.bounds());
            backdrop.setMaterial(NSVisualEffectMaterial::HUDWindow);
            backdrop.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
            backdrop.setState(NSVisualEffectState::Active);
            backdrop.setAutoresizingMask(mask);
            effect.setStyle(NSGlassEffectViewStyle::Clear);
            effect.setCornerRadius(24.0);
            effect.setAutoresizingMask(mask);
            web.removeFromSuperview();
            effect.setContentView(Some(&retained));
            root.addSubview(&backdrop);
            root.addSubview(&effect);
            // Preserve Tauri's existing root/container ownership and restore focus after reparenting.
            window.makeFirstResponder(Some(&retained));
        }
        Material {
            mode: "liquid_glass",
        }
    }
}
