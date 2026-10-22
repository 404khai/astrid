//! Pure presentation state. Deliberately has no dependency on Astrid or GPUI.
use std::ops::{Add, Mul, Sub};

pub const WORLD_LIMIT: f32 = 20_000.;
/// Solve x(t) then evaluate y(t) for cubic-bezier(0.23, 1, 0.32, 1).
pub fn ease_out(progress: f32) -> f32 {
    if progress <= 0. {
        return 0.;
    }
    if progress >= 1. {
        return 1.;
    }
    let (mut lo, mut hi) = (0., 1.);
    for _ in 0..18 {
        let t = (lo + hi) * 0.5;
        let u = 1. - t;
        let x = 3. * u * u * t * 0.23 + 3. * u * t * t * 0.32 + t * t * t;
        if x < progress {
            lo = t;
        } else {
            hi = t;
        }
    }
    let t = (lo + hi) * 0.5;
    1. - (1. - t).powi(3)
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V {
    pub x: f32,
    pub y: f32,
}
pub const fn v(x: f32, y: f32) -> V {
    V { x, y }
}
impl Add for V {
    type Output = Self;
    fn add(self, b: Self) -> Self {
        v(self.x + b.x, self.y + b.y)
    }
}
impl Sub for V {
    type Output = Self;
    fn sub(self, b: Self) -> Self {
        v(self.x - b.x, self.y - b.y)
    }
}
impl Mul<f32> for V {
    type Output = Self;
    fn mul(self, b: f32) -> Self {
        v(self.x * b, self.y * b)
    }
}
impl V {
    pub fn finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
    pub fn bounded(self) -> Self {
        v(
            self.x.clamp(-WORLD_LIMIT, WORLD_LIMIT),
            self.y.clamp(-WORLD_LIMIT, WORLD_LIMIT),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub pan: V,
    zoom: f32,
    min: f32,
    max: f32,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            pan: v(60., 60.),
            zoom: 0.8,
            min: 0.3,
            max: 1.65,
        }
    }
}
impl Camera {
    pub fn zoom(self) -> f32 {
        self.zoom
    }
    pub fn world(self, screen: V) -> V {
        (screen - self.pan) * (1. / self.zoom)
    }
    pub fn screen(self, world: V) -> V {
        world * self.zoom + self.pan
    }
    pub fn set_limits(&mut self, min: f32, max: f32) -> bool {
        if !min.is_finite() || !max.is_finite() || min < 0.1 || max > 4. || min > max {
            return false;
        }
        self.min = min;
        self.max = max;
        self.zoom = self.zoom.clamp(min, max);
        true
    }
    pub fn zoom_at(&mut self, screen: V, factor: f32) {
        if !screen.finite() || !factor.is_finite() || factor <= 0. {
            return;
        }
        let world = self.world(screen);
        self.zoom = (self.zoom * factor).clamp(self.min, self.max);
        self.pan = (screen - world * self.zoom).bounded();
    }
    pub fn pan_by(&mut self, delta: V) {
        if delta.finite() {
            self.pan = (self.pan + delta).bounded();
        }
    }
    pub fn frame(&self, origin: V, extent: V, viewport: V) -> Self {
        if !origin.finite() || !extent.finite() || !viewport.finite() {
            return *self;
        }
        let zoom = ((viewport.x - 100.).max(1.) / extent.x.max(1.))
            .min((viewport.y - 120.).max(1.) / extent.y.max(1.))
            .clamp(self.min, self.max);
        Self {
            pan: (viewport * 0.5 - (origin + extent * 0.5) * zoom).bounded(),
            zoom,
            ..*self
        }
    }
    pub fn interpolate(self, to: Self, t: f32) -> Self {
        Self {
            pan: self.pan + (to.pan - self.pan) * t,
            zoom: self.zoom + (to.zoom - self.zoom) * t,
            ..self
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Card {
    pub id: usize,
    pub pos: V,
    pub expanded: bool,
    pub pinned: bool,
}
impl Card {
    pub fn size(&self) -> V {
        v(380., if self.expanded { 520. } else { 330. })
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Gesture {
    Pan { start: V, camera: Camera },
    Card { id: usize, start: V, origin: V },
}
#[derive(Clone, Debug)]
pub struct Layout {
    pub cards: Vec<Card>,
    pub selected: Option<usize>,
    pub camera: Camera,
}
impl Default for Layout {
    fn default() -> Self {
        Self {
            cards: [
                v(540., 100.),
                v(1040., 205.),
                v(40., 280.),
                v(540., 730.),
                v(40., 920.),
            ]
            .into_iter()
            .enumerate()
            .map(|(id, pos)| Card {
                id,
                pos,
                expanded: id != 4,
                pinned: false,
            })
            .collect(),
            selected: Some(0),
            camera: Camera::default(),
        }
    }
}
impl Layout {
    pub fn card(&self, id: usize) -> Option<&Card> {
        self.cards.iter().find(|c| c.id == id)
    }
    pub fn card_mut(&mut self, id: usize) -> Option<&mut Card> {
        self.cards.iter_mut().find(|c| c.id == id)
    }
    pub fn focus(&mut self, id: usize) {
        if let Some(i) = self.cards.iter().position(|c| c.id == id) {
            let c = self.cards.remove(i);
            self.cards.push(c);
            self.selected = Some(id);
        }
    }
    pub fn overview(&self, viewport: V) -> Camera {
        if self.cards.is_empty() {
            return Camera::default();
        }
        let lo = self.cards.iter().fold(v(f32::MAX, f32::MAX), |a, c| {
            v(a.x.min(c.pos.x), a.y.min(c.pos.y))
        });
        let hi = self.cards.iter().fold(v(f32::MIN, f32::MIN), |a, c| {
            v(a.x.max(c.pos.x + c.size().x), a.y.max(c.pos.y + c.size().y))
        });
        self.camera.frame(lo, hi - lo, viewport)
    }
    pub fn drag(&mut self, gesture: Gesture, pointer: V) {
        if !pointer.finite() {
            return;
        }
        match gesture {
            Gesture::Pan { start, camera } => {
                self.camera = camera;
                self.camera.pan_by(pointer - start);
            }
            Gesture::Card { id, start, origin } => {
                let pos = (origin + (pointer - start) * (1. / self.camera.zoom())).bounded();
                if let Some(c) = self.card_mut(id)
                    && !c.pinned
                {
                    c.pos = pos;
                }
            }
        }
    }
    pub fn reset(&mut self) -> Self {
        std::mem::take(self)
    }
    pub fn cycle(&mut self, backwards: bool) -> Option<usize> {
        let mut ids: Vec<_> = self.cards.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        if ids.is_empty() {
            self.selected = None;
            return None;
        }
        let index = ids
            .iter()
            .position(|id| Some(*id) == self.selected)
            .unwrap_or(0);
        let next = (index + if backwards { ids.len() - 1 } else { 1 }) % ids.len();
        self.focus(ids[next]);
        Some(ids[next])
    }
}

/// Palette navigation is shared by keyboard and pointer activation.
pub fn move_selection(index: usize, count: usize, backwards: bool) -> usize {
    if count == 0 {
        0
    } else {
        (index.min(count - 1) + if backwards { count - 1 } else { 1 }) % count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn near(a: V, b: V) {
        assert!(
            (a.x - b.x).abs() < 0.02 && (a.y - b.y).abs() < 0.02,
            "{a:?} != {b:?}"
        );
    }
    #[test]
    fn coordinates_round_trip() {
        for z in [0.3, 0.8, 1.65] {
            let mut c = Camera::default();
            c.zoom_at(v(0., 0.), z / c.zoom());
            for p in [v(-500., 123.), v(0., 0.), v(1540., -2800.)] {
                near(c.world(c.screen(p)), p);
                near(c.screen(c.world(p)), p);
            }
        }
    }
    #[test]
    fn zoom_keeps_pointer_world_point() {
        let mut c = Camera::default();
        let p = v(372., 281.);
        let w = c.world(p);
        for factor in [1.4, 0.3, 1000., 0.0001] {
            c.zoom_at(p, factor);
            near(c.world(p), w);
        }
    }
    #[test]
    fn bounds_and_invalid_input_are_stable() {
        let mut c = Camera::default();
        let original = c;
        for f in [f32::NAN, f32::INFINITY, -1., 0.] {
            c.zoom_at(v(2., 2.), f);
            assert_eq!(c, original);
        }
        c.pan_by(v(f32::NAN, 2.));
        assert_eq!(c, original);
        assert!(!c.set_limits(0., 2.));
        assert!(!c.set_limits(2., 1.));
        assert!(c.set_limits(0.4, 1.2));
        for _ in 0..10000 {
            c.pan_by(v(100., -100.));
            c.zoom_at(v(300., 200.), 1.02);
        }
        assert!(c.pan.finite());
        assert!(c.pan.x.abs() <= WORLD_LIMIT);
        assert!(c.zoom() <= 1.2);
    }
    #[test]
    fn drag_uses_world_units_and_pins() {
        let mut l = Layout::default();
        let origin = l.card(1).unwrap().pos;
        let g = Gesture::Card {
            id: 1,
            start: v(10., 20.),
            origin,
        };
        l.drag(g, v(90., 100.));
        near(l.card(1).unwrap().pos, origin + v(100., 100.));
        l.card_mut(1).unwrap().pinned = true;
        l.drag(g, v(300., 400.));
        near(l.card(1).unwrap().pos, origin + v(100., 100.));
    }
    #[test]
    fn pan_does_not_drag_cards() {
        let mut l = Layout::default();
        let cards = l.cards.clone();
        let c = l.camera;
        l.drag(
            Gesture::Pan {
                start: v(4., 4.),
                camera: c,
            },
            v(24., 34.),
        );
        assert_eq!(cards, l.cards);
        near(l.camera.pan, c.pan + v(20., 30.));
    }
    #[test]
    fn focus_is_unique_and_preserves_positions() {
        let mut l = Layout::default();
        let pos = l.card(2).unwrap().pos;
        l.focus(2);
        l.focus(2);
        assert_eq!(l.cards.last().unwrap().id, 2);
        assert_eq!(l.cards.len(), 5);
        assert_eq!(l.selected, Some(2));
        assert_eq!(pos, l.card(2).unwrap().pos);
        l.focus(100);
        assert_eq!(l.selected, Some(2));
    }
    #[test]
    fn reset_is_recoverable() {
        let mut l = Layout::default();
        l.card_mut(0).unwrap().pos = v(900., 800.);
        l.card_mut(0).unwrap().pinned = true;
        let saved = l.reset();
        assert_eq!(l.cards, Layout::default().cards);
        l = saved;
        assert_eq!(l.card(0).unwrap().pos, v(900., 800.));
        assert!(l.card(0).unwrap().pinned);
    }
    #[test]
    fn fit_includes_all_cards_and_empty_is_safe() {
        let mut l = Layout::default();
        let viewport = v(1400., 900.);
        let c = l.overview(viewport);
        for card in &l.cards {
            let a = c.screen(card.pos);
            let b = c.screen(card.pos + card.size());
            assert!(a.x >= 0. && a.y >= 0. && b.x <= viewport.x && b.y <= viewport.y);
        }
        l.cards.clear();
        assert!(l.overview(v(0., 0.)).pan.finite());
        assert_eq!(l.cycle(false), None);
    }
    #[test]
    fn keyboard_selection_wraps() {
        let mut l = Layout::default();
        assert_eq!(l.cycle(true), Some(4));
        assert_eq!(l.cycle(false), Some(0));
        assert_eq!(move_selection(0, 5, true), 4);
        assert_eq!(move_selection(4, 5, false), 0);
        assert_eq!(move_selection(7, 0, false), 0);
    }
    #[test]
    fn interrupted_camera_starts_at_current_value() {
        let a = Camera::default();
        let b = a.frame(v(400., 200.), v(420., 460.), v(1100., 700.));
        let mid = a.interpolate(b, 0.4);
        assert_eq!(mid.interpolate(a, 0.), mid);
        assert_eq!(mid.interpolate(a, 1.), a);
    }
    #[test]
    fn easing_is_bounded_monotonic_and_exact_at_endpoints() {
        assert_eq!(ease_out(0.), 0.);
        assert_eq!(ease_out(1.), 1.);
        let mut prev = 0.;
        for i in 0..101 {
            let value = ease_out(i as f32 / 100.);
            assert!(value >= prev && value <= 1.);
            prev = value;
        }
        assert!(ease_out(0.2) > 0.5);
    }
}
