use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub fn printable(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
        .collect()
}
pub(super) fn single_line(value: &str) -> String {
    printable(value).replace(['\n', '\t'], " ")
}
pub(super) fn truncate(value: &str, width: usize) -> String {
    let value = single_line(value);
    if value.width() <= width {
        return value;
    }
    if width == 0 {
        return String::new();
    }
    let mut result = String::new();
    let mut used = 0;
    for c in value.graphemes(true) {
        let cells = c.width();
        if used + cells > width - 1 {
            break;
        }
        result.push_str(c);
        used += cells;
    }
    result.push('…');
    result
}
