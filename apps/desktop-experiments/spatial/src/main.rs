mod art;
mod fixtures;
mod home;
mod home_ui;
mod icons;
use home::{BOTS, View};
mod magnify;
mod model;
mod native_chrome;
use fixtures::{MockSource, Session, SessionSource, search};
use gpui::{prelude::*, *};
use model::{Camera, Gesture, Layout, V, move_selection, v};
use std::time::Instant;

const INK: u32 = 0xe6edf2;
const MUTED: u32 = 0x8195a5;
const MINT: u32 = 0x00dfc4;
const TOP: f32 = 0.;
const EDGE: f32 = 0.;

#[derive(Clone, Copy)]
enum Command {
    Search,
    Palette,
    Folders,
    Sidebar,
    Bot(usize),
    Overview,
    Home,
    Reset,
    Undo,
    Motion,
    Focus(usize),
    Expand(usize),
    Pin(usize),
    Artifact(usize),
    Send,
    Zoom(f32),
    Empty,
    Dense,
}
struct Transition {
    from: Camera,
    to: Camera,
    start: Instant,
    carousel_from: f32,
    carousel_to: f32,
}
struct Metrics {
    last: Option<Instant>,
    intervals: Vec<f64>,
    render_us: Vec<f64>,
    frames: usize,
    benchmark: bool,
}
struct Spatial {
    screen: View,
    sidebar: bool,
    bot: usize,
    swipe: home::Swipe,
    swipe_last: Option<Instant>,
    folder_hover: Option<usize>,
    folder_lifts: [f32; 5],
    folder_tick: Instant,
    layout: Layout,
    chrome: Option<std::rc::Rc<native_chrome::NativeChrome>>,
    sessions: Vec<Session>,
    gesture: Option<Gesture>,
    focus: FocusHandle,
    focused_space: Option<usize>,
    carousel: f32,
    first_render: bool,
    drag_moved: bool,
    horizontal: f32,
    last_switch: Option<Instant>,
    palette: bool,
    commands: bool,
    query: String,
    result_index: usize,
    composing: bool,
    draft: String,
    notes: Vec<String>,
    artifact: Option<usize>,
    undo: Option<Layout>,
    reduced: bool,
    transition: Option<Transition>,
    viewport: V,
    status: String,
    metrics: Metrics,
}
fn pv(p: Point<Pixels>) -> V {
    v(p.x.into(), p.y.into())
}
fn label(text: impl Into<SharedString>, font: f32, color: u32) -> Div {
    div()
        .text_size(px(font))
        .text_color(rgb(color))
        .child(text.into())
}
fn avatar(id: usize, s: f32) -> impl IntoElement {
    img(art::avatar(id))
        .w(px(40. * s))
        .h(px(40. * s))
        .object_fit(ObjectFit::Contain)
}
impl Spatial {
    fn new(cx: &mut Context<Self>) -> Self {
        let reduced = std::process::Command::new("defaults")
            .args(["read", "com.apple.universalaccess", "reduceMotion"])
            .output()
            .ok()
            .is_some_and(|o| String::from_utf8_lossy(&o.stdout).trim() == "1");
        let mut layout = Layout::default();
        layout.camera.set_limits(0.3, 1.65);
        Self {
            screen: View::Home,
            sidebar: true,
            bot: 0,
            swipe: home::Swipe::default(),
            swipe_last: None,
            folder_hover: None,
            folder_lifts: [0.; 5],
            folder_tick: Instant::now(),
            layout,
            chrome: None,
            sessions: MockSource.sessions(),
            gesture: None,
            focus: cx.focus_handle(),
            focused_space: None,
            carousel: 0.,
            first_render: true,
            drag_moved: false,
            horizontal: 0.,
            last_switch: None,
            palette: false,
            commands: false,
            query: String::new(),
            result_index: 0,
            composing: false,
            draft: String::new(),
            notes: vec![],
            artifact: None,
            undo: None,
            reduced,
            transition: None,
            viewport: v(1400., 730.),
            status: "Mock workspace · Nothing is connected or running".into(),
            metrics: Metrics {
                last: None,
                intervals: vec![],
                render_us: vec![],
                frames: 0,
                benchmark: std::env::args().any(|a| a == "--benchmark"),
            },
        }
    }
    fn local(&self, p: Point<Pixels>) -> V {
        pv(p) - v(EDGE, TOP)
    }
    fn sample_motion(&mut self) {
        if let Some(t) = &self.transition {
            let time = (t.start.elapsed().as_secs_f32() / 0.22).min(1.);
            // GPUI-native camera interpolation. Same strong curve as Emil's ease-out.
            let eased = model::ease_out(time);
            self.layout.camera = t.from.interpolate(t.to, eased);
            self.carousel = t.carousel_from + (t.carousel_to - t.carousel_from) * eased;
            if time >= 1. {
                self.transition = None;
            }
        }
    }
    fn go(&mut self, target: Camera, animated: bool) {
        self.sample_motion();
        if self.reduced || !animated {
            self.layout.camera = target;
            self.transition = None;
        } else {
            self.transition = Some(Transition {
                from: self.layout.camera,
                to: target,
                start: Instant::now(),
                carousel_from: self.carousel,
                carousel_to: self.carousel,
            });
        }
    }
    fn focus_session(&mut self, id: usize, animated: bool) {
        self.screen = View::Canvas;
        self.sample_motion();
        let from = if self.focused_space.is_some() {
            self.carousel
        } else {
            id as f32
        };
        self.focused_space = Some(id);
        self.horizontal = 0.;
        self.layout.focus(id);
        if let Some(c) = self.layout.card(id) {
            self.go(
                {
                    let mut target =
                        self.layout
                            .camera
                            .frame(c.pos, c.size(), self.viewport - v(0., 180.));
                    target.pan = target.pan + v(0., 80.);
                    target
                },
                animated,
            );
            if let Some(t) = &mut self.transition {
                t.carousel_from = from;
                t.carousel_to = id as f32;
            } else {
                self.carousel = id as f32;
            }
        }
    }
    fn run(&mut self, cmd: Command, animated: bool, cx: &mut Context<Self>) {
        self.gesture = None;
        match cmd {
            Command::Sidebar => {
                self.sidebar = !self.sidebar;
            }
            Command::Bot(id) => {
                self.bot = id % BOTS.len();
                self.screen = View::Home;
            }
            Command::Folders => {
                self.screen = View::Folders;
                self.focused_space = None;
            }
            Command::Search | Command::Palette => {
                self.palette = !self.palette;
                self.commands = matches!(cmd, Command::Palette);
                self.query.clear();
                self.result_index = 0;
                self.composing = false;
                self.artifact = None;
            }
            Command::Overview => {
                self.screen = View::Canvas;
                self.focused_space = None;
                let mut target = self.layout.overview(self.viewport - v(40., 180.));
                target.pan = target.pan + v(20., 80.);
                self.go(target, animated);
            }
            Command::Home => {
                self.screen = View::Home;
                self.focused_space = None;
                self.go(Camera::default(), animated);
            }
            Command::Focus(id) => {
                self.palette = false;
                self.focus_session(id, animated);
            }
            Command::Expand(id) => {
                if let Some(c) = self.layout.card_mut(id) {
                    c.expanded = !c.expanded;
                }
                self.layout.focus(id);
            }
            Command::Pin(id) => {
                if let Some(c) = self.layout.card_mut(id) {
                    c.pinned = !c.pinned;
                }
            }
            Command::Artifact(id) => {
                self.artifact = Some(id);
                self.palette = false;
                self.composing = false;
            }
            Command::Reset => {
                self.screen = View::Canvas;
                self.transition = None;
                self.focused_space = None;
                self.undo = Some(self.layout.reset());
                self.first_render = true;
                self.status = "Layout reset · Undo is available".into();
            }
            Command::Undo => {
                if let Some(saved) = self.undo.take() {
                    self.transition = None;
                    self.layout = saved;
                    self.focused_space = None;
                    self.status = "Previous layout restored".into();
                }
            }
            Command::Motion => {
                self.reduced = !self.reduced;
                if let Some(chrome) = &self.chrome {
                    chrome.set_reduced_motion(self.reduced);
                }
                self.transition = None;
                self.status = format!(
                    "Reduced motion {} · Presentation setting only",
                    if self.reduced { "on" } else { "off" }
                );
            }
            Command::Send => {
                if !self.draft.trim().is_empty() {
                    self.notes.push(self.draft.trim().to_string());
                    if self.notes.len() > 8 {
                        self.notes.remove(0);
                    }
                    self.draft.clear();
                    self.status = "Mock note added locally · No message was sent".into();
                    self.focus_session(self.bot, animated);
                }
            }
            Command::Zoom(f) => {
                self.sample_motion();
                self.transition = None;
                self.layout.camera.zoom_at(self.viewport * 0.5, f);
            }
            Command::Empty => {
                self.screen = View::Canvas;
                self.focused_space = None;
                self.undo = Some(self.layout.clone());
                self.layout.cards.clear();
                self.layout.selected = None;
                self.status = "Empty layout fixture · Reset or Undo to recover".into();
            }
            Command::Dense => {
                self.screen = View::Canvas;
                self.undo = Some(self.layout.clone());
                for (i, c) in self.layout.cards.iter_mut().enumerate() {
                    c.pos = v(220. + i as f32 * 48., 100. + i as f32 * 52.);
                    c.expanded = true;
                }
                self.sessions[2].name =
                    "Keihatsu — a deliberately long session title for narrow window verification"
                        .into();
                self.go(self.layout.overview(self.viewport), animated);
            }
        }
        cx.notify();
    }
    fn button(
        &self,
        id: impl Into<ElementId>,
        text: impl Into<SharedString>,
        cmd: Command,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let icon = match cmd {
            Command::Search => "Search01",
            Command::Palette => "Command",
            Command::Overview => "LayoutGrid",
            Command::Home | Command::Reset => "Refresh",
            Command::Undo => "Undo02",
            Command::Motion => "Motion01",
            Command::Focus(_) => "ArrowExpand",
            Command::Expand(id) => {
                if self.layout.card(id).is_some_and(|c| c.expanded) {
                    "ArrowShrink"
                } else {
                    "ArrowExpand"
                }
            }
            Command::Pin(id) => {
                if self.layout.card(id).is_some_and(|c| c.pinned) {
                    "PinOff"
                } else {
                    "Pin"
                }
            }
            Command::Artifact(_) => "Attachment01",
            Command::Send => "Sent",
            Command::Zoom(f) => {
                if f > 1. {
                    "ZoomInArea"
                } else {
                    "ZoomOutArea"
                }
            }
            _ => "LayoutGrid",
        };
        div()
            .id(id)
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .rounded_lg()
            .cursor_pointer()
            .text_size(px(12.))
            .text_color(rgb(INK))
            .hover(|s| s.bg(rgb(0x21313f)))
            .active(|s| s.bg(rgb(0x334654)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    this.run(cmd, true, cx);
                }),
            )
            .child(
                svg()
                    .path(format!("{icon}.svg"))
                    .size(px(16.))
                    .text_color(rgb(INK)),
            )
            .child(text.into())
    }
    fn results(&self) -> Vec<(String, Command)> {
        if self.commands {
            vec![
                ("Overview · Fit all sessions", Command::Overview),
                ("Bot home", Command::Home),
                ("Folder overview", Command::Folders),
                ("Toggle sidebar", Command::Sidebar),
                ("Reset layout · Recover with Undo", Command::Reset),
                ("Undo layout reset", Command::Undo),
                ("Toggle reduced motion", Command::Motion),
                ("Inspect empty layout fixture", Command::Empty),
                ("Inspect overlap + long-title fixture", Command::Dense),
            ]
            .into_iter()
            .filter(|(name, _)| name.to_lowercase().contains(&self.query.to_lowercase()))
            .map(|(n, c)| (n.into(), c))
            .collect()
        } else {
            search(&self.sessions, &self.query)
                .into_iter()
                .filter(|id| self.layout.card(*id).is_some())
                .map(|id| (self.sessions[id].name.clone(), Command::Focus(id)))
                .collect()
        }
    }
    fn key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let k = &event.keystroke;
        let cmd = k.modifiers.platform;
        if cmd && k.key == "q" {
            cx.quit();
            return;
        }
        if cmd && k.key == "b" {
            self.run(Command::Sidebar, false, cx);
            return;
        }
        if !self.palette
            && !self.composing
            && self.screen != View::Canvas
            && (k.key == "left" || k.key == "right")
        {
            self.bot = home::adjacent(self.bot, k.key == "left");
            if self.screen == View::Folders {
                self.folder_hover = Some(self.bot);
            }
            cx.notify();
            return;
        }
        if !self.palette && self.screen == View::Folders && k.key == "enter" {
            self.run(Command::Focus(self.bot), false, cx);
            return;
        }
        if cmd && (k.key == "k" || k.key == "p") {
            self.run(
                if k.key == "p" || k.modifiers.shift {
                    Command::Palette
                } else {
                    Command::Search
                },
                false,
                cx,
            );
            return;
        }
        if k.key == "escape" {
            self.palette = false;
            if let Some(chrome) = &self.chrome {
                chrome.focus_canvas();
            }
            self.artifact = None;
            self.composing = false;
            self.gesture = None;
            self.transition = None;
            cx.notify();
            return;
        }
        if self.palette || self.composing {
            if self.palette && (k.key == "up" || k.key == "down") {
                self.result_index =
                    move_selection(self.result_index, self.results().len(), k.key == "up");
            } else if k.key == "enter" {
                if self.palette {
                    if let Some((_, action)) = self.results().get(self.result_index).cloned() {
                        self.palette = false;
                        self.run(action, false, cx);
                    }
                } else {
                    self.run(Command::Send, false, cx);
                }
            } else if cmd && k.key == "v" {
                if let Some(t) = cx.read_from_clipboard().and_then(|c| c.text()) {
                    self.append_text(&t);
                }
            } else if k.key == "backspace" {
                if self.palette {
                    self.query.pop();
                    self.result_index = 0;
                } else {
                    self.draft.pop();
                }
            } else if !cmd
                && !k.modifiers.control
                && let Some(text) = &k.key_char
            {
                self.append_text(text);
            }
            cx.notify();
            return;
        }
        let action = match k.key.as_str() {
            "0" => Some(Command::Overview),
            "1" => Some(Command::Home),
            "=" | "+" => Some(Command::Zoom(1.15)),
            "-" => Some(Command::Zoom(1. / 1.15)),
            "r" if cmd => Some(Command::Reset),
            "z" if cmd => Some(Command::Undo),
            "m" => Some(Command::Motion),
            "enter" => self.layout.selected.map(Command::Expand),
            "p" => self.layout.selected.map(Command::Pin),
            "tab" => {
                if let Some(id) = self.layout.cycle(k.modifiers.shift) {
                    self.focus_session(id, false);
                }
                None
            }
            "left" | "right" | "up" | "down" => {
                if self.focused_space.is_some() && (k.key == "left" || k.key == "right") {
                    if let Some(id) = self.layout.cycle(k.key == "left") {
                        self.focus_session(id, false);
                    }
                    cx.notify();
                    return;
                }
                self.transition = None;
                self.layout.camera.pan_by(match k.key.as_str() {
                    "left" => v(60., 0.),
                    "right" => v(-60., 0.),
                    "up" => v(0., 60.),
                    _ => v(0., -60.),
                });
                None
            }
            "/" => Some(Command::Palette),
            _ => None,
        };
        if let Some(a) = action {
            self.run(a, false, cx);
        } else {
            cx.notify();
        }
        window.focus(&self.focus);
    }
    fn append_text(&mut self, text: &str) {
        let target = if self.palette {
            self.result_index = 0;
            &mut self.query
        } else {
            &mut self.draft
        };
        let remaining = 500usize.saturating_sub(target.chars().count());
        target.extend(text.chars().filter(|c| !c.is_control()).take(remaining));
    }
    fn card(&self, c: &model::Card, cx: &Context<Self>) -> AnyElement {
        let session = &self.sessions[c.id];
        let z = self.layout.camera.zoom();
        let p = if self.focused_space.is_some() {
            v(
                self.viewport.x * 0.5 + (c.id as f32 - self.carousel) * (380. * z + 40.)
                    - c.size().x * z * 0.5,
                90. + ((self.viewport.y - 205. - c.size().y * z) * 0.5).max(0.),
            )
        } else {
            self.layout.camera.screen(c.pos)
        };
        let sz = c.size() * z;
        let selected = self.layout.selected == Some(c.id);
        let summary = self.focused_space.is_none() && z < 1.05;
        let mut card = div()
            .id(("session", c.id))
            .absolute()
            .left(px(p.x))
            .top(px(p.y))
            .w(px(sz.x))
            .h(px(sz.y))
            .rounded(px(30. * z.max(0.8)))
            .bg(rgb(0x04090c))
            .border_1()
            .border_color(rgb(if selected { 0x547078 } else { 0x17282f }))
            .shadow_lg()
            .overflow_hidden()
            .flex()
            .flex_col()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener({
                    let id = c.id;
                    move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.focus_session(id, true);
                        this.composing = false;
                        cx.notify();
                    }
                }),
            );
        let id = c.id;
        let header = div()
            .id(("header", id))
            .flex()
            .items_center()
            .gap(px(12. * z))
            .p(px(20. * z))
            .cursor(if c.pinned {
                CursorStyle::Arrow
            } else {
                CursorStyle::OpenHand
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, e: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    if this.screen != View::Canvas {
                        return;
                    }
                    this.sample_motion();
                    this.transition = None;
                    this.layout.focus(id);
                    this.drag_moved = false;
                    this.composing = false;
                    if let Some(chrome) = &this.chrome {
                        chrome.focus_canvas();
                    }
                    if this.focused_space.is_some() {
                        this.focus_session(id, true);
                        cx.notify();
                        return;
                    }
                    if let Some(c) = this.layout.card(id) {
                        this.gesture = Some(Gesture::Card {
                            id,
                            start: this.local(e.position),
                            origin: c.pos,
                        });
                    }
                    if e.click_count == 2 {
                        this.gesture = None;
                        this.focus_session(id, true);
                    }
                    cx.notify();
                }),
            )
            .child(avatar(session.id, if summary { 0.85 } else { z }))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(
                        label(
                            session.name.clone(),
                            if summary { 13. } else { 16. * z },
                            INK,
                        )
                        .truncate(),
                    )
                    .when(!summary, |d| {
                        d.child(label(session.identity, 10.5 * z, MUTED).truncate())
                    }),
            )
            .when(!summary, |d| {
                d.child(label(
                    if c.pinned { "PIN" } else { "•••" },
                    10. * z,
                    session.color,
                ))
            });
        card = card.child(header);
        if summary {
            let mut body = div()
                .px(px(20. * z))
                .pb(px(20. * z))
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(12. * z))
                .child(label(
                    format!("•••  {} · mock", session.status),
                    10.,
                    session.color,
                ));
            if id == 2 && z >= 0.62 {
                body = body.child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(self.reader_preview(z)),
                );
            } else {
                body = body.child(div().flex_1());
            }
            body = body
                .child(
                    label(session.summary, 14., 0xb3c1cb).line_clamp(if z < 0.55 { 1 } else { 2 }),
                )
                .child(label(session.identity, 10., MUTED).truncate());
            card = card.child(body);
        } else {
            let mut body = div()
                .id(("body", id))
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .px(px(22. * z))
                .flex()
                .flex_col()
                .gap(px(13. * z))
                .child(
                    div().flex().child(
                        div()
                            .rounded_full()
                            .bg(rgb(0x172831))
                            .px(px(10. * z))
                            .py(px(5. * z))
                            .child(label(
                                format!("●  {} · Mock", session.status),
                                10.5 * z,
                                session.color,
                            )),
                    ),
                )
                .child(label(session.summary, 21. * z, INK).line_clamp(2));
            if c.expanded {
                body = body
                    .child(
                        div()
                            .ml(px(35. * z))
                            .p(px(12. * z))
                            .rounded(px(14. * z))
                            .bg(rgb(0x1d2631))
                            .child(label(session.prompt, 12. * z, 0xc5d0da).line_clamp(2)),
                    )
                    .child(label(session.response, 13. * z, 0xc8d4dc).line_clamp(5))
                    .child(
                        div().flex().flex_col().gap(px(7. * z)).children(
                            session
                                .activities
                                .iter()
                                .map(|a| label(format!("↳  {a}"), 10.5 * z, MUTED).line_clamp(1)),
                        ),
                    );
                if id == 0
                    && let Some(note) = self.notes.last()
                {
                    body = body
                        .child(label(format!("Local note: {note}"), 11. * z, MINT).line_clamp(2));
                }
            }
            card = card.child(body).child(
                div()
                    .px(px(16. * z))
                    .pb(px(13. * z))
                    .flex()
                    .flex_col()
                    .gap(px(6. * z))
                    .when(c.expanded, |d| {
                        d.child(self.button(
                            ("artifact", id),
                            session.artifact,
                            Command::Artifact(id),
                            cx,
                        ))
                    })
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .items_center()
                            .child(self.button(
                                ("expand", id),
                                if c.expanded {
                                    "Collapse ↙"
                                } else {
                                    "Expand ↗"
                                },
                                Command::Expand(id),
                                cx,
                            ))
                            .child(self.button(
                                ("pin", id),
                                if c.pinned { "Unpin" } else { "Pin" },
                                Command::Pin(id),
                                cx,
                            ))
                            .child(self.button(("focus", id), "Focus ⤢", Command::Focus(id), cx)),
                    ),
            );
        }
        if id == 2 && self.focused_space.is_none() {
            div()
                .absolute()
                .left(px(p.x))
                .top(px(p.y))
                .w(px(sz.x))
                .h(px(sz.y))
                .child(
                    div()
                        .absolute()
                        .left(px(-18.))
                        .top(px(36.))
                        .w(px(sz.x))
                        .h(px(sz.y))
                        .rounded(px(30.))
                        .bg(rgba(0x030a0ee8))
                        .border_1()
                        .border_color(rgb(0x1d3039)),
                )
                .child(card.left(px(0.)).top(px(0.)))
                .into_any_element()
        } else {
            card.into_any_element()
        }
    }
    fn reader_preview(&self, z: f32) -> impl IntoElement {
        div()
            .w(px(154. * z))
            .h(px(285. * z))
            .rounded(px(22. * z))
            .border_2()
            .border_color(rgb(0x60717b))
            .bg(rgb(0x0b1119))
            .p(px(6. * z))
            .flex()
            .flex_col()
            .gap(px(7. * z))
            .child(
                div()
                    .w(px(58. * z))
                    .h(px(8. * z))
                    .rounded_full()
                    .bg(rgb(0x010407))
                    .mx_auto(),
            )
            .child(
                div()
                    .flex_1()
                    .relative()
                    .rounded(px(15. * z))
                    .overflow_hidden()
                    .child(img(art::scene()).size_full().object_fit(ObjectFit::Cover))
                    .child(
                        label("THE QUIET HOURS", 12. * z, INK)
                            .absolute()
                            .left(px(10. * z))
                            .bottom(px(20. * z)),
                    ),
            )
            .child(label("Continue reading", 10. * z, INK).px(px(7. * z)))
            .child(
                label("Mock reader preview", 8. * z, MUTED)
                    .px(px(7. * z))
                    .pb(px(7. * z)),
            )
    }
    fn minimap(&self, cx: &Context<Self>) -> impl IntoElement {
        let camera = self.layout.camera;
        let viewport = self.viewport;
        let mut map = div()
            .id("minimap")
            .absolute()
            .right_4()
            .bottom(px(112.))
            .w(px(174.))
            .h(px(112.))
            .rounded_xl()
            .bg(rgb(0x0c1720))
            .border_1()
            .border_color(rgb(0x2a3c49))
            .overflow_hidden()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _, _, cx| cx.stop_propagation()),
            )
            .child(label("WORKSPACE", 8., MUTED).absolute().left_3().top_2());
        // Map includes all cards and the viewport; useful even after a long pan.
        let a = camera.world(v(0., 0.));
        let b = camera.world(viewport);
        let lo = self
            .layout
            .cards
            .iter()
            .fold(v(a.x.min(0.), a.y.min(0.)), |a, c| {
                v(a.x.min(c.pos.x), a.y.min(c.pos.y))
            });
        let hi = self
            .layout
            .cards
            .iter()
            .fold(v(b.x.max(1400.), b.y.max(1050.)), |a, c| {
                v(a.x.max(c.pos.x + c.size().x), a.y.max(c.pos.y + c.size().y))
            });
        let scale = (148. / (hi.x - lo.x).max(1.)).min(75. / (hi.y - lo.y).max(1.));
        for c in &self.layout.cards {
            let p = (c.pos - lo) * scale + v(12., 25.);
            let s = c.size() * scale;
            let id = c.id;
            map = map.child(
                div()
                    .id(("mini", id))
                    .absolute()
                    .left(px(p.x))
                    .top(px(p.y))
                    .w(px(s.x))
                    .h(px(s.y))
                    .rounded(px(2.))
                    .bg(rgb(if self.layout.selected == Some(id) {
                        self.sessions[id].color
                    } else {
                        0x374653
                    }))
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.run(Command::Focus(id), true, cx);
                        }),
                    ),
            );
        }
        let p = (a - lo) * scale + v(12., 25.);
        let s = viewport * (scale / camera.zoom());
        // View outline is drawn before hits can be occluded by a covering div.
        map.child(
            canvas(
                |_, _, _| (),
                move |bounds, _, w, _| {
                    w.paint_quad(outline(
                        Bounds::new(
                            bounds.origin + point(px(p.x), px(p.y)),
                            size(px(s.x), px(s.y)),
                        ),
                        rgb(0x00bfae),
                        BorderStyle::Solid,
                    ));
                },
            )
            .absolute()
            .size_full(),
        )
    }
    fn modal(&self, cx: &Context<Self>) -> AnyElement {
        let mut panel = div()
            .id("modal-panel")
            .w(px(560.))
            .max_w_full()
            .mt(px(82.))
            .rounded_2xl()
            .border_1()
            .border_color(rgb(0x405261))
            .bg(rgb(0x101c27))
            .shadow_lg()
            .p_4()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _, _, cx| cx.stop_propagation()),
            );
        if self.palette {
            let results = self.results();
            panel = panel
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .mb_3()
                        .child(label(
                            if self.commands {
                                "Commands"
                            } else {
                                "Find a session"
                            },
                            18.,
                            INK,
                        ))
                        .child(label("ESC to close", 11., MUTED)),
                )
                .child(
                    div()
                        .rounded_lg()
                        .bg(rgb(0x071019))
                        .border_1()
                        .border_color(rgb(0x547b8c))
                        .p_3()
                        .child(if let Some(chrome) = &self.chrome {
                            native_chrome::host(chrome.clone(), native_chrome::Island::PaletteInput)
                                .w_full()
                                .h(px(24.))
                                .into_any_element()
                        } else {
                            label(self.query.clone(), 15., INK).into_any_element()
                        }),
                )
                .child(
                    label("↑ ↓ to navigate   ↵ to open   ⌘⇧K for commands", 10., MUTED)
                        .mt_3()
                        .mb_2(),
                );
            if results.is_empty() {
                panel = panel
                    .child(label("No matches. Try a name, identity, or topic.", 13., MUTED).py_5());
            }
            for (i, (name, action)) in results.into_iter().enumerate() {
                panel = panel.child(
                    div()
                        .id(("result", i))
                        .p_3()
                        .rounded_lg()
                        .bg(rgb(if self.result_index == i {
                            0x254348
                        } else {
                            0x101c27
                        }))
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(0x263747)))
                        .text_color(rgb(INK))
                        .text_size(px(14.))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.palette = false;
                                this.run(action, true, cx);
                            }),
                        )
                        .child(name),
                );
            }
        } else if let Some(id) = self.artifact {
            let s = &self.sessions[id];
            panel=panel.child(label(s.artifact,20.,INK)).child(label("MOCK ARTIFACT · Static fixture preview",11.,MINT).mt_2())
                .child(label(s.response,15.,INK).my_5())
                .children(s.activities.iter().map(|a|label(*a,13.,MUTED).mb_3()))
                .child(label("No file was fetched, generated, or executed. Escape or click outside to close.",12.,MUTED).mt_4());
        }
        div()
            .id("modal-backdrop")
            .absolute()
            .size_full()
            .bg(rgba(0x02070bbd))
            .flex()
            .justify_center()
            .items_start()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    cx.stop_propagation();
                    this.palette = false;
                    this.artifact = None;
                    cx.notify();
                }),
            )
            .child(panel)
            .into_any_element()
    }
}
impl Render for Spatial {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let started = Instant::now();
        self.sample_motion();
        let window_size = window.viewport_size();
        let width: f32 = window_size.width.into();
        let height: f32 = window_size.height.into();
        self.viewport = v((width - EDGE * 2.).max(1.), (height - TOP).max(1.));
        if self.first_render {
            self.first_render = false;
        }
        if self.metrics.benchmark && self.metrics.frames < 240 {
            if self.metrics.frames == 0 {
                self.screen = View::Canvas;
                self.sidebar = false;
                self.layout.camera = self.layout.overview(self.viewport);
            }
            let t = self.metrics.frames as f32 / 240.;
            self.layout
                .camera
                .pan_by(v((t * 12.).sin() * 0.9, (t * 10.).cos() * 0.5));
            if let Some(last) = self.metrics.last {
                self.metrics
                    .intervals
                    .push(started.duration_since(last).as_secs_f64() * 1000.);
            }
            self.metrics.last = Some(started);
            self.metrics.frames += 1;
        }
        if self.transition.is_some() || (self.metrics.benchmark && self.metrics.frames < 240) {
            window.request_animation_frame();
        }
        let camera = self.layout.camera;
        let mut viewport = div()
            .id("canvas")
            .absolute()
            .left(px(EDGE))
            .top(px(TOP))
            .w(px(self.viewport.x))
            .h(px(self.viewport.y))
            .rounded(px(42.))
            .border_1()
            .border_color(rgb(0x263b4a))
            .bg(rgb(0x071621))
            .overflow_hidden()
            .cursor_grab()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, e: &MouseDownEvent, _, cx| {
                    if this.screen != View::Canvas {
                        return;
                    }
                    this.sample_motion();
                    this.transition = None;
                    this.composing = false;
                    this.drag_moved = false;
                    this.layout.selected = None;
                    this.focused_space = None;
                    if let Some(chrome) = &this.chrome {
                        chrome.focus_canvas();
                    }
                    this.gesture = Some(Gesture::Pan {
                        start: this.local(e.position),
                        camera: this.layout.camera,
                    });
                    cx.notify();
                }),
            )
            .child(
                img(art::scene())
                    .absolute()
                    .size_full()
                    .object_fit(ObjectFit::Cover),
            )
            .children(
                self.layout
                    .cards
                    .iter()
                    .filter(|_| self.screen == View::Canvas)
                    .map(|c| self.card(c, cx)),
            );
        if self.screen == View::Canvas && self.layout.cards.is_empty() {
            viewport = viewport.child(
                div()
                    .absolute()
                    .left(px((self.viewport.x - 330.) / 2.))
                    .top(px(self.viewport.y / 2. - 70.))
                    .w(px(330.))
                    .child(label("A little room for something new.", 22., INK))
                    .child(label("This is the empty layout fixture.", 13., MUTED).my_3())
                    .child(self.button(
                        "restore-empty",
                        "Restore mock sessions ↗",
                        Command::Reset,
                        cx,
                    )),
            );
        }
        if self.screen == View::Canvas && self.focused_space.is_none() {
            viewport = viewport.child(self.minimap(cx));
        }
        if self.screen == View::Canvas {
            viewport = viewport.child(
                div()
                    .absolute()
                    .left_5()
                    .bottom(px(26.))
                    .flex()
                    .items_center()
                    .gap_1()
                    .rounded_xl()
                    .bg(rgba(0x07131bb0))
                    .child(self.button("zoom-out", "−", Command::Zoom(1. / 1.15), cx))
                    .child(
                        label(format!("{:03.0}%", camera.zoom() * 100.), 11., INK)
                            .w(px(38.))
                            .text_center(),
                    )
                    .child(self.button("zoom-in", "+", Command::Zoom(1.15), cx))
                    .child(self.button("fit", "Fit all  0", Command::Overview, cx)),
            );
        }
        let mut root = div()
            .id("spatial-root")
            .track_focus(&self.focus)
            .size_full()
            .bg(rgb(0x101923))
            .font_family("Helvetica Neue")
            .on_key_down(cx.listener(Self::key))
            .on_scroll_wheel(cx.listener(|this, e: &ScrollWheelEvent, _, cx| {
                if this.gesture.is_some() {
                    return;
                }
                let d = pv(e.delta.pixel_delta(px(24.)));
                if this.screen != View::Canvas {
                    if matches!(e.touch_phase, TouchPhase::Started)
                        || this
                            .swipe_last
                            .is_none_or(|last| last.elapsed().as_millis() > 240)
                    {
                        this.swipe.reset();
                    }
                    this.swipe_last = Some(Instant::now());
                    if let Some(backwards) = this.swipe.push(d.x, d.y) {
                        this.bot = home::adjacent(this.bot, backwards);
                        if this.screen == View::Folders {
                            this.folder_hover = Some(this.bot);
                        }
                        cx.notify();
                    }
                    return;
                }
                this.sample_motion();
                if e.modifiers.platform || e.modifiers.control {
                    this.transition = None;
                    this.focused_space = None;
                    this.layout
                        .camera
                        .zoom_at(this.local(e.position), (-d.y * 0.006).exp());
                } else {
                    if this.focused_space.is_some() {
                        if d.x.abs() > d.y.abs()
                            && this
                                .last_switch
                                .is_none_or(|t| t.elapsed().as_millis() > 260)
                        {
                            this.horizontal += d.x;
                            if this.horizontal.abs() > 65. {
                                let backwards = this.horizontal > 0.;
                                if let Some(id) = this.layout.cycle(backwards) {
                                    this.focus_session(id, true);
                                    this.last_switch = Some(Instant::now());
                                }
                            }
                        }
                    } else {
                        this.transition = None;
                        this.layout.camera.pan_by(d);
                    }
                }
                cx.notify();
            }))
            .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, _, cx| {
                if let Some(g) = this.gesture {
                    if e.pressed_button == Some(MouseButton::Left) {
                        let p = this.local(e.position);
                        let start = match g {
                            Gesture::Pan { start, .. } | Gesture::Card { start, .. } => start,
                        };
                        if (p.x - start.x).abs() + (p.y - start.y).abs() > 4. {
                            this.drag_moved = true;
                            this.focused_space = None;
                            this.layout.drag(g, p);
                        }
                        cx.notify();
                    } else {
                        this.gesture = None;
                    }
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if !this.drag_moved
                        && let Some(Gesture::Card { id, .. }) = this.gesture
                    {
                        this.focus_session(id, true);
                        cx.notify();
                    }
                    this.gesture = None;
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, _| {
                    this.gesture = None;
                }),
            )
            .child(viewport)
            .child(
                label("ASTRID  /  MOCK", 10., 0xa2b7c6)
                    .absolute()
                    .left(px(if self.sidebar { 300. } else { 180. }))
                    .top(px(34.)),
            );
        if self.screen == View::Home {
            root = root.child(self.home_surface(width, height, cx));
        }
        if self.screen == View::Folders {
            root = root.child(self.folder_surface(width, height, window, cx));
        }
        if self.sidebar {
            root = root.child(self.sidebar_surface(height, cx));
        }
        if let Some(chrome) = &self.chrome {
            chrome.sync_overlay(self.palette, self.artifact.is_some());
            chrome.sync_focus(self.screen == View::Home);
            let inset = if self.sidebar { 280. } else { 0. };
            let content_width = width - inset;
            let composer_width = (content_width - 140.).clamp(300., 700.);
            let composer_left = inset + (content_width - composer_width) * 0.5;
            let composer_bottom = if self.screen == View::Home {
                height * 0.36 - 58.
            } else {
                32.
            };
            root = root
                .child(
                    native_chrome::host(chrome.clone(), native_chrome::Island::Composer)
                        .absolute()
                        .left(px(composer_left))
                        .bottom(px(composer_bottom))
                        .w(px(composer_width))
                        .h(px(58.)),
                )
                .child(
                    native_chrome::host(chrome.clone(), native_chrome::Island::Add)
                        .absolute()
                        .left(px(composer_left - 58.))
                        .bottom(px(composer_bottom + 4.))
                        .size(px(48.)),
                )
                .child(
                    native_chrome::host(chrome.clone(), native_chrome::Island::Mode)
                        .absolute()
                        .right(px(86.))
                        .top(px(18.))
                        .w(px(90.))
                        .h(px(44.)),
                )
                .child(
                    native_chrome::host(chrome.clone(), native_chrome::Island::Search)
                        .absolute()
                        .left(px(if self.sidebar {
                            222.
                        } else {
                            width * 0.5 + 132.
                        }))
                        .top(px(if self.sidebar { 88. } else { 18. }))
                        .size(px(42.)),
                )
                .child(
                    native_chrome::host(chrome.clone(), native_chrome::Island::Settings)
                        .absolute()
                        .right(px(24.))
                        .top(px(18.))
                        .size(px(44.)),
                )
                .child(
                    native_chrome::host(chrome.clone(), native_chrome::Island::Sidebar)
                        .absolute()
                        .left(px(if self.sidebar { 222. } else { 114. }))
                        .top(px(18.))
                        .size(px(44.)),
                );
        }
        if self.palette || self.artifact.is_some() {
            root = root.child(self.modal(cx));
        }
        if self.metrics.benchmark && self.metrics.frames <= 240 {
            self.metrics
                .render_us
                .push(started.elapsed().as_secs_f64() * 1e6);
            if self.metrics.frames == 240 {
                self.metrics.frames += 1;
                print_metrics("frame interval ms", &self.metrics.intervals);
                print_metrics("render construction us", &self.metrics.render_us);
                println!(
                    "BENCHMARK_COMPLETE 240 frames; stopped requesting frames; five virtual cards; CPU callback timings; not GPU timestamps"
                );
            }
        }
        root
    }
}
fn print_metrics(name: &str, values: &[f64]) {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    if !sorted.is_empty() {
        println!(
            "{name}: n={} median={:.3} p95={:.3} max={:.3}",
            sorted.len(),
            sorted[sorted.len() / 2],
            sorted[(sorted.len() * 95 / 100).min(sorted.len() - 1)],
            sorted[sorted.len() - 1]
        );
    }
}
fn main() {
    Application::new()
        .with_assets(icons::Icons)
        .run(|cx: &mut App| {
            let bounds = Bounds::centered(None, size(px(1440.), px(940.)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(760.), px(560.))),
                    titlebar: Some(TitlebarOptions {
                        title: Some("Astrid — Spatial · Mock prototype".into()),
                        appears_transparent: true,
                        traffic_light_position: Some(point(px(20.), px(34.))),
                    }),
                    ..Default::default()
                },
                |window, cx| {
                    let view = cx.new(|cx| {
                        let state = Spatial::new(cx);
                        window.focus(&state.focus);
                        state
                    });
                    let (native_tx, native_rx) = async_channel::bounded(32);
                    match native_chrome::NativeChrome::new(window, cx, native_tx) {
                        Ok(chrome) => {
                            eprintln!(
                                "Native chrome: {}",
                                if chrome.liquid_glass {
                                    "NSGlassEffectView"
                                } else {
                                    "NSVisualEffectView fallback"
                                }
                            );
                            view.update(cx, |this, cx| {
                                chrome.set_reduced_motion(this.reduced);
                                this.chrome = Some(chrome);
                                cx.notify();
                            });
                        }
                        Err(error) => eprintln!("Native chrome unavailable: {error}"),
                    }
                    view.update(cx, |_, cx| {
                        cx.spawn_in(window, async move |view, cx| {
                            while let Ok(action) = native_rx.recv().await {
                                let _ = cx.update(|window, cx| {
                                    view.update(cx, |this, cx| {
                                        window.focus(&this.focus);
                                        match action {
                                            native_chrome::NativeAction::Sidebar => {
                                                this.run(Command::Sidebar, false, cx)
                                            }
                                            native_chrome::NativeAction::Search => {
                                                this.run(Command::Search, false, cx)
                                            }
                                            native_chrome::NativeAction::FocusPrimary => {
                                                this.run(Command::Home, true, cx)
                                            }
                                            native_chrome::NativeAction::Overview => {
                                                this.run(Command::Folders, true, cx)
                                            }
                                            native_chrome::NativeAction::Palette => {
                                                this.run(Command::Palette, false, cx)
                                            }
                                            native_chrome::NativeAction::QueryChanged => {
                                                if this.palette {
                                                    if let Some(chrome) = &this.chrome {
                                                        this.query = chrome.query();
                                                        this.result_index = 0;
                                                    }
                                                    cx.notify();
                                                }
                                            }
                                            native_chrome::NativeAction::Navigate(backwards) => {
                                                this.result_index = move_selection(
                                                    this.result_index,
                                                    this.results().len(),
                                                    backwards,
                                                );
                                                cx.notify();
                                            }
                                            native_chrome::NativeAction::Accept => {
                                                if let Some((_, command)) =
                                                    this.results().get(this.result_index).cloned()
                                                {
                                                    this.palette = false;
                                                    this.run(command, false, cx);
                                                }
                                            }
                                            native_chrome::NativeAction::Dismiss => {
                                                this.palette = false;
                                                cx.notify();
                                            }
                                            native_chrome::NativeAction::Note(text) => {
                                                this.draft = text;
                                                this.run(Command::Send, false, cx);
                                            }
                                        }
                                    })
                                });
                            }
                        })
                        .detach()
                    });
                    let (tx, rx) = async_channel::bounded(64);
                    if let Err(error) = magnify::install(window, tx) {
                        eprintln!("Trackpad magnification unavailable: {error}");
                    }
                    view.update(cx, |_, cx| {
                        cx.spawn_in(window, async move |view, cx| {
                            while let Ok(delta) = rx.recv().await {
                                let _ = cx.update(|window, cx| {
                                    view.update(cx, |this, cx| {
                                        if !this.palette
                                            && this.artifact.is_none()
                                            && this.gesture.is_none()
                                        {
                                            if this.screen != View::Canvas {
                                                this.screen = View::Canvas;
                                            }
                                            this.sample_motion();
                                            this.transition = None;
                                            this.focused_space = None;
                                            let anchor = this.local(window.mouse_position());
                                            this.layout
                                                .camera
                                                .zoom_at(anchor, magnify::factor(delta));
                                            cx.notify();
                                        }
                                    })
                                });
                            }
                        })
                        .detach()
                    });
                    view
                },
            )
            .expect("open GPUI window");
            cx.on_window_closed(|cx| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            cx.activate(true);
        });
}
