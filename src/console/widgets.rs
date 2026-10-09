use super::{
    format::truncate,
    input::Editor,
    layout,
    theme::{self, Ink},
};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::Line,
    widgets::Paragraph,
};

#[cfg(test)]
pub(super) fn composer(
    frame: &mut Frame,
    editor: &mut Editor,
    model: &str,
    notice: &str,
    color: bool,
) {
    composer_in_area(frame, frame.area(), editor, model, notice, color);
}

pub(super) fn composer_in_area(
    frame: &mut Frame,
    bounds: Rect,
    editor: &mut Editor,
    model: &str,
    notice: &str,
    color: bool,
) {
    let unbound = model.starts_with("unbound ·");
    let options = editor.options();
    editor.selected = editor.selected.min(options.len().saturating_sub(1));
    let menu_rows = if editor.menu() {
        options.len().min(6) + 1
    } else {
        0
    };
    let width = usize::from(bounds.width.saturating_sub(2)).max(1);
    let rows = editor
        .textarea
        .lines()
        .iter()
        .map(|line| {
            use unicode_width::UnicodeWidthStr;
            // Leave room for the cursor when the final cell is occupied.
            (line.width() + 1).div_ceil(width).max(1)
        })
        .sum::<usize>();
    let height = (menu_rows + rows.max(2) + 2).min(usize::from(bounds.height)) as u16;
    let bounds = Rect::new(bounds.x, bounds.bottom() - height, bounds.width, height);
    let (menu, area, footer) = layout::composer(bounds, menu_rows);
    frame.render_widget(Paragraph::new("").style(theme::composer_style(color)), menu);
    frame.render_widget(Paragraph::new("").style(theme::composer_style(color)), area);
    if menu.height > 0 {
        let mut lines = vec![Line::styled(
            if editor.model_menu() {
                editor
                    .choice_title
                    .clone()
                    .unwrap_or_else(|| "Switch model — type to filter".to_owned())
            } else {
                if editor.mention().is_some() {
                    editor
                        .lookup_error
                        .clone()
                        .unwrap_or_else(|| "Files — Enter/Tab insert".into())
                } else {
                    "Commands".to_owned()
                }
            },
            Ink::Dim.style(color),
        )];
        let start = editor
            .selected
            .saturating_sub(menu.height.saturating_sub(2) as usize);
        for (index, (name, detail)) in options
            .iter()
            .enumerate()
            .skip(start)
            .take(menu.height.saturating_sub(1) as usize)
        {
            lines.push(Line::styled(
                format!(
                    "{} {name}  {detail}",
                    if index == editor.selected { "▸" } else { " " }
                ),
                if index == editor.selected {
                    if unbound {
                        theme::composer_style(color).patch(Ink::Unbound.style(color))
                    } else {
                        theme::selection_style(color)
                    }
                } else {
                    theme::composer_style(color)
                },
            ));
        }
        if options.is_empty() {
            lines.push(Line::from("No matches"));
        }
        frame.render_widget(
            Paragraph::new(lines).style(theme::composer_style(color)),
            menu,
        );
    }
    editor.textarea.set_style(theme::composer_style(color));
    editor.textarea.set_placeholder_style(Ink::Dim.style(color));
    editor.textarea.set_cursor_line_style(Style::default());
    editor
        .textarea
        .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
    let editor_area = if area.width > 2 {
        frame.render_widget(
            Paragraph::new("❯").style(theme::title_ink(unbound).style(color)),
            Rect::new(area.x, area.y, 2, area.height),
        );
        Rect::new(area.x + 2, area.y, area.width - 2, area.height)
    } else {
        area
    };
    frame.render_widget(&editor.textarea, editor_area);
    let help = if let Some(notice) = editor.notice {
        format!("{model} · {notice}")
    } else if !notice.is_empty() {
        format!("{model} · {notice}")
    } else if editor.menu() {
        format!("{model} · ↑/↓ select · Enter confirm · Esc cancel")
    } else {
        format!("{model} · Enter sends · Ctrl-J newline · / commands")
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled("─".repeat(footer.width as usize), Ink::Dim.style(color)),
            theme::mode_line(truncate(&help, footer.width as usize), unbound, color),
        ])
        .style(Ink::Dim.style(color)),
        footer,
    );
}

pub(super) fn status(
    frame: &mut Frame,
    status: &str,
    answer: &str,
    waiting: bool,
    done: bool,
    color: bool,
) {
    let (_, area, input) = layout::execution(frame.area());
    frame.render_widget(
        Paragraph::new(theme::mode_line(
            truncate(status, area.width as usize),
            status.starts_with("unbound"),
            color,
        )),
        area,
    );
    let label = if waiting {
        format!("› type yes to approve: {answer}")
    } else if done {
        "› run ended".into()
    } else {
        "› executing · Ctrl-C cancel".into()
    };
    frame.render_widget(
        Paragraph::new(truncate(&label, input.width as usize)).style(if waiting {
            theme::title_ink(status.starts_with("unbound")).style(color)
        } else {
            Ink::Dim.style(color)
        }),
        input,
    );
    if waiting && input.width > 0 {
        use unicode_width::UnicodeWidthStr;
        frame.set_cursor_position((
            input.x + (label.width() as u16).min(input.width - 1),
            input.y,
        ));
    }
}

pub(super) fn line_area(area: Rect, row: u16) -> Rect {
    Rect::new(area.x, area.y + row, area.width, 1)
}
