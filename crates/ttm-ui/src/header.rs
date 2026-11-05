//! `Header`: a rounded frame with a title and a one-line summary.
//!
//! ```text
//! ╭─ Aurora · rev 43 ────────╮
//! │ 12 tasks, 3 in progress  │
//! ╰──────────────────────────╯
//! ```
//! `--plain`: `Aurora · rev 43 — 12 tasks, 3 in progress`.

use crate::Render;
use crate::mode::Mode;
use crate::theme::{Token, paint};
use crate::width::{display_width, pad, truncate};

#[derive(Debug, Clone)]
pub struct Header {
    title: String,
    summary: Option<String>,
}

impl Header {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            summary: None,
        }
    }

    pub fn summary(mut self, line: impl Into<String>) -> Self {
        let line = line.into();
        self.summary = (!line.is_empty()).then_some(line);
        self
    }
}

impl Render for Header {
    fn render(&self, mode: &Mode) -> String {
        if mode.plain {
            return match &self.summary {
                Some(s) => format!("{} — {s}", self.title),
                None => self.title.clone(),
            };
        }
        // Inner width: between the two corners.
        let max_inner = mode.width.saturating_sub(2).max(8);
        let title_w = display_width(&self.title);
        let summary_w = self.summary.as_deref().map(display_width).unwrap_or(0);
        let inner = (title_w + 4).max(summary_w + 2).min(max_inner);
        let title = truncate(&self.title, inner.saturating_sub(4));
        let fill = inner.saturating_sub(display_width(&title) + 3);
        let frame = |s: &str| paint(mode, Token::Muted, s);
        let mut lines = vec![format!(
            "{}{}{}",
            frame("╭─ "),
            paint(mode, Token::Ink, &title),
            frame(&format!(" {}╮", "─".repeat(fill)))
        )];
        if let Some(summary) = &self.summary {
            lines.push(format!(
                "{}{}{}",
                frame("│ "),
                pad(summary, inner - 2, crate::width::Align::Left),
                frame(" │")
            ));
        }
        lines.push(frame(&format!("╰{}╯", "─".repeat(inner))));
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::ColorLevel;

    #[test]
    fn fits_and_aligns() {
        let mode = Mode::fixed(ColorLevel::None, false, 80);
        let out = Header::new("Aurora").summary("12 tasks").render(&mode);
        let widths: Vec<_> = out.lines().map(display_width).collect();
        assert!(widths.iter().all(|w| *w == widths[0]), "{out}");
        let long = Header::new("x".repeat(200))
            .summary("y".repeat(200))
            .render(&mode);
        assert!(long.lines().all(|l| display_width(l) == 80), "{long}");
        let plain = Mode::fixed(ColorLevel::None, true, 80);
        assert_eq!(Header::new("A").summary("b").render(&plain), "A — b");
    }
}
