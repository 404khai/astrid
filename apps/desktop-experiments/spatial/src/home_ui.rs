use super::*;
impl Spatial {
    pub(super) fn home_surface(&self, width: f32, height: f32, cx: &Context<Self>) -> AnyElement {
        let inset = if self.sidebar { 280. } else { 0. };
        let available = width - inset;
        div()
            .absolute()
            .left(px(inset))
            .top(px(height * 0.25))
            .w(px(available))
            .flex()
            .flex_col()
            .items_center()
            .gap_3()
            .child(img(art::logo()).size(px(80.)))
            .child(
                label(
                    "What can I take care of for you?",
                    if available < 650. { 25. } else { 34. },
                    INK,
                )
                .text_center(),
            )
            .child(
                label(
                    format!(
                        "{} · Your own space to think, explore, and begin.",
                        BOTS[self.bot]
                    ),
                    14.,
                    0xb0c2ce,
                )
                .text_center(),
            )
            .child(
                div()
                    .mt_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(avatar(self.bot, 0.8))
                    .child(label("Demo workspace · No agents connected", 11., 0x99aebb)),
            )
            .child(
                div()
                    .mt(px(height * 0.25))
                    .flex()
                    .gap_2()
                    .child(self.button("home-folders", "Browse spaces", Command::Folders, cx))
                    .child(self.button("home-canvas", "Spatial canvas", Command::Overview, cx)),
            )
            .child(label(
                "Swipe left or right to switch bots · ← →",
                11.,
                0x92a8b9,
            ))
            .into_any_element()
    }
    pub(super) fn sidebar_surface(&self, height: f32, cx: &Context<Self>) -> AnyElement {
        let mut sidebar = div()
            .id("bot-sidebar")
            .absolute()
            .left_0()
            .top_0()
            .w(px(280.))
            .h(px(height))
            .bg(rgba(0x09131dec))
            .border_r_1()
            .border_color(rgba(0x9ac6e01c))
            .pt(px(88.))
            .px_4()
            .flex()
            .flex_col()
            .gap_2()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _, _, cx| cx.stop_propagation()),
            )
            .child(self.button("sidebar-search", "Search   ⌘K", Command::Search, cx))
            .child(self.button("sidebar-open", "Open spaces", Command::Folders, cx))
            .child(self.button("sidebar-new", "New conversation", Command::Home, cx))
            .child(
                div()
                    .mt_5()
                    .mb_2()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(avatar(self.bot, 1.1))
                    .child(label(BOTS[self.bot], 18., INK)),
            )
            .child(self.button(
                "sidebar-hub",
                format!("{}’s Hub", BOTS[self.bot]),
                Command::Folders,
                cx,
            ));
        for id in [self.bot, (self.bot + 1) % BOTS.len()] {
            let session = &self.sessions[id];
            sidebar = sidebar.child(
                div()
                    .id(("recent-session", id))
                    .mt_2()
                    .p_3()
                    .rounded_xl()
                    .bg(rgba(0x41536335))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgba(0x60768a50)))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.run(Command::Focus(id), true, cx);
                        }),
                    )
                    .child(label(session.name.clone(), 13., INK).truncate())
                    .child(label(session.summary, 11., MUTED).mt_2().line_clamp(2))
                    .child(label("Mock session", 9., 0x708b9f).mt_2()),
            );
        }
        sidebar
            .child(
                div()
                    .absolute()
                    .bottom(px(70.))
                    .left(px(16.))
                    .right(px(16.))
                    .flex()
                    .justify_between()
                    .children((0..BOTS.len()).map(|id| {
                        div()
                            .id(("bot", id))
                            .p_1()
                            .rounded_full()
                            .bg(if id == self.bot {
                                rgba(0x9bc9e33a)
                            } else {
                                rgba(0x00000000)
                            })
                            .cursor_pointer()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    this.run(Command::Bot(id), false, cx);
                                }),
                            )
                            .child(avatar(id, 0.9))
                    })),
            )
            .child(
                div()
                    .absolute()
                    .bottom(px(18.))
                    .left(px(16.))
                    .child(self.button(
                        "sidebar-settings",
                        "Settings · Mock controls",
                        Command::Palette,
                        cx,
                    )),
            )
            .into_any_element()
    }
    pub(super) fn folder_surface(
        &mut self,
        width: f32,
        height: f32,
        window: &mut Window,
        cx: &Context<Self>,
    ) -> AnyElement {
        let inset = if self.sidebar { 280. } else { 0. };
        let scale = ((width - inset - 80.) / 880.)
            .min((height - 240.) / 620.)
            .clamp(0.35, 1.);
        let stack_width = 880. * scale;
        let stack_top = ((height - 620. * scale) * 0.5).max(110.);
        let dt = self.folder_tick.elapsed().as_secs_f32().min(0.05);
        self.folder_tick = Instant::now();
        let mut moving = false;
        for id in 0..5 {
            let target = if self.folder_hover == Some(id) {
                44.
            } else {
                0.
            };
            let old = self.folder_lifts[id];
            self.folder_lifts[id] = home::lift(old, target, dt, self.reduced);
            moving |= (self.folder_lifts[id] - target).abs() > 0.2;
        }
        if moving {
            window.request_animation_frame();
        }
        let mut stack = div()
            .absolute()
            .left(px(inset + (width - inset - stack_width) * 0.5))
            .top(px(stack_top))
            .w(px(stack_width))
            .h(px(620. * scale));
        for (id, _) in BOTS.iter().enumerate() {
            let w = (600. + id as f32 * 70.) * scale;
            let y = id as f32 * 52. * scale;
            let lift = self.folder_lifts[id] * scale;
            // Keep hit rectangles stationary while artwork rises; this prevents
            // enter/leave oscillation when the pointer is on the folder lip.
            stack = stack.child(
                div()
                    .id(("folder", id))
                    .absolute()
                    .left(px((stack_width - w) * 0.5))
                    .top(px(y))
                    .w(px(w))
                    .h(px(390. * scale))
                    .cursor_pointer()
                    .on_hover(cx.listener(move |this, hovered, _, cx| {
                        this.folder_hover = if *hovered {
                            Some(id)
                        } else if this.folder_hover == Some(id) {
                            None
                        } else {
                            this.folder_hover
                        };
                        cx.notify();
                    }))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.bot = id;
                            this.run(Command::Focus(id), true, cx);
                        }),
                    )
                    .child(
                        img(art::folder(id))
                            .absolute()
                            .top(px(-lift))
                            .w(px(w))
                            .h(px(390. * scale)),
                    )
                    .child(
                        label(BOTS[id], 14. * scale, 0x17313b)
                            .absolute()
                            .left(px(w * 0.12))
                            .top(px(11. * scale - lift)),
                    )
                    .when(id == 4, |s| {
                        s.child(
                            div()
                                .absolute()
                                .top(px(150. * scale - lift))
                                .w_full()
                                .flex()
                                .flex_col()
                                .items_center()
                                .gap_3()
                                .child(avatar(id, 2. * scale))
                                .child(label(BOTS[id], 24. * scale, 0x43332e))
                                .child(label("Your spaces · Demo", 12. * scale, 0x775142)),
                        )
                    }),
            );
        }
        div()
            .absolute()
            .size_full()
            .child(
                label(format!("Your spaces / {}", BOTS[self.bot]), 24., INK)
                    .absolute()
                    .left(px(inset + 40.))
                    .top(px(92.)),
            )
            .child(
                label(
                    "Hover to leaf through · Click to open · ← → and Return",
                    11.,
                    0xa9bdcb,
                )
                .absolute()
                .left(px(inset + 40.))
                .top(px(126.)),
            )
            .child(stack)
            .into_any_element()
    }
}
