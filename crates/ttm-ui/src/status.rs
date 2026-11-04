//! `Status`: the glyph of a task phase (`○ ● ◐ ✓ ✕ ⏸`) or of a check (`✓ ! ✕`), coloured; a word with `--plain`.

use crate::Render;
use crate::mode::Mode;
use crate::theme::{Token, paint};
use ttm_api::vocab::{self, Phase};

/// Result of a check (`ttm doctor`, `ttm status`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Ok,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Phase { phase: Phase, waiting: bool },
    Check(Level),
}

impl Status {
    /// The status of a workflow category (`in-progress`, `qc`, `approval`, …) or `blocked`/`paused`.
    pub fn of_category(category: &str) -> Self {
        Status::Phase {
            phase: vocab::phase_of_category(category),
            waiting: vocab::is_waiting_category(category),
        }
    }

    pub fn phase(phase: Phase) -> Self {
        Status::Phase {
            phase,
            waiting: false,
        }
    }

    pub fn level(level: Level) -> Self {
        Status::Check(level)
    }

    pub fn ok() -> Self {
        Status::Check(Level::Ok)
    }

    pub fn warn() -> Self {
        Status::Check(Level::Warn)
    }

    pub fn fail() -> Self {
        Status::Check(Level::Fail)
    }

    fn token(self) -> Token {
        match self {
            Status::Phase { phase, waiting } => match phase {
                Phase::Todo => Token::Muted,
                Phase::Doing => Token::Teal,
                Phase::Review if waiting => Token::Yellow,
                Phase::Review => Token::Purple,
                Phase::Done => Token::Teal,
                Phase::Blocked => Token::Coral,
                Phase::Paused => Token::Yellow,
            },
            Status::Check(Level::Ok) => Token::Teal,
            Status::Check(Level::Warn) => Token::Yellow,
            Status::Check(Level::Fail) => Token::Coral,
        }
    }

    /// The glyph without colour.
    pub fn glyph(self) -> &'static str {
        match self {
            Status::Phase { phase, .. } => phase.glyph(),
            Status::Check(Level::Ok) => "✓",
            Status::Check(Level::Warn) => "!",
            Status::Check(Level::Fail) => "✕",
        }
    }

    /// The `--plain` word.
    pub fn word(self) -> &'static str {
        match self {
            Status::Phase { phase, .. } => phase.word(),
            Status::Check(Level::Ok) => "ok",
            Status::Check(Level::Warn) => "warn",
            Status::Check(Level::Fail) => "FAIL",
        }
    }
}

impl Render for Status {
    fn render(&self, mode: &Mode) -> String {
        if mode.plain {
            self.word().to_string()
        } else {
            paint(mode, self.token(), self.glyph())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::ColorLevel;

    #[test]
    fn glyphs_and_words() {
        let mode = Mode::fixed(ColorLevel::None, false, 80);
        let plain = Mode::fixed(ColorLevel::None, true, 80);
        assert_eq!(Status::of_category("in-progress").render(&mode), "●");
        assert_eq!(Status::of_category("approval").render(&plain), "review");
        assert_eq!(Status::fail().render(&plain), "FAIL");
        assert_eq!(Status::warn().render(&mode), "!");
        assert_eq!(Status::of_category("approval").token(), Token::Yellow);
        assert_eq!(Status::of_category("qc").token(), Token::Purple);
    }
}
