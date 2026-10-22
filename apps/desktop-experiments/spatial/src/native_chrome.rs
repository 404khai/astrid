//! Instance-local AppKit islands. GPUI owns layout; AppKit owns glass and text input.
//! All methods and Drop run on the main thread (Rc deliberately prevents Send).
use async_channel::Sender;
use cocoa_foundation::{
    base::{NO, YES, id, nil},
    foundation::{NSPoint, NSRect, NSSize, NSString},
};
use gpui::{App, AssetSource, Bounds, IntoElement, Pixels, Styled, Window, canvas};
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    runtime::{BOOL, Class, Object, Sel},
    sel, sel_impl,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    ffi::CStr,
    rc::Rc,
};

#[derive(Clone, Debug)]
pub enum NativeAction {
    Search,
    Sidebar,
    Overview,
    FocusPrimary,
    Palette,
    Note(String),
    QueryChanged,
    Navigate(bool),
    Accept,
    Dismiss,
}
#[derive(Clone, Copy)]
pub enum Island {
    Composer = 0,
    Mode = 1,
    Search = 2,
    Settings = 3,
    Sidebar = 4,
    Add = 5,
    PaletteInput = 6,
}
struct ActionState {
    sender: Sender<NativeAction>,
    field: id,
    view: id,
}
thread_local! {static ACTIONS:RefCell<HashMap<usize,ActionState>>=RefCell::new(HashMap::new());}
extern "C" fn action(target: &Object, _: Sel, sender: id) {
    // Only AppKit callbacks touch this map; asynchronous delivery avoids borrowing
    // GPUI's App from inside its own Cocoa dispatch stack.
    unsafe {
        let tag: isize = msg_send![sender, tag];
        ACTIONS.with(|actions| {
            let actions = actions.borrow();
            let Some(state) = actions.get(&(target as *const _ as usize)) else {
                return;
            };
            let event = match tag {
                0 => NativeAction::Search,
                1 => NativeAction::Sidebar,
                2 => NativeAction::Palette,
                5 => NativeAction::Accept,
                4 => {
                    let selected: isize = msg_send![sender, selectedSegment];
                    if selected == 0 {
                        NativeAction::Overview
                    } else {
                        NativeAction::FocusPrimary
                    }
                }
                3 => {
                    let value: id = msg_send![state.field, stringValue];
                    let utf8: *const std::ffi::c_char = msg_send![value, UTF8String];
                    if utf8.is_null() {
                        return;
                    }
                    let text = CStr::from_ptr(utf8)
                        .to_string_lossy()
                        .chars()
                        .take(500)
                        .collect::<String>();
                    if text.trim().is_empty() {
                        return;
                    }
                    NativeAction::Note(text)
                }
                _ => return,
            };
            if state.sender.try_send(event).is_ok() {
                if tag == 3 {
                    let empty = ns("");
                    let _: () = msg_send![state.field,setStringValue:empty];
                    let _: () = msg_send![empty, release];
                }
                let window: id = msg_send![state.view, window];
                let _: BOOL = msg_send![window,makeFirstResponder:state.view];
            }
        });
    }
}
extern "C" fn text_changed(target: &Object, _: Sel, _notification: id) {
    ACTIONS.with(|a| {
        if let Some(state) = a.borrow().get(&(target as *const _ as usize)) {
            let _ = state.sender.try_send(NativeAction::QueryChanged);
        }
    });
}
extern "C" fn text_command(
    target: &Object,
    _: Sel,
    _control: id,
    _editor: id,
    command: Sel,
) -> BOOL {
    let event = if command == sel!(moveUp:) {
        NativeAction::Navigate(true)
    } else if command == sel!(moveDown:) {
        NativeAction::Navigate(false)
    } else if command == sel!(insertNewline:) {
        NativeAction::Accept
    } else if command == sel!(cancelOperation:) {
        NativeAction::Dismiss
    } else {
        return NO;
    };
    ACTIONS.with(|a| {
        if let Some(state) = a.borrow().get(&(target as *const _ as usize)) {
            let _ = state.sender.try_send(event);
        }
    });
    YES
}
unsafe fn ns(value: &str) -> id {
    unsafe { NSString::alloc(nil).init_str(value) }
}
fn rect(x: f64, y: f64, w: f64, h: f64) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(w, h))
}
struct Part {
    glass: id,
    content: id,
    controls: Vec<id>,
}
pub struct NativeChrome {
    parts: Vec<Part>,
    target: id,
    view: id,
    pub liquid_glass: bool,
    overlay: Cell<(bool, bool)>,
    focus_palette: Cell<bool>,
}
impl NativeChrome {
    pub fn new(
        window: &Window,
        cx: &App,
        sender: Sender<NativeAction>,
    ) -> anyhow::Result<Rc<Self>> {
        let handle =
            HasWindowHandle::window_handle(window).map_err(|e| anyhow::anyhow!("{e:?}"))?;
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            anyhow::bail!("AppKit required")
        };
        let view = handle.ns_view.as_ptr() as id;
        unsafe {
            let target_class = if let Some(c) = Class::get("AstridChromeActions") {
                c
            } else {
                let mut c = ClassDecl::new("AstridChromeActions", class!(NSObject))
                    .ok_or_else(|| anyhow::anyhow!("register native actions"))?;
                c.add_method(sel!(perform:), action as extern "C" fn(&Object, Sel, id));
                c.add_method(
                    sel!(controlTextDidChange:),
                    text_changed as extern "C" fn(&Object, Sel, id),
                );
                c.add_method(
                    sel!(control:textView:doCommandBySelector:),
                    text_command as extern "C" fn(&Object, Sel, id, id, Sel) -> BOOL,
                );
                c.register()
            };
            let target: id = msg_send![target_class, new];
            let liquid_glass = Class::get("NSGlassEffectView").is_some();
            let mut parts = vec![];
            for index in 0..7 {
                let glass: id = msg_send![
                    if index == 6 {
                        class!(NSView)
                    } else if liquid_glass {
                        Class::get("NSGlassEffectView").unwrap()
                    } else {
                        class!(NSVisualEffectView)
                    },
                    new
                ];
                if liquid_glass && index != 6 {
                    let _: () = msg_send![glass,setStyle:0isize];
                    let _: () = msg_send![glass,setCornerRadius:28f64];
                    let interactive: BOOL =
                        msg_send![glass,respondsToSelector:sel!(setEffectIsInteractive:)];
                    if interactive == YES {
                        let _: () = msg_send![glass,setEffectIsInteractive:YES];
                    }
                } else if index != 6 {
                    let _: () = msg_send![glass,setMaterial:13isize];
                    let _: () = msg_send![glass,setBlendingMode:1isize];
                    let _: () = msg_send![glass,setState:1isize];
                    let _: () = msg_send![glass,setWantsLayer:YES];
                    let layer: id = msg_send![glass, layer];
                    let _: () = msg_send![layer,setCornerRadius:28f64];
                }
                let name = ns("NSAppearanceNameDarkAqua");
                let appearance: id = msg_send![class!(NSAppearance),appearanceNamed:name];
                let _: () = msg_send![glass,setAppearance:appearance];
                let _: () = msg_send![name, release];
                let content: id = msg_send![class!(NSView), new];
                if liquid_glass && index != 6 {
                    let _: () = msg_send![glass,setContentView:content];
                } else {
                    let _: () = msg_send![glass,addSubview:content];
                }
                let mut controls = vec![];
                if index == 0 || index == 6 {
                    let field: id = msg_send![class!(NSTextField), new];
                    let _: () = msg_send![field,setBezeled:NO];
                    let _: () = msg_send![field,setDrawsBackground:NO];
                    let _: () = msg_send![field,setFocusRingType:1isize];
                    let white: id = msg_send![class!(NSColor), whiteColor];
                    let _: () = msg_send![field,setTextColor:white];
                    let font: id = msg_send![class!(NSFont),systemFontOfSize:15f64];
                    let _: () = msg_send![field,setFont:font];
                    let placeholder = ns(if index == 0 {
                        "Ask Astrid · mock note"
                    } else {
                        "Type a session, identity, or command…"
                    });
                    let _: () = msg_send![field,setPlaceholderString:placeholder];
                    let _: () = msg_send![placeholder, release];
                    let _: () = msg_send![field,setTag:3isize];
                    let _: () = msg_send![field,setTarget:target];
                    let _: () = msg_send![field,setAction:sel!(perform:)];
                    let _: () = msg_send![content,addSubview:field];
                    controls.push(field);
                    if index == 0 {
                        controls.push(button(
                            content,
                            target,
                            3,
                            "Sent",
                            "Add local mock note",
                            cx,
                        )?);
                    } else {
                        let _: () = msg_send![field,setDelegate:target];
                        let _: () = msg_send![field,setTag:5isize];
                    }
                } else if index == 1 {
                    let control: id = msg_send![class!(NSSegmentedControl), new];
                    let _: () = msg_send![control,setSegmentCount:2isize];
                    let _: () = msg_send![control,setTrackingMode:0isize];
                    for (segment, icon) in ["Computer", "Meister"].iter().enumerate() {
                        let image = native_image(icon)?;
                        let _: () = msg_send![control,setImage:image forSegment:segment as isize];
                        let _: () = msg_send![image, release];
                        let _: () = msg_send![control,setWidth:38f64 forSegment:segment as isize];
                    }
                    let _: () = msg_send![control,setSelectedSegment:0isize];
                    let _: () = msg_send![control,setTag:4isize];
                    let _: () = msg_send![control,setTarget:target];
                    let _: () = msg_send![control,setAction:sel!(perform:)];
                    let title = ns("Folder overview or bot home");
                    let _: () = msg_send![control,setAccessibilityLabel:title];
                    let _: () = msg_send![title, release];
                    let _: () = msg_send![content,addSubview:control];
                    controls.push(control);
                } else {
                    let (tag, icon, title) = match index {
                        2 => (0, "Search01", "Find a mock session"),
                        3 => (2, "Settings01", "Workspace commands"),
                        5 => (2, "Add01", "Mock workspace commands"),
                        _ => (1, "SidebarLeft", "Toggle sidebar"),
                    };
                    controls.push(button(content, target, tag, icon, title, cx)?);
                }
                if index != 6 {
                    let _: () = msg_send![view,addSubview:glass];
                }
                parts.push(Part {
                    glass,
                    content,
                    controls,
                });
            }
            ACTIONS.with(|actions| {
                actions.borrow_mut().insert(
                    target as usize,
                    ActionState {
                        sender,
                        field: parts[0].controls[0],
                        view,
                    },
                )
            });
            Ok(Rc::new(Self {
                parts,
                target,
                view,
                liquid_glass,
                overlay: Cell::new((false, false)),
                focus_palette: Cell::new(false),
            }))
        }
    }
    pub fn focus_canvas(&self) {
        unsafe {
            let window: id = msg_send![self.view, window];
            let _: BOOL = msg_send![window,makeFirstResponder:self.view];
        }
    }
    pub fn set_reduced_motion(&self, reduced: bool) {
        unsafe {
            for part in &self.parts {
                let supported: BOOL =
                    msg_send![part.glass,respondsToSelector:sel!(setEffectIsInteractive:)];
                if supported == YES {
                    let _: () =
                        msg_send![part.glass,setEffectIsInteractive:if reduced{NO}else{YES}];
                }
            }
        }
    }
    pub fn sync_focus(&self, focused: bool) {
        unsafe {
            let _: () = msg_send![self.parts[1].controls[0],setSelectedSegment:if focused{1isize}else{0isize}];
        }
    }
    pub fn query(&self) -> String {
        unsafe {
            let value: id = msg_send![self.parts[6].controls[0], stringValue];
            let ptr: *const std::ffi::c_char = msg_send![value, UTF8String];
            if ptr.is_null() {
                String::new()
            } else {
                CStr::from_ptr(ptr)
                    .to_string_lossy()
                    .chars()
                    .take(500)
                    .collect()
            }
        }
    }
    pub fn sync_overlay(&self, palette: bool, artifact: bool) {
        if self.overlay.replace((palette, artifact)) != (palette, artifact) {
            unsafe {
                for (index, part) in self.parts.iter().enumerate() {
                    let hidden = if index == 6 {
                        !palette
                    } else {
                        palette || artifact
                    };
                    if hidden {
                        let _: () = msg_send![part.glass, removeFromSuperview];
                    } else {
                        let _: () = msg_send![self.view,addSubview:part.glass];
                    }
                }
                if palette {
                    let empty = ns("");
                    let _: () = msg_send![self.parts[6].controls[0],setStringValue:empty];
                    let _: () = msg_send![empty, release];
                    self.focus_palette.set(true);
                } else {
                    self.focus_canvas();
                }
            }
        }
    }
    fn place(&self, which: Island, bounds: Bounds<Pixels>, window: &Window) {
        let part = &self.parts[which as usize];
        let w: f32 = bounds.size.width.into();
        let h: f32 = bounds.size.height.into();
        unsafe {
            let flipped: BOOL = msg_send![self.view, isFlipped];
            let client_h: f32 = window.viewport_size().height.into();
            let y: f32 = bounds.origin.y.into();
            let y = native_y(y, h, client_h, flipped == YES);
            let x: f32 = bounds.origin.x.into();
            let _: () = msg_send![part.glass,setFrame:rect(x as f64,y as f64,w as f64,h as f64)];
            let _: () = msg_send![part.content,setFrame:rect(0.,0.,w as f64,h as f64)];
            match which {
                Island::PaletteInput => {
                    let _: () = msg_send![part.controls[0],setFrame:rect(0.,(h as f64-24.)/2.,w as f64,24.)];
                    if self.focus_palette.replace(false) {
                        let window: id = msg_send![self.view, window];
                        let _: BOOL = msg_send![window,makeFirstResponder:part.controls[0]];
                    }
                }
                Island::Composer => {
                    let _: () = msg_send![part.controls[0],setFrame:rect(24.,(h as f64-24.)/2.,(w as f64-88.).max(40.),24.)];
                    let _: () = msg_send![part.controls[1],setFrame:rect(w as f64-52.,(h as f64-40.)/2.,40.,40.)];
                }
                Island::Mode => {
                    let _: () =
                        msg_send![part.controls[0],setFrame:rect(4.,4.,w as f64-8.,h as f64-8.)];
                }
                _ => {
                    let _: () =
                        msg_send![part.controls[0],setFrame:rect(4.,4.,w as f64-8.,h as f64-8.)];
                }
            }
        }
    }
}
fn native_image(icon: &str) -> anyhow::Result<id> {
    unsafe {
        let bytes = if icon == "Meister" {
            std::borrow::Cow::Borrowed(include_bytes!("../assets/avatars/0.svg").as_slice())
        } else {
            crate::icons::Icons
                .load(&format!("{icon}.svg"))?
                .ok_or_else(|| anyhow::anyhow!("missing icon {icon}"))?
        };
        let tree = resvg::usvg::Tree::from_data(&bytes, &resvg::usvg::Options::default())?;
        let mut pixmap = resvg::tiny_skia::Pixmap::new(48, 48)
            .ok_or_else(|| anyhow::anyhow!("allocate icon"))?;
        let scale = 48. / tree.size().width();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );
        let png = pixmap.encode_png()?;
        let data: id = msg_send![class!(NSData),dataWithBytes:png.as_ptr() length:png.len()];
        let image: id = msg_send![class!(NSImage), alloc];
        let image: id = msg_send![image,initWithData:data];
        let _: () = msg_send![image,setTemplate:if icon=="Meister"{NO}else{YES}];
        let _: () = msg_send![image,setSize:NSSize::new(21.,21.)];
        Ok(image)
    }
}
unsafe fn button(
    parent: id,
    target: id,
    tag: isize,
    icon: &str,
    title: &str,
    _cx: &App,
) -> anyhow::Result<id> {
    unsafe {
        let image = native_image(icon)?;
        let button: id = msg_send![class!(NSButton), new];
        let _: () = msg_send![button,setImage:image];
        let _: () = msg_send![image, release];
        let _: () = msg_send![button,setImagePosition:1isize];
        let _: () = msg_send![button,setBordered:NO];
        let _: () = msg_send![button,setTag:tag];
        let _: () = msg_send![button,setTarget:target];
        let _: () = msg_send![button,setAction:sel!(perform:)];
        let title = ns(title);
        let _: () = msg_send![button,setToolTip:title];
        let _: () = msg_send![button,setAccessibilityLabel:title];
        let _: () = msg_send![title, release];
        let _: () = msg_send![parent,addSubview:button];
        Ok(button)
    }
}
impl Drop for NativeChrome {
    fn drop(&mut self) {
        unsafe {
            ACTIONS.with(|actions| actions.borrow_mut().remove(&(self.target as usize)));
            for part in &self.parts {
                let _: () = msg_send![part.glass, removeFromSuperview];
                for c in &part.controls {
                    let _: () = msg_send![*c, release];
                }
                let _: () = msg_send![part.content, release];
                let _: () = msg_send![part.glass, release];
            }
            let _: () = msg_send![self.target, release];
        }
    }
}
/// GPUI prepaint is the source of truth for native island frames after resize.
pub fn host(chrome: Rc<NativeChrome>, which: Island) -> impl IntoElement + Styled {
    canvas(
        move |bounds, window, _| chrome.place(which, bounds, window),
        |_, _, _, _| (),
    )
}

fn native_y(y: f32, height: f32, client_height: f32, flipped: bool) -> f32 {
    if flipped {
        y
    } else {
        client_height - y - height
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_layout_converts_origin_without_retina_scaling() {
        assert_eq!(native_y(34., 58., 900., false), 808.);
        assert_eq!(native_y(34., 58., 900., true), 34.);
        assert_eq!(native_y(34., 58., 560., false), 468.);
    }
    #[test]
    fn native_palette_delegates_keyboard_without_app_reentry() {
        unsafe {
            let target: id = msg_send![class!(NSObject), new];
            let (tx, rx) = async_channel::bounded(8);
            ACTIONS.with(|a| {
                a.borrow_mut().insert(
                    target as usize,
                    ActionState {
                        sender: tx,
                        field: nil,
                        view: nil,
                    },
                )
            });
            assert_eq!(
                text_command(
                    &*target,
                    sel!(control:textView:doCommandBySelector:),
                    nil,
                    nil,
                    sel!(moveDown:)
                ),
                YES
            );
            assert!(matches!(
                rx.try_recv().unwrap(),
                NativeAction::Navigate(false)
            ));
            text_changed(&*target, sel!(controlTextDidChange:), nil);
            assert!(matches!(rx.try_recv().unwrap(), NativeAction::QueryChanged));
            assert_eq!(
                text_command(
                    &*target,
                    sel!(control:textView:doCommandBySelector:),
                    nil,
                    nil,
                    sel!(insertNewline:)
                ),
                YES
            );
            assert!(matches!(rx.try_recv().unwrap(), NativeAction::Accept));
            assert_eq!(
                text_command(
                    &*target,
                    sel!(control:textView:doCommandBySelector:),
                    nil,
                    nil,
                    sel!(cancelOperation:)
                ),
                YES
            );
            assert!(matches!(rx.try_recv().unwrap(), NativeAction::Dismiss));
            assert_eq!(
                text_command(
                    &*target,
                    sel!(control:textView:doCommandBySelector:),
                    nil,
                    nil,
                    sel!(moveLeft:)
                ),
                NO
            );
            ACTIONS.with(|a| a.borrow_mut().remove(&(target as usize)));
            let _: () = msg_send![target, release];
        }
    }
}
