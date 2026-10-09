use super::{
    format::truncate,
    theme::{Ink, paint_logo},
};
use crate::logo::LOGO;
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
impl Identity {
    pub(super) fn rows(&self, width: usize) -> Vec<String> {
        let labels = width.saturating_sub(4).min(14);
        let row = |label: &str, value: &str| {
            let label = truncate(label, labels.saturating_sub(2));
            format!(
                "{label:labels$}{}",
                truncate(value, width.saturating_sub(labels))
            )
        };
        let mut rows = vec![
            row("mode", &self.mode),
            row("model", &self.model),
            row("provider", &self.provider),
            row("cwd", &self.cwd),
        ];
        if let Some(instructions) = &self.instructions {
            rows.push(row("instructions", instructions));
        }
        rows.push(row(
            "tools",
            &tool_list(&self.tools, width.saturating_sub(labels)),
        ));
        rows
    }
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

pub(super) fn append_header(
    out: &mut impl Write,
    identity: &Identity,
    width: usize,
    color: bool,
) -> io::Result<()> {
    let mut metadata = vec![
        format!("astrid  {}", env!("CARGO_PKG_VERSION")),
        String::new(),
    ];
    metadata.extend(identity.rows(width.saturating_sub(34).max(20)));
    writeln!(out)?;
    if width >= 78 {
        for (row, logo) in LOGO.lines().enumerate() {
            let text = metadata.get(row).map(String::as_str).unwrap_or("");
            let ink = if row == 0 {
                super::theme::title_ink(identity.mode == "unbound")
            } else {
                Ink::Normal
            };
            writeln!(
                out,
                "{}{}{}",
                paint_logo(logo, row, color, identity.mode == "unbound"),
                " ".repeat(34 - logo.width()),
                ink.paint(text, color)
            )?;
        }
    } else {
        for (row, logo) in LOGO.lines().enumerate() {
            writeln!(
                out,
                "{}",
                paint_logo(
                    &truncate(logo, width),
                    row,
                    color,
                    identity.mode == "unbound"
                )
            )?;
        }
        writeln!(out)?;
        writeln!(
            out,
            "{}",
            super::theme::title_ink(identity.mode == "unbound").paint(&metadata[0], color)
        )?;
        for row in identity.rows(width) {
            writeln!(out, "{row}")?;
        }
    }
    writeln!(out)?;
    out.flush()
}

/// Live startup identity, owned by the composer until the first task is sent.
pub(super) fn header_lines(
    identity: &Identity,
    width: usize,
    color: bool,
) -> Vec<ratatui::text::Line<'static>> {
    use ratatui::{
        style::{Color, Style},
        text::{Line, Span},
    };
    let wide = width >= 78;
    let mut metadata = vec![
        format!("astrid  {}", env!("CARGO_PKG_VERSION")),
        String::new(),
    ];
    metadata.extend(identity.rows(width.saturating_sub(34).max(20)));
    let mut lines = vec![Line::default()];
    for (row, logo) in LOGO.lines().enumerate() {
        let mut spans: Vec<Span<'static>> = truncate(logo, width)
            .chars()
            .enumerate()
            .map(|(column, pixel)| {
                let (r, g, b) = super::theme::logo_rgb(row, column, identity.mode == "unbound");
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
        if wide {
            spans.push(Span::raw(" ".repeat(34 - logo.width())));
            spans.push(Span::styled(
                metadata.get(row).cloned().unwrap_or_default(),
                if row == 0 {
                    super::theme::title_ink(identity.mode == "unbound")
                } else {
                    Ink::Normal
                }
                .style(color),
            ));
        }
        lines.push(Line::from(spans));
    }
    if !wide {
        lines.push(Line::default());
        lines.push(Line::styled(
            metadata[0].clone(),
            super::theme::title_ink(identity.mode == "unbound").style(color),
        ));
        lines.extend(identity.rows(width).into_iter().map(Line::from));
    }
    lines.push(Line::default());
    lines
}
