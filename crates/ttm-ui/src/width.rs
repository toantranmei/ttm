//! Display width (terminal columns), truncation with `…` and padding. Escape codes take no room.

use unicode_width::UnicodeWidthChar;

/// Horizontal alignment of a cell or a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    #[default]
    Left,
    Right,
    Center,
}

/// Splits `text` into escape sequences (zero width) and characters.
enum Piece<'a> {
    Escape(&'a str),
    Char(char),
}

fn pieces(text: &str) -> impl Iterator<Item = Piece<'_>> {
    let bytes = text.as_bytes();
    let mut i = 0;
    std::iter::from_fn(move || {
        if i >= text.len() {
            return None;
        }
        if bytes[i] == 0x1b {
            let start = i;
            i += 1;
            if i < bytes.len() && bytes[i] == b'[' {
                // CSI: ESC [ params final(0x40..=0x7e)
                i += 1;
                while i < bytes.len() && !(0x40..=0x7e).contains(&bytes[i]) {
                    i += 1;
                }
                i = (i + 1).min(bytes.len());
            } else if i < bytes.len() && bytes[i] == b']' {
                // OSC: ESC ] … (BEL | ESC \)
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == 0x07 {
                        i += 1;
                        break;
                    }
                    if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
            } else if i < bytes.len() {
                i += 1;
            }
            return Some(Piece::Escape(&text[start..i]));
        }
        let ch = text[i..].chars().next()?;
        i += ch.len_utf8();
        Some(Piece::Char(ch))
    })
}

fn char_width(ch: char) -> usize {
    ch.width().unwrap_or(0)
}

/// Columns `text` takes in a terminal (CJK and most emoji count 2; escape codes 0).
pub fn display_width(text: &str) -> usize {
    pieces(text)
        .map(|p| match p {
            Piece::Escape(_) => 0,
            Piece::Char(ch) => char_width(ch),
        })
        .sum()
}

/// Removes escape codes.
pub fn strip_ansi(text: &str) -> String {
    pieces(text)
        .filter_map(|p| match p {
            Piece::Escape(_) => None,
            Piece::Char(ch) => Some(ch),
        })
        .collect()
}

/// `text` cut to at most `max` columns, ending with `…` when something was cut. Never wraps. Escape codes are kept
/// and a reset is added when a coloured text was cut.
pub fn truncate(text: &str, max: usize) -> String {
    if display_width(text) <= max {
        return text.to_string();
    }
    if max == 0 {
        return String::new();
    }
    let budget = max - 1; // room for `…`
    let mut out = String::new();
    let mut used = 0;
    let mut escaped = false;
    for piece in pieces(text) {
        match piece {
            Piece::Escape(code) => {
                escaped = true;
                out.push_str(code);
            }
            Piece::Char(ch) => {
                let w = char_width(ch);
                if used + w > budget {
                    break;
                }
                used += w;
                out.push(ch);
            }
        }
    }
    out.push('…');
    if escaped {
        out.push_str("\u{1b}[0m");
    }
    out
}

/// `text` padded with spaces to `width` columns (cut first when longer).
pub fn pad(text: &str, width: usize, align: Align) -> String {
    let text = truncate(text, width);
    let gap = width.saturating_sub(display_width(&text));
    match align {
        Align::Left => format!("{text}{}", " ".repeat(gap)),
        Align::Right => format!("{}{text}", " ".repeat(gap)),
        Align::Center => {
            let left = gap / 2;
            format!("{}{text}{}", " ".repeat(left), " ".repeat(gap - left))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths() {
        assert_eq!(display_width("abc"), 3);
        assert_eq!(display_width("Tiếng Việt có dấu"), 17);
        assert_eq!(display_width("中文"), 4);
        assert_eq!(display_width("🤖 agent"), 8);
        assert_eq!(display_width("\u{1b}[38;2;1;2;3mok\u{1b}[0m"), 2);
        assert_eq!(strip_ansi("\u{1b}[1mok\u{1b}[0m"), "ok");
    }

    #[test]
    fn truncate_vietnamese_cjk_emoji() {
        assert_eq!(truncate("Đăng nhập bằng Google", 10), "Đăng nhập…");
        assert_eq!(display_width(&truncate("Đăng nhập bằng Google", 10)), 10);
        // A wide character never straddles the edge.
        assert_eq!(truncate("中文中文", 4), "中…");
        assert_eq!(truncate("中文中文", 5), "中文…");
        assert_eq!(truncate("🤖🤖🤖", 4), "🤖…");
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("abc", 0), "");
        assert_eq!(truncate("abcdef", 1), "…");
        let colored = truncate("\u{1b}[31mabcdef\u{1b}[0m", 4);
        assert_eq!(strip_ansi(&colored), "abc…");
        assert!(colored.ends_with("\u{1b}[0m"));
    }

    #[test]
    fn padding() {
        assert_eq!(pad("ab", 4, Align::Left), "ab  ");
        assert_eq!(pad("ab", 4, Align::Right), "  ab");
        assert_eq!(pad("ab", 5, Align::Center), " ab  ");
        assert_eq!(pad("中", 3, Align::Left), "中 ");
        assert_eq!(pad("abcdef", 4, Align::Left), "abc…");
    }
}
