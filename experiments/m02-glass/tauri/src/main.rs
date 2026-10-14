// Independent M02-00 material experiment. No Astrid dependency, commands or plugins.
use tauri::Manager;
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let window = app
                .get_webview_window("material-probe")
                .ok_or("missing probe window")?;
            window.with_webview(|platform| {
                #[cfg(target_os = "macos")]
                unsafe {
                    install_material(platform.inner().cast(), platform.ns_window().cast());
                }
            })?;
            window.set_focus()?;
            // Bounded, no app-lifetime runtime behavior inferred.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(90));
                handle.exit(0);
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("material probe failed");
}
#[cfg(target_os = "macos")]
unsafe fn install_material(web: *mut objc2_app_kit::NSView, window: *mut objc2_app_kit::NSWindow) {
    use objc2::{MainThreadMarker, MainThreadOnly, rc::Retained, runtime::AnyClass};
    use objc2_app_kit::{
        NSApplication, NSColor, NSGlassEffectView, NSGlassEffectViewStyle, NSTextField, NSView,
        NSWorkspace,
    };
    use objc2_foundation::{NSPoint, NSRect, NSSize};
    let mtm = MainThreadMarker::new().expect("with_webview must run on main thread");
    // Public class availability check: never send 26+ selectors on an older OS.
    if AnyClass::get(c"NSGlassEffectView").is_none()
        || NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceTransparency()
        || std::env::args().any(|a| a == "--opaque-fallback")
    {
        eprintln!("material=opaque-fallback");
        unsafe {
            (&*window).setBackgroundColor(Some(&NSColor::windowBackgroundColor()));
        }
        return;
    }
    unsafe {
        let web = &*web;
        let window = &*window;
        let root = NSView::initWithFrame(NSView::alloc(mtm), web.frame());
        // Application-owned stripes expose whether WK backing obscures glass.
        for index in 0..10 {
            let stripe = NSTextField::initWithFrame(
                NSTextField::alloc(mtm),
                NSRect::new(
                    NSPoint::new(index as f64 * web.frame().size.width / 10.0, 0.0),
                    NSSize::new(web.frame().size.width / 10.0 + 1.0, web.frame().size.height),
                ),
            );
            stripe.setEditable(false);
            stripe.setBordered(false);
            stripe.setDrawsBackground(true);
            stripe.setBackgroundColor(Some(
                &NSColor::colorWithCalibratedHue_saturation_brightness_alpha(
                    index as f64 / 10.0,
                    0.7,
                    0.7,
                    1.0,
                ),
            ));
            root.addSubview(&stripe);
        }
        let glass = NSGlassEffectView::initWithFrame(NSGlassEffectView::alloc(mtm), web.frame());
        glass.setStyle(NSGlassEffectViewStyle::Regular);
        glass.setCornerRadius(24.0);
        // Keep a strong reference while detaching from the Tauri hierarchy.
        let retained_web =
            Retained::retain(web as *const NSView as *mut NSView).expect("live WKWebView");
        web.removeFromSuperview();
        glass.setContentView(Some(&retained_web));
        root.addSubview(&glass);
        window.setContentView(Some(&root));
        let focused = window.makeFirstResponder(Some(web));
        eprintln!("wk_first_responder={focused}");
        window.setBackgroundColor(Some(&NSColor::systemBlueColor()));
        #[allow(deprecated)]
        NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);
        eprintln!(
            "material=NSGlassEffectView style=regular content=WKWebView tauri=2.11.5 transparency=wry-private-api"
        );
    }
}
