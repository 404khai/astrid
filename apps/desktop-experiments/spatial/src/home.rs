//! Presentation-only bot navigation. A gesture can select at most one bot.
#[derive(Clone, Copy, PartialEq)]
pub enum View {
    Home,
    Folders,
    Canvas,
}
pub const BOTS: [&str; 5] = ["Meister", "Shelly", "Nora", "Corbin", "Brief"];
#[derive(Default)]
pub struct Swipe {
    distance: f32,
    consumed: bool,
}
impl Swipe {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    pub fn push(&mut self, x: f32, y: f32) -> Option<bool> {
        if self.consumed || !x.is_finite() || !y.is_finite() || x.abs() <= y.abs() {
            return None;
        }
        self.distance += x;
        if self.distance.abs() < 65. {
            return None;
        }
        self.consumed = true;
        Some(self.distance > 0.)
    }
}
pub fn adjacent(bot: usize, backwards: bool) -> usize {
    (bot + if backwards { BOTS.len() - 1 } else { 1 }) % BOTS.len()
}
/// Exponential approach uses the current value, so reversing never jumps.
pub fn lift(current: f32, target: f32, dt: f32, reduced: bool) -> f32 {
    if reduced || (target - current).abs() < 0.2 {
        target
    } else {
        current + (target - current) * (1. - (-dt.clamp(0., 0.05) * 22.).exp())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn swipe_locks_until_next_gesture_and_ignores_vertical_motion() {
        let mut swipe = Swipe::default();
        assert_eq!(swipe.push(20., 80.), None);
        assert_eq!(swipe.push(-40., 1.), None);
        assert_eq!(swipe.push(-30., 1.), Some(false));
        assert_eq!(swipe.push(-1000., 0.), None);
        swipe.reset();
        assert_eq!(swipe.push(70., 0.), Some(true));
        assert_eq!(swipe.push(f32::NAN, 0.), None);
    }
    #[test]
    fn hover_reverses_from_current_position_and_reduced_motion_finishes() {
        let raised = lift(0., 44., 0.016, false);
        assert!(raised > 0. && raised < 44.);
        let reversed = lift(raised, 0., 0.016, false);
        assert!(reversed > 0. && reversed < raised);
        assert_eq!(lift(raised, 0., 0.016, true), 0.);
        assert_eq!(lift(0., 44., 0.016, true), 44.);
    }
    #[test]
    fn bots_wrap_without_affecting_layout() {
        assert_eq!(adjacent(0, true), 4);
        assert_eq!(adjacent(4, false), 0);
    }
}
