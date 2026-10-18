//! Original geometric ASTRID glyphs. Each pixel is a terminal block, not an image.
const GLYPHS: [[&str; 7]; 6] = [
    [
        " ███ ",
        "█   █",
        "█   █",
        "█████",
        "█   █",
        "█   █",
        "█   █",
    ],
    [
        "█████",
        "█    ",
        "█    ",
        "█████",
        "    █",
        "    █",
        "█████",
    ],
    [
        "█████",
        "  █  ",
        "  █  ",
        "  █  ",
        "  █  ",
        "  █  ",
        "  █  ",
    ],
    [
        "████ ",
        "█   █",
        "█   █",
        "████ ",
        "█ █  ",
        "█  █ ",
        "█   █",
    ],
    [
        "█████",
        "  █  ",
        "  █  ",
        "  █  ",
        "  █  ",
        "  █  ",
        "█████",
    ],
    [
        "████ ",
        "█   █",
        "█   █",
        "█   █",
        "█   █",
        "█   █",
        "████ ",
    ],
];

pub(super) fn rows(scale: usize) -> Vec<String> {
    (0..7)
        .map(|row| {
            GLYPHS
                .iter()
                .map(|glyph| {
                    glyph[row]
                        .chars()
                        .map(|c| c.to_string().repeat(scale))
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
                .join(&" ".repeat(scale))
        })
        .collect()
}
