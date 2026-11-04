//! `Callout`: the result line of a command (`✓` success, `!` warning, `✕` error, `›` note), details, then
//! `Next: <command>`. `--plain`: `OK:`, `WARNING:`, `ERROR:`, `NOTE:`.

use crate::Render;
use crate::mode::Mode;
use crate::theme::{Token, paint};
use crate::width::truncate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Success,
    Warning,
    Error,
    Info,
}

impl Kind {
    fn glyph(self) -> &'static str {
        match self {
            Kind::Success => "✓",
            Kind::Warning => "!",
            Kind::Error => "✕",
            Kind::Info => "›",
        }
    }

    fn word(self) -> &'static str {
        match self {
            Kind::Success => "OK:",
            Kind::Warning => "WARNING:",
            Kind::Error => "ERROR:",
            Kind::Info => "NOTE:",
        }
    }

    fn token(self) -> Token {
        match self {
            Kind::Success => Token::Teal,
            Kind::Warning => Token::Yellow,
            Kind::Error => Token::Coral,
            Kind::Info => Token::Blue,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Callout {
    kind: Kind,
    message: String,
    details: Vec<String>,
    next: Vec<String>,
}

impl Callout {
    pub fn new(kind: Kind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            details: Vec::new(),
            next: Vec::new(),
        }
    }

    pub fn success(message: impl Into<String>) -> Self {
        Self::new(Kind::Success, message)
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(Kind::Warning, message)
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(Kind::Error, message)
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self::new(Kind::Info, message)
    }

    /// A line under the message, indented by two spaces.
    pub fn detail(mut self, line: impl Into<String>) -> Self {
        self.details.push(line.into());
        self
    }

    pub fn details<I: IntoIterator<Item = S>, S: Into<String>>(mut self, lines: I) -> Self {
        self.details.extend(lines.into_iter().map(Into::into));
        self
    }

    /// A command to run next (`Next: <cmd>`); several make several lines.
    pub fn next(mut self, command: impl Into<String>) -> Self {
        self.next.push(command.into());
        self
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }
}

/// `Next: <cmd>` with `Next:` muted and the command blue.
pub fn next_line(mode: &Mode, command: &str) -> String {
    format!(
        "{} {}",
        paint(mode, Token::Muted, "Next:"),
        paint(mode, Token::Blue, command)
    )
}

impl Render for Callout {
    fn render(&self, mode: &Mode) -> String {
        let lead = if mode.plain {
            self.kind.word().to_string()
        } else {
            paint(mode, self.kind.token(), self.kind.glyph())
        };
        let mut lines = vec![format!("{lead} {}", self.message)];
        for d in &self.details {
            for part in d.lines() {
                lines.push(format!("  {}", truncate(part, mode.width.saturating_sub(2))));
            }
        }
        for cmd in &self.next {
            lines.push(next_line(mode, cmd));
        }
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::ColorLevel;

    #[test]
    fn forms() {
        let c = Callout::success("Signed in as An")
            .detail("host: x")
            .next("ttm whoami");
        let mode = Mode::fixed(ColorLevel::None, false, 80);
        assert_eq!(c.render(&mode), "✓ Signed in as An\n  host: x\nNext: ttm whoami");
        let plain = Mode::fixed(ColorLevel::None, true, 80);
        assert_eq!(
            c.render(&plain),
            "OK: Signed in as An\n  host: x\nNext: ttm whoami"
        );
    }
}
