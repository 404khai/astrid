mod fixtures;
mod icons;
mod magnify;
mod model;
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
    layout: Layout,
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
fn avatar(color: u32, s: f32) -> Div {
    div()
        .w(px(34. * s))
        .h(px(32. * s))
        .rounded(px(12. * s))
        .bg(rgb(color))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(5. * s))
        .child(
            div()
                .w(px(3.5 * s))
                .h(px(9. * s))
                .rounded_full()
                .bg(rgb(0x061217)),
        )
        .child(
            div()
                .w(px(3.5 * s))
                .h(px(9. * s))
                .rounded_full()
                .bg(rgb(0x061217)),
        )
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
            layout,
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
            Command::Search | Command::Palette => {
                self.palette = !self.palette;
                self.commands = matches!(cmd, Command::Palette);
                self.query.clear();
                self.result_index = 0;
                self.composing = false;
                self.artifact = None;
            }
            Command::Overview => {
                self.focused_space = None;
                let mut target = self.layout.overview(self.viewport - v(40., 180.));
                target.pan = target.pan + v(20., 80.);
                self.go(target, animated);
            }
            Command::Home => {
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
                    self.status = "Mock note added to Meister · No message was sent".into();
                    self.layout.focus(0);
                }
            }
            Command::Zoom(f) => {
                self.sample_motion();
                self.transition = None;
                self.layout.camera.zoom_at(self.viewport * 0.5, f);
            }
            Command::Empty => {
                self.focused_space = None;
                self.undo = Some(self.layout.clone());
                self.layout.cards.clear();
                self.layout.selected = None;
                self.status = "Empty layout fixture · Reset or Undo to recover".into();
            }
            Command::Dense => {
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
                ("Reset view · 100% home", Command::Home),
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
                self.viewport.x * 0.5 + (c.id as f32 - self.carousel) * (420. * z + 32.)
                    - c.size().x * z * 0.5,
                90. + ((self.viewport.y - 205. - c.size().y * z) * 0.5).max(0.),
            )
        } else {
            self.layout.camera.screen(c.pos)
        };
        let sz = c.size() * z;
        let selected = self.layout.selected == Some(c.id);
        let summary = z < 0.72;
        let mut card = div()
            .id(("session", c.id))
            .absolute()
            .left(px(p.x))
            .top(px(p.y))
            .w(px(sz.x))
            .h(px(sz.y))
            .rounded(px(38. * z.max(0.8)))
            .bg(rgb(0x080f14))
            .border_1()
            .border_color(rgb(if selected { 0x4b777c } else { 0x25343f }))
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
                    this.sample_motion();
                    this.transition = None;
                    this.layout.focus(id);
                    this.drag_moved = false;
                    this.composing = false;
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
            .child(avatar(session.color, if summary { 0.85 } else { z }))
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
            card = card.child(
                div()
                    .px(px(16.))
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(label(session.status, 11., session.color))
                    .child(label(session.summary, 12., INK).line_clamp(2))
                    .child(label("MOCK SESSION", 9., MUTED)),
            );
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
        card.into_any_element()
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
                        .child(label(
                            if self.query.is_empty() {
                                "Type to search…".into()
                            } else {
                                format!("{}│", self.query)
                            },
                            15.,
                            if self.query.is_empty() { MUTED } else { INK },
                        )),
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
            let mut target = self.layout.overview(self.viewport - v(40., 180.));
            target.pan = target.pan + v(20., 80.);
            self.layout.camera = target;
        }
        if self.metrics.benchmark && self.metrics.frames < 240 {
            if self.metrics.frames == 0 {
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
            .bg(rgb(0x081b27))
            .overflow_hidden()
            .cursor_grab()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, e: &MouseDownEvent, _, cx| {
                    this.sample_motion();
                    this.transition = None;
                    this.composing = false;
                    this.drag_moved = false;
                    this.layout.selected = None;
                    this.focused_space = None;
                    this.gesture = Some(Gesture::Pan {
                        start: this.local(e.position),
                        camera: this.layout.camera,
                    });
                    cx.notify();
                }),
            )
            .on_scroll_wheel(cx.listener(|this, e: &ScrollWheelEvent, _, cx| {
                if this.gesture.is_some() {
                    return;
                }
                this.sample_motion();
                let d = pv(e.delta.pixel_delta(px(24.)));
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
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        let step = (52. * camera.zoom()).max(24.);
                        let pan = camera.pan;
                        let start_x = pan.x.rem_euclid(step);
                        let start_y = pan.y.rem_euclid(step);
                        let width: f32 = bounds.size.width.into();
                        let height: f32 = bounds.size.height.into();
                        for ix in 0..(width / step).ceil() as usize {
                            for iy in 0..(height / step).ceil() as usize {
                                let p = bounds.origin
                                    + point(
                                        px(start_x + ix as f32 * step),
                                        px(start_y + iy as f32 * step),
                                    );
                                window.paint_quad(fill(
                                    Bounds::new(p, size(px(1.), px(1.))),
                                    rgb(0x24404c),
                                ));
                            }
                        }
                    },
                )
                .absolute()
                .size_full(),
            )
            .child(div().absolute().left_6().top(px(82.)).child(label(
                "YOUR WORK, WITH ROOM TO THINK",
                10.,
                0x65828e,
            )))
            .children(self.layout.cards.iter().map(|c| self.card(c, cx)));
        if self.layout.cards.is_empty() {
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
        viewport = viewport.child(self.minimap(cx)).child(
            div()
                .absolute()
                .left_5()
                .bottom_4()
                .flex()
                .items_center()
                .gap_1()
                .rounded_xl()
                .bg(rgb(0x101f2b))
                .child(self.button("zoom-out", "−", Command::Zoom(1. / 1.15), cx))
                .child(
                    label(format!("{:03.0}%", camera.zoom() * 100.), 11., INK)
                        .w(px(38.))
                        .text_center(),
                )
                .child(self.button("zoom-in", "+", Command::Zoom(1.15), cx))
                .child(self.button("fit", "Fit all  0", Command::Overview, cx)),
        );
        let composer_width = (width - 96.).min(720.);
        let composer = div()
            .id("composer")
            .absolute()
            .bottom(px(38.))
            .left(px((width - composer_width) / 2.))
            .w(px(composer_width))
            .h(px(54.))
            .rounded_full()
            .border_1()
            .border_color(rgb(if self.composing { 0x6aa0a8 } else { 0x374653 }))
            .bg(rgb(0x17232e))
            .flex()
            .items_center()
            .px_4()
            .gap_3()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    cx.stop_propagation();
                    this.composing = true;
                    window.focus(&this.focus);
                    cx.notify();
                }),
            )
            .child(avatar(MINT, 0.8))
            .child(
                label(
                    if self.draft.is_empty() {
                        "Leave Meister a mock note…".into()
                    } else {
                        format!("{}{}", self.draft, if self.composing { "│" } else { "" })
                    },
                    14.,
                    if self.draft.is_empty() { MUTED } else { INK },
                )
                .flex_1()
                .min_w_0()
                .truncate(),
            )
            .child(self.button("send", "Add note ↗", Command::Send, cx));
        let mut root = div()
            .id("spatial-root")
            .track_focus(&self.focus)
            .size_full()
            .bg(rgb(0x101923))
            .font_family("Helvetica Neue")
            .on_key_down(cx.listener(Self::key))
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
                div()
                    .absolute()
                    .top(px(20.))
                    .left(px(100.))
                    .right(px(20.))
                    .bg(rgba(0x091722eb))
                    .rounded_full()
                    .px_4()
                    .py_2()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        img(std::path::PathBuf::from(concat!(
                            env!("CARGO_MANIFEST_DIR"),
                            "/assets/astrid-logo.png"
                        )))
                        .w(px(40.))
                        .h(px(40.)),
                    )
                    .when(width > 1000., |d| {
                        d.child(div().child(label("Astrid", 20., INK)).child(label(
                            "Spatial workspace",
                            11.,
                            MUTED,
                        )))
                    })
                    .when(width > 1100., |d| {
                        d.child(
                            div()
                                .ml_3()
                                .rounded_full()
                                .border_1()
                                .border_color(rgb(0x34535b))
                                .px_3()
                                .py_1()
                                .child(label("DS–00  /  MOCK DATA", 10., MINT)),
                        )
                    })
                    .child(div().flex_1())
                    .child(self.button(
                        "search",
                        if width > 1000. {
                            "Find session  ⌘K"
                        } else {
                            "Search ⌘K"
                        },
                        Command::Search,
                        cx,
                    ))
                    .when(width > 1200., |d| {
                        d.child(self.button("commands", "Commands  ⌘⇧K", Command::Palette, cx))
                    })
                    .child(self.button("overview", "Overview", Command::Overview, cx))
                    .child(self.button("reset", "Reset", Command::Reset, cx))
                    .when(self.undo.is_some(), |d| {
                        d.child(self.button("undo", "Undo", Command::Undo, cx))
                    })
                    .child(self.button(
                        "motion",
                        if width < 1000. {
                            "Motion"
                        } else if self.reduced {
                            "Motion: reduced"
                        } else {
                            "Motion: full"
                        },
                        Command::Motion,
                        cx,
                    )),
            )
            .child(composer)
            .child(
                label(self.status.clone(), 10., MUTED)
                    .absolute()
                    .bottom(px(13.))
                    .left(px(30.))
                    .max_w(px(width - 60.))
                    .truncate(),
            )
            .when(width > 1100., |d| {
                d.child(
                    label(
                        "Pinch to zoom · Click a space to focus · Swipe horizontally to browse",
                        10.,
                        MUTED,
                    )
                    .absolute()
                    .right(px(28.))
                    .bottom(px(13.)),
                )
            });
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
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |window, cx| {
                    let view = cx.new(|cx| {
                        let state = Spatial::new(cx);
                        window.focus(&state.focus);
                        state
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
