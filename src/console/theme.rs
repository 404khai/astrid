#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Ink {
    #[default]
    Normal,
    Dim,
    Accent,
    Unbound,
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
            Self::Accent => "38;2;68;89;249",
            Self::Unbound => "38;2;249;68;71",
            Self::Success => "32",
            Self::Warning => "33",
            Self::Error => "31",
            Self::Reply => "36",
        };
        format!("\x1b[{code}m{text}\x1b[0m")
    }
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
            Self::Accent => Style::default().fg(Color::Rgb(68, 89, 249)),
            Self::Unbound => Style::default().fg(Color::Rgb(249, 68, 71)),
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
        composer_style(true).patch(Ink::Accent.style(true))
    } else {
        ratatui::style::Style::default()
    }
}

/// Original bracketed-face palette, including its teal/yellow eye pixels.
pub(super) fn logo_rgb(row: usize, column: usize, unbound: bool) -> (u8, u8, u8) {
    if (3..=5).contains(&row) && matches!(column, 10 | 11 | 16 | 17) {
        if unbound {
            (247, 198, 0)
        } else {
            (0, 247, 213)
        }
    } else if unbound {
        if (2..=7).contains(&row) {
            (236, 26, 29)
        } else {
            (249, 68, 71)
        }
    } else if (2..=7).contains(&row) {
        (26, 50, 236)
    } else {
        (68, 89, 249)
    }
}

/// ARC stops at 0%, 50%, 100%, interpolated over the seven glyph rows.
/// Unbound retains ADR 0011's red danger bands; the wordmark has no eyes.
pub(super) fn wordmark_rgb(row: usize, unbound: bool) -> (u8, u8, u8) {
    let (edge, middle) = if unbound {
        ([249_i32, 68, 71], [236_i32, 26, 29])
    } else {
        ([68_i32, 89, 249], [26_i32, 50, 236])
    };
    let step = 3 - row.min(6).abs_diff(3);
    let channel = |i: usize| (edge[i] + (middle[i] - edge[i]) * step as i32 / 3) as u8;
    (channel(0), channel(1), channel(2))
}

pub(super) fn wordmark_style(row: usize, color: bool, unbound: bool) -> ratatui::style::Style {
    let (r, g, b) = wordmark_rgb(row, unbound);
    if color {
        ratatui::style::Style::default().fg(ratatui::style::Color::Rgb(r, g, b))
    } else {
        ratatui::style::Style::default()
    }
}

pub(super) fn shadow_style(color: bool) -> ratatui::style::Style {
    if color {
        ratatui::style::Style::default().fg(ratatui::style::Color::Rgb(35, 40, 66))
    } else {
        ratatui::style::Style::default()
    }
}

pub(super) fn title_ink(unbound: bool) -> Ink {
    if unbound { Ink::Unbound } else { Ink::Accent }
}

/// Resolve client theme accents using the effective permission mode.
pub(super) fn mode_ink(ink: Ink, unbound: bool) -> Ink {
    if unbound && matches!(ink, Ink::Accent | Ink::Reply) {
        Ink::Unbound
    } else {
        ink
    }
}

pub(super) fn mode_line(text: String, unbound: bool, color: bool) -> ratatui::text::Line<'static> {
    use ratatui::text::{Line, Span};
    if unbound && let Some(rest) = text.strip_prefix("unbound") {
        Line::from(vec![
            Span::styled("unbound", Ink::Unbound.style(color)),
            Span::styled(rest.to_owned(), Ink::Dim.style(color)),
        ])
    } else {
        Line::styled(text, Ink::Dim.style(color))
    }
}
