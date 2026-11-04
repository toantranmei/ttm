//! Brand colour tokens (same values as `apps/web/app/assets/css/tokens.css` of the server).

use crate::mode::{ColorLevel, Mode};
use anstyle::{AnsiColor, Color, RgbColor, Style};

/// What a piece of text means; the colour follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Token {
    /// Emphasis: bold in the terminal's own text colour (brand ink is unreadable on dark backgrounds).
    Ink,
    /// Success, running.
    Teal,
    /// Waiting, warning.
    Yellow,
    /// Error, blocked.
    Coral,
    /// Review, design.
    Purple,
    /// Information, links, commands to run.
    Blue,
    /// Secondary text: faint.
    Muted,
    /// Mei-er only.
    Gold,
}

impl Token {
    /// Brand RGB, for the tokens that have one.
    pub fn rgb(self) -> Option<RgbColor> {
        match self {
            Token::Teal => Some(RgbColor(0x5b, 0xb7, 0xa8)),
            Token::Yellow => Some(RgbColor(0xf2, 0xb8, 0x4b)),
            Token::Coral => Some(RgbColor(0xe9, 0x79, 0x62)),
            Token::Purple => Some(RgbColor(0xb5, 0x97, 0xd6)),
            Token::Blue => Some(RgbColor(0x55, 0xa4, 0xcf)),
            Token::Gold => Some(RgbColor(0xd4, 0xa7, 0x2c)),
            Token::Ink | Token::Muted => None,
        }
    }

    /// Nearest of the 16 basic colours.
    pub fn ansi16(self) -> Option<AnsiColor> {
        match self {
            Token::Teal => Some(AnsiColor::Cyan),
            Token::Yellow => Some(AnsiColor::Yellow),
            Token::Coral => Some(AnsiColor::Red),
            Token::Purple => Some(AnsiColor::Magenta),
            Token::Blue => Some(AnsiColor::Blue),
            Token::Gold => Some(AnsiColor::BrightYellow),
            Token::Ink | Token::Muted => None,
        }
    }
}

/// The terminal style of `token` at colour depth `level` (plain style when there is no colour).
pub fn style(token: Token, level: ColorLevel) -> Style {
    if level == ColorLevel::None {
        return Style::new();
    }
    let base = match token {
        Token::Ink => Style::new().bold(),
        Token::Muted => Style::new().dimmed(),
        Token::Gold => Style::new().bold(),
        _ => Style::new(),
    };
    let color = match level {
        ColorLevel::None => None,
        ColorLevel::TrueColor => token.rgb().map(Color::Rgb),
        ColorLevel::Ansi256 => token
            .rgb()
            .map(|rgb| Color::Ansi256(anstyle_lossy::rgb_to_xterm(rgb))),
        ColorLevel::Ansi16 => token.ansi16().map(Color::Ansi),
    };
    base.fg_color(color)
}

/// `text` wrapped in the escape codes of `token`, or unchanged without colour.
pub fn paint(mode: &Mode, token: Token, text: &str) -> String {
    if !mode.colored() || text.is_empty() {
        return text.to_string();
    }
    let style = style(token, mode.color);
    format!("{}{text}{}", style.render(), style.render_reset())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paint_depths() {
        let none = Mode::fixed(ColorLevel::None, false, 80);
        assert_eq!(paint(&none, Token::Teal, "ok"), "ok");
        let tc = Mode::fixed(ColorLevel::TrueColor, false, 80);
        assert_eq!(
            paint(&tc, Token::Teal, "ok"),
            "\u{1b}[38;2;91;183;168mok\u{1b}[0m"
        );
        let c16 = Mode::fixed(ColorLevel::Ansi16, false, 80);
        assert_eq!(paint(&c16, Token::Coral, "x"), "\u{1b}[31mx\u{1b}[0m");
        let c256 = Mode::fixed(ColorLevel::Ansi256, false, 80);
        assert!(paint(&c256, Token::Blue, "x").starts_with("\u{1b}[38;5;"));
        assert_eq!(paint(&tc, Token::Ink, "x"), "\u{1b}[1mx\u{1b}[0m");
        assert_eq!(paint(&tc, Token::Muted, "x"), "\u{1b}[2mx\u{1b}[0m");
        // Plain never colours, whatever was asked.
        let plain = Mode::fixed(ColorLevel::TrueColor, true, 80);
        assert_eq!(paint(&plain, Token::Gold, "x"), "x");
    }
}
