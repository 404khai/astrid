//! App-local NSView responder for trackpad magnification absent from GPUI 0.2.2.
//! No global event tap, permissions, screen capture, or input injection.
#![allow(unexpected_cfgs)] // objc 0.2's sel! macro checks legacy cargo-clippy cfg.
use async_channel::Sender;
use gpui::Window;
use objc::{
    declare::ClassDecl,
    msg_send,
    runtime::{Class, Object, Sel},
    sel, sel_impl,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{cell::RefCell, collections::HashMap};

thread_local! {static SENDERS:RefCell<HashMap<usize,Sender<f32>>>=RefCell::new(HashMap::new());}
unsafe extern "C" {
    fn object_setClass(object: *mut Object, class: *const Class) -> *const Class;
}

extern "C" fn magnify(view: &Object, _: Sel, event: *mut Object) {
    // Cocoa delivers this responder on the main thread. Pass a finite delta over
    // a bounded async channel; never re-enter a borrowed GPUI App from Objective-C.
    let delta: f64 = unsafe { msg_send![event, magnification] };
    if delta.is_finite() {
        SENDERS.with(|senders| {
            if let Some(tx) = senders.borrow().get(&(view as *const _ as usize)) {
                let _ = tx.try_send(delta as f32);
            }
        });
    }
}
pub fn install(window: &Window, sender: Sender<f32>) -> anyhow::Result<()> {
    let handle = HasWindowHandle::window_handle(window)
        .map_err(|error| anyhow::anyhow!("native view unavailable: {error:?}"))?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        anyhow::bail!("macOS AppKit window required");
    };
    let view = handle.ns_view.as_ptr() as *mut Object;
    // SAFETY: the AppKit window owns a live NSView. A subclass preserves all GPUI
    // ivars and inherited methods; only this process's one view changes class.
    unsafe {
        let superclass = (*view).class();
        let subclass = if let Some(class) = Class::get("AstridSpatialMagnifyView") {
            class
        } else {
            let mut decl = ClassDecl::new("AstridSpatialMagnifyView", superclass)
                .ok_or_else(|| anyhow::anyhow!("cannot register magnify responder"))?;
            decl.add_method(
                sel!(magnifyWithEvent:),
                magnify as extern "C" fn(&Object, Sel, *mut Object),
            );
            decl.register()
        };
        object_setClass(view, subclass);
    }
    SENDERS.with(|senders| senders.borrow_mut().insert(view as usize, sender));
    Ok(())
}
// Cocoa magnification is negative for a pinch inward and positive for spreading.
pub fn factor(delta: f32) -> f32 {
    (1. + delta).clamp(0.2, 5.)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pinch_direction_and_limits() {
        assert!(factor(-0.2) < 1.);
        assert!(factor(0.2) > 1.);
        assert_eq!(factor(-100.), 0.2);
        assert_eq!(factor(100.), 5.);
    }
    #[test]
    fn native_selector_delivers_bounded_delta_without_gpui_reentry() {
        extern "C" fn delta(_: &Object, _: Sel) -> f64 {
            0.25
        }
        unsafe {
            let base = Class::get("NSObject").expect("Foundation linked");
            let mut decl = ClassDecl::new("AstridFixtureMagnificationEvent", base).unwrap();
            decl.add_method(
                sel!(magnification),
                delta as extern "C" fn(&Object, Sel) -> f64,
            );
            let event_class = decl.register();
            let event: *mut Object = msg_send![event_class, new];
            let view: *mut Object = msg_send![base, new];
            let (tx, rx) = async_channel::bounded(1);
            SENDERS.with(|s| s.borrow_mut().insert(view as usize, tx));
            magnify(&*view, sel!(magnifyWithEvent:), event);
            magnify(&*view, sel!(magnifyWithEvent:), event); // Full queue never blocks Cocoa.
            assert_eq!(rx.try_recv().unwrap(), 0.25);
            assert!(rx.try_recv().is_err());
            SENDERS.with(|s| s.borrow_mut().remove(&(view as usize)));
            let _: () = msg_send![event, release];
            let _: () = msg_send![view, release];
        }
    }
}
