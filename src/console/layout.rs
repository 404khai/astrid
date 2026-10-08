use ratatui::layout::{Constraint, Layout, Rect};

pub(super) fn composer(area: Rect, menu_rows: usize) -> (Rect, Rect, Rect) {
    let [menu, editor, footer] = Layout::vertical([
        Constraint::Length(menu_rows.min(area.height.saturating_sub(3) as usize) as u16),
        Constraint::Min(1),
        Constraint::Length(2),
    ])
    .areas(area);
    (menu, editor, footer)
}
pub(super) fn execution(area: Rect) -> (Rect, Rect, Rect) {
    let [tail, status, input] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(area);
    (tail, status, input)
}
