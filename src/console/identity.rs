use super::{format::truncate, theme::Ink};
use std::io::{self, Write};
use unicode_width::UnicodeWidthStr;

#[derive(Clone)]
pub struct Identity {
    pub model: String,
    pub mode: String,
    pub provider: String,
    pub cwd: String,
    pub instructions: Option<String>,
    pub tools: Vec<String>,
}
pub(super) fn tool_list(tools: &[String], width: usize) -> String {
    for shown in (0..=tools.len().min(5)).rev() {
        let mut value = tools[..shown].join(", ");
        let remaining = tools.len() - shown;
        if remaining > 0 {
            if shown > 0 {
                value.push_str("  ");
            }
            value.push_str(&format!("+{remaining} more"));
        }
        if value.width() <= width {
            return value;
        }
    }
    truncate(&format!("{} tools", tools.len()), width)
}

fn mascot_lines(color: bool, unbound: bool) -> Vec<ratatui::text::Line<'static>> {
    use ratatui::{
        style::{Color, Style},
        text::{Line, Span},
    };
    crate::logo::LOGO
        .lines()
        .enumerate()
        .map(|(row, text)| {
            let mut spans: Vec<_> = text
                .chars()
                .enumerate()
                .map(|(column, pixel)| {
                    let (r, g, b) = super::theme::logo_rgb(row, column, unbound);
                    Span::styled(
                        pixel.to_string(),
                        if color && pixel != ' ' {
                            Style::default().fg(Color::Rgb(r, g, b))
                        } else {
                            Style::default()
                        },
                    )
                })
                .collect();
            spans.push(Span::raw(" ".repeat(30 - text.width())));
            Line::from(spans)
        })
        .collect()
}

fn wordmark_lines(scale: usize, color: bool, unbound: bool) -> Vec<ratatui::text::Line<'static>> {
    use ratatui::text::{Line, Span};
    let pixels = super::wordmark::rows(scale);
    (0usize..=7)
        .map(|row| {
            let front: Vec<char> = pixels
                .get(row)
                .map(|s| s.chars().collect())
                .unwrap_or_default();
            let previous: Vec<char> = row
                .checked_sub(1)
                .and_then(|i| pixels.get(i))
                .map(|s| s.chars().collect())
                .unwrap_or_default();
            Line::from(
                (0..=pixels[0].width())
                    .map(|x| {
                        if front.get(x) == Some(&'█') {
                            Span::styled("█", super::theme::wordmark_style(row, color, unbound))
                        } else if x > 0 && previous.get(x - 1) == Some(&'█') {
                            Span::styled("░", super::theme::shadow_style(color))
                        } else {
                            Span::raw(" ")
                        }
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}

fn hero_lines(
    width: usize,
    height: usize,
    color: bool,
    unbound: bool,
) -> Vec<ratatui::text::Line<'static>> {
    use ratatui::text::{Line, Span};
    if width < 30 || height < 16 {
        return Vec::new();
    }
    let mascot = mascot_lines(color, unbound);
    let mut lines = Vec::new();
    if width >= 69 {
        lines.push(Line::default());
        let scale = if width >= 104 { 2 } else { 1 };
        let wordmark = wordmark_lines(scale, color, unbound);
        let text_width = wordmark[0].width();
        for (row, face) in mascot.into_iter().enumerate() {
            let mut spans = face.spans;
            spans.push(Span::raw("   "));
            if let Some(text) = row.checked_sub(1).and_then(|r| wordmark.get(r)) {
                spans.extend(text.spans.clone());
            } else {
                spans.push(Span::raw(" ".repeat(text_width)));
            }
            lines.push(Line::from(spans));
        }
    } else if width >= 36 && height >= 26 {
        lines.push(Line::default());
        lines.extend(mascot);
        lines.push(Line::default());
        lines.extend(wordmark_lines(1, color, unbound));
    } else {
        // Full original mascot with a compact text wordmark when stacking cannot fit.
        lines.push(Line::styled(
            "ASTRID",
            super::theme::title_ink(unbound).style(color),
        ));
        lines.extend(mascot);
    }
    lines.push(Line::styled(
        "─".repeat(width.min(56)),
        Ink::Dim.style(color),
    ));
    for line in &mut lines {
        line.spans.insert(
            0,
            Span::raw(" ".repeat(width.saturating_sub(line.width()) / 2)),
        );
    }
    lines
}

/// Shared terminal-native welcome. Height is a budget, so decoration yields to input.
pub(super) fn header_lines(
    identity: &Identity,
    width: usize,
    height: usize,
    color: bool,
) -> Vec<ratatui::text::Line<'static>> {
    use ratatui::text::{Line, Span};
    let column = width.min(76);
    let left = width.saturating_sub(column) / 2;
    let unbound = identity.mode == "unbound";
    let mut lines = Vec::new();
    let centered = |text: &str, ink: Ink| {
        let text = truncate(text, column);
        Line::from(vec![
            Span::raw(" ".repeat(left + column.saturating_sub(text.width()) / 2)),
            Span::styled(text, ink.style(color)),
        ])
    };
    lines.extend(hero_lines(width, height, color, unbound));
    lines.push(centered(
        &if column < 30 {
            format!("{} · astrid", identity.mode)
        } else {
            format!("astrid  {} · {}", env!("CARGO_PKG_VERSION"), identity.mode)
        },
        super::theme::title_ink(unbound),
    ));
    if height > lines.len() {
        lines.push(centered(&format!("model  {}", identity.model), Ink::Normal));
    }
    if height > lines.len() {
        lines.push(centered(&format!("directory  {}", identity.cwd), Ink::Dim));
    }
    if height > lines.len() {
        lines.push(centered(
            &format!("provider  {}", identity.provider),
            Ink::Dim,
        ));
    }
    if height > lines.len() {
        let instructions = identity.instructions.as_deref().unwrap_or("no AGENTS.md");
        lines.push(centered(
            &format!(
                "{instructions} · {}",
                tool_list(
                    &identity.tools,
                    column.saturating_sub(instructions.width() + 3)
                )
            ),
            Ink::Dim,
        ));
    }
    if height > lines.len() {
        lines.push(Line::default());
    }
    lines.truncate(height);
    lines
}

/// Serialize the same styled rows for append and compatibility terminals.
pub(super) fn write_line(
    out: &mut impl Write,
    line: &ratatui::text::Line<'_>,
    color: bool,
) -> io::Result<()> {
    use ratatui::style::{Color, Modifier};
    for span in &line.spans {
        if color {
            if let Some(Color::Rgb(r, g, b)) = span.style.fg {
                write!(out, "\x1b[38;2;{r};{g};{b}m")?;
            }
            if span.style.add_modifier.contains(Modifier::DIM) {
                write!(out, "\x1b[2m")?;
            }
        }
        write!(out, "{}", span.content)?;
        if color {
            write!(out, "\x1b[0m")?;
        }
    }
    Ok(())
}

pub(super) fn append_header(
    out: &mut impl Write,
    identity: &Identity,
    width: usize,
    height: usize,
    color: bool,
) -> io::Result<()> {
    // Plain output stays compact and never emits decorative block art.
    for line in header_lines(
        identity,
        width,
        height.min(if color { 18 } else { 6 }),
        color,
    ) {
        write_line(out, &line, color)?;
        writeln!(out)?;
    }
    out.flush()
}
