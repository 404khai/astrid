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
            let ink = if row == 0 { Ink::Accent } else { Ink::Normal };
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
        writeln!(out, "{}", Ink::Accent.paint(&metadata[0], color))?;
        for row in identity.rows(width) {
            writeln!(out, "{row}")?;
        }
    }
    writeln!(out)?;
    out.flush()
}
