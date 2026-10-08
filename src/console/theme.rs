#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Ink {
    #[default]
    Normal,
    Dim,
    Accent,
    Success,
    Warning,
    Error,
    Reply,
}
impl Ink {
    pub(super) fn paint(self, text: &str, color: bool) -> String {
        if !color {
            return text.to_owned();
        }
        let code = match self {
            Self::Normal => "0",
            Self::Dim => "2",
            Self::Accent => "38;2;0;247;213",
            Self::Success => "32",
            Self::Warning => "33",
            Self::Error => "31",
            Self::Reply => "36",
        };
        format!("\x1b[{code}m{text}\x1b[0m")
    }
}
pub(super) fn paint_logo(line: &str, row: usize, color: bool, unbound: bool) -> String {
    if !color {
        return line.to_owned();
    }
    let mut result = String::new();
    for (column, pixel) in line.chars().enumerate() {
        if pixel == ' ' {
            result.push(pixel);
            continue;
        }
        let code = if (3..=5).contains(&row) && matches!(column, 10 | 11 | 16 | 17) {
            if unbound {
                "38;2;247;198;0"
            } else {
                "38;2;0;247;213"
            }
        } else if unbound {
            // Upper, middle, and lower arch bands.
            if (2..=7).contains(&row) {
                "38;2;236;26;29"
            } else {
                "38;2;249;68;71"
            }
        } else {
            "38;2;68;89;249"
        };
        result.push_str(&format!("\x1b[{code}m{pixel}\x1b[0m"));
    }
    result
}

impl Ink {
    pub(super) fn style(self, color: bool) -> ratatui::style::Style {
        use ratatui::style::{Color, Modifier, Style};
        if !color {
            return Style::default();
        }
        match self {
            Self::Normal => Style::default(),
            Self::Dim => Style::default().add_modifier(Modifier::DIM),
            Self::Accent => Style::default().fg(Color::Rgb(0, 247, 213)),
            Self::Success => Style::default().fg(Color::Green),
            Self::Warning => Style::default().fg(Color::Yellow),
            Self::Error => Style::default().fg(Color::Red),
            Self::Reply => Style::default().fg(Color::Cyan),
        }
    }
}

pub(super) fn composer_style(color: bool) -> ratatui::style::Style {
    if color {
        Ink::Normal
            .style(true)
            .bg(ratatui::style::Color::Indexed(235))
    } else {
        ratatui::style::Style::default()
    }
}
pub(super) fn selection_style(color: bool) -> ratatui::style::Style {
    if color {
        composer_style(true).fg(ratatui::style::Color::Indexed(208))
    } else {
        ratatui::style::Style::default()
    }
}
