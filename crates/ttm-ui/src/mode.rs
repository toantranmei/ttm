//! How output is drawn: colour depth, plain text, JSON, interactivity and width.

use std::io::IsTerminal;

/// Colour depth of the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ColorLevel {
    None,
    Ansi16,
    Ansi256,
    TrueColor,
}

/// Rendering mode, decided once per process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mode {
    pub color: ColorLevel,
    /// `--plain` / `TTM_PLAIN=1`: no frames, no glyphs, no emoji.
    pub plain: bool,
    /// `--json`: nothing for humans goes to stdout.
    pub json: bool,
    /// stdin and stdout are both terminals (prompts and spinners allowed).
    pub interactive: bool,
    /// Columns to fit in (40..=200).
    pub width: usize,
}

pub const MIN_WIDTH: usize = 40;
pub const MAX_WIDTH: usize = 200;
pub const DEFAULT_WIDTH: usize = 80;

/// What the process can see of its terminal, for [`Mode::detect_with`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Probe {
    pub stdout_tty: bool,
    pub stdin_tty: bool,
    /// Columns reported by the terminal, if any.
    pub columns: Option<usize>,
}

impl Probe {
    /// The real terminal of this process.
    pub fn current() -> Probe {
        Probe {
            stdout_tty: std::io::stdout().is_terminal(),
            stdin_tty: std::io::stdin().is_terminal(),
            columns: terminal_size::terminal_size().map(|(w, _)| w.0 as usize),
        }
    }
}

fn set(value: Option<String>) -> bool {
    value.is_some_and(|v| !v.is_empty())
}

fn truthy(value: Option<String>) -> bool {
    value.is_some_and(|v| {
        let v = v.trim().to_ascii_lowercase();
        !(v.is_empty() || v == "0" || v == "false" || v == "no" || v == "off")
    })
}

impl Mode {
    /// Mode of this process from the real environment and terminal.
    pub fn detect(plain_flag: bool, json_flag: bool) -> Mode {
        Mode::detect_with(
            plain_flag,
            json_flag,
            &|name| std::env::var(name).ok(),
            Probe::current(),
        )
    }

    /// Same as [`Mode::detect`] with the environment and terminal given (tests).
    ///
    /// Order: `--json` (no colour, nothing human on stdout) → `--plain`/`TTM_PLAIN` → `NO_COLOR` set → no colour →
    /// `CLICOLOR_FORCE` → colour even when piped → stdout not a TTY → no colour → `TERM=dumb` → no colour →
    /// `COLORTERM=truecolor|24bit` or `WT_SESSION` → truecolor → `TERM=*256color*` → 256 → else 16 colours.
    pub fn detect_with(
        plain_flag: bool,
        json_flag: bool,
        env: &dyn Fn(&str) -> Option<String>,
        probe: Probe,
    ) -> Mode {
        let plain = plain_flag || truthy(env("TTM_PLAIN"));
        let width = Mode::width_from(env, probe.columns);
        let interactive = probe.stdout_tty && probe.stdin_tty;
        let color = if json_flag || plain || set(env("NO_COLOR")) {
            ColorLevel::None
        } else {
            let forced = truthy(env("CLICOLOR_FORCE"));
            let term = env("TERM").unwrap_or_default();
            if !forced && (!probe.stdout_tty || term == "dumb") {
                ColorLevel::None
            } else {
                Mode::level_from(env, &term)
            }
        };
        Mode {
            color,
            plain,
            json: json_flag,
            interactive,
            width,
        }
    }

    fn level_from(env: &dyn Fn(&str) -> Option<String>, term: &str) -> ColorLevel {
        let colorterm = env("COLORTERM").unwrap_or_default().to_ascii_lowercase();
        if colorterm == "truecolor" || colorterm == "24bit" || set(env("WT_SESSION")) {
            ColorLevel::TrueColor
        } else if term.contains("256color") {
            ColorLevel::Ansi256
        } else {
            ColorLevel::Ansi16
        }
    }

    /// `TTM_COLUMNS` > `COLUMNS` > terminal > 80, clamped to 40..=200.
    fn width_from(env: &dyn Fn(&str) -> Option<String>, columns: Option<usize>) -> usize {
        let parse = |name: &str| {
            env(name)
                .and_then(|v| v.trim().parse::<usize>().ok())
                .filter(|n| *n > 0)
        };
        parse("TTM_COLUMNS")
            .or_else(|| parse("COLUMNS"))
            .or(columns.filter(|n| *n > 0))
            .unwrap_or(DEFAULT_WIDTH)
            .clamp(MIN_WIDTH, MAX_WIDTH)
    }

    /// A fixed mode for tests and for output that must never change (hooks).
    pub fn fixed(color: ColorLevel, plain: bool, width: usize) -> Mode {
        Mode {
            color: if plain { ColorLevel::None } else { color },
            plain,
            json: false,
            interactive: false,
            width: width.clamp(MIN_WIDTH, MAX_WIDTH),
        }
    }

    pub fn colored(&self) -> bool {
        self.color != ColorLevel::None
    }

    /// Spinners and progress bars are drawn only for a person at a terminal.
    pub fn animated(&self) -> bool {
        self.interactive && !self.plain && !self.json
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_of(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |name| pairs.iter().find(|(k, _)| *k == name).map(|(_, v)| v.to_string())
    }

    const TTY: Probe = Probe {
        stdout_tty: true,
        stdin_tty: true,
        columns: Some(100),
    };
    const PIPE: Probe = Probe {
        stdout_tty: false,
        stdin_tty: false,
        columns: None,
    };

    #[test]
    fn json_and_plain_win() {
        let env = env_of(&[("COLORTERM", "truecolor"), ("CLICOLOR_FORCE", "1")]);
        let m = Mode::detect_with(false, true, &env, TTY);
        assert_eq!((m.color, m.json), (ColorLevel::None, true));
        let m = Mode::detect_with(true, false, &env, TTY);
        assert_eq!((m.color, m.plain), (ColorLevel::None, true));
        let m = Mode::detect_with(false, false, &env_of(&[("TTM_PLAIN", "1")]), TTY);
        assert!(m.plain);
        let m = Mode::detect_with(false, false, &env_of(&[("TTM_PLAIN", "0")]), TTY);
        assert!(!m.plain);
    }

    #[test]
    fn no_color_then_force_then_tty() {
        let m = Mode::detect_with(
            false,
            false,
            &env_of(&[("NO_COLOR", "1"), ("CLICOLOR_FORCE", "1")]),
            TTY,
        );
        assert_eq!(m.color, ColorLevel::None);
        // An empty NO_COLOR does not count.
        let m = Mode::detect_with(false, false, &env_of(&[("NO_COLOR", "")]), TTY);
        assert_eq!(m.color, ColorLevel::Ansi16);
        let m = Mode::detect_with(false, false, &env_of(&[("CLICOLOR_FORCE", "1")]), PIPE);
        assert_eq!(m.color, ColorLevel::Ansi16);
        let m = Mode::detect_with(false, false, &env_of(&[]), PIPE);
        assert_eq!(m.color, ColorLevel::None);
        let m = Mode::detect_with(false, false, &env_of(&[("TERM", "dumb")]), TTY);
        assert_eq!(m.color, ColorLevel::None);
    }

    #[test]
    fn color_depth() {
        let m = Mode::detect_with(false, false, &env_of(&[("COLORTERM", "24bit")]), TTY);
        assert_eq!(m.color, ColorLevel::TrueColor);
        let m = Mode::detect_with(false, false, &env_of(&[("WT_SESSION", "x")]), TTY);
        assert_eq!(m.color, ColorLevel::TrueColor);
        let m = Mode::detect_with(false, false, &env_of(&[("TERM", "xterm-256color")]), TTY);
        assert_eq!(m.color, ColorLevel::Ansi256);
        let m = Mode::detect_with(false, false, &env_of(&[("TERM", "xterm")]), TTY);
        assert_eq!(m.color, ColorLevel::Ansi16);
    }

    #[test]
    fn width_order_and_clamp() {
        let m = Mode::detect_with(
            false,
            false,
            &env_of(&[("TTM_COLUMNS", "120"), ("COLUMNS", "90")]),
            TTY,
        );
        assert_eq!(m.width, 120);
        let m = Mode::detect_with(false, false, &env_of(&[("COLUMNS", "90")]), TTY);
        assert_eq!(m.width, 90);
        assert_eq!(Mode::detect_with(false, false, &env_of(&[]), TTY).width, 100);
        assert_eq!(Mode::detect_with(false, false, &env_of(&[]), PIPE).width, 80);
        assert_eq!(
            Mode::detect_with(false, false, &env_of(&[("TTM_COLUMNS", "10")]), PIPE).width,
            40
        );
        assert_eq!(
            Mode::detect_with(false, false, &env_of(&[("TTM_COLUMNS", "999")]), PIPE).width,
            200
        );
        assert_eq!(
            Mode::detect_with(false, false, &env_of(&[("TTM_COLUMNS", "abc")]), PIPE).width,
            80
        );
    }

    #[test]
    fn interactive_needs_both_ttys() {
        let half = Probe {
            stdout_tty: true,
            stdin_tty: false,
            columns: None,
        };
        assert!(!Mode::detect_with(false, false, &env_of(&[]), half).interactive);
        assert!(Mode::detect_with(false, false, &env_of(&[]), TTY).interactive);
    }
}
