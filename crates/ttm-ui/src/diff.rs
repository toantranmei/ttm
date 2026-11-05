//! `Diff`: unified text diff (`+` teal, `-` coral, `@@` purple, 3 lines of context) or a list of changes for a
//! workflow upgrade, a change request or an install preview.

use crate::Render;
use crate::mode::Mode;
use crate::theme::{Token, paint};
use crate::width::truncate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Removed,
    Changed,
}

#[derive(Debug, Clone)]
pub struct Change {
    pub kind: ChangeKind,
    pub label: String,
    pub detail: Option<String>,
}

impl Change {
    pub fn added(label: impl Into<String>) -> Self {
        Self::new(ChangeKind::Added, label)
    }

    pub fn removed(label: impl Into<String>) -> Self {
        Self::new(ChangeKind::Removed, label)
    }

    pub fn changed(label: impl Into<String>) -> Self {
        Self::new(ChangeKind::Changed, label)
    }

    pub fn new(kind: ChangeKind, label: impl Into<String>) -> Self {
        Self {
            kind,
            label: label.into(),
            detail: None,
        }
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

#[derive(Debug, Clone)]
pub enum Diff {
    Text { old: String, new: String, label: String },
    Changes(Vec<Change>),
}

impl Diff {
    pub fn text(old: &str, new: &str, label: impl Into<String>) -> Self {
        Diff::Text {
            old: old.to_string(),
            new: new.to_string(),
            label: label.into(),
        }
    }

    pub fn changes(changes: &[Change]) -> Self {
        Diff::Changes(changes.to_vec())
    }

    /// True when there is nothing to show.
    pub fn is_empty(&self) -> bool {
        match self {
            Diff::Text { old, new, .. } => old == new,
            Diff::Changes(c) => c.is_empty(),
        }
    }
}

impl Render for Diff {
    fn render(&self, mode: &Mode) -> String {
        if self.is_empty() {
            return paint(mode, Token::Muted, "No changes.");
        }
        let width = mode.width;
        match self {
            Diff::Text { old, new, label } => {
                let diff = similar::TextDiff::from_lines(old, new);
                let text = diff
                    .unified_diff()
                    .context_radius(3)
                    .header(&format!("a/{label}"), &format!("b/{label}"))
                    .to_string();
                text.lines()
                    .map(|line| {
                        // File names are never cut; content lines are, like every other line.
                        let line = if line.starts_with("+++") || line.starts_with("---") {
                            line.to_string()
                        } else {
                            truncate(line, width)
                        };
                        let token = if line.starts_with("+++") || line.starts_with("---") {
                            Some(Token::Ink)
                        } else if line.starts_with("@@") {
                            Some(Token::Purple)
                        } else if line.starts_with('+') {
                            Some(Token::Teal)
                        } else if line.starts_with('-') {
                            Some(Token::Coral)
                        } else {
                            None
                        };
                        match token {
                            Some(t) => paint(mode, t, &line),
                            None => line,
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            Diff::Changes(changes) => changes
                .iter()
                .map(|c| {
                    let (sign, token) = match c.kind {
                        ChangeKind::Added => ("+", Token::Teal),
                        ChangeKind::Removed => ("-", Token::Coral),
                        ChangeKind::Changed => ("~", Token::Yellow),
                    };
                    let head = format!("{sign} {}", c.label);
                    let line = match &c.detail {
                        Some(d) => format!("{}  {}", paint(mode, token, &head), paint(mode, Token::Muted, d)),
                        None => paint(mode, token, &head),
                    };
                    truncate(&line, width)
                })
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::ColorLevel;

    #[test]
    fn unified_and_changes() {
        let mode = Mode::fixed(ColorLevel::None, false, 80);
        let out = Diff::text("a\nb\n", "a\nc\n", "config.json").render(&mode);
        assert!(
            out.contains("--- a/config.json") && out.contains("-b") && out.contains("+c"),
            "{out}"
        );
        assert_eq!(Diff::text("x", "x", "f").render(&mode), "No changes.");
        let out =
            Diff::changes(&[Change::added("state qc").detail("QC"), Change::removed("rule r")]).render(&mode);
        assert_eq!(out, "+ state qc  QC\n- rule r");
    }
}
