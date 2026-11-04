//! `KeyValue`: keys right-aligned and muted, values cut to the width. `--plain`: `key: value`.

use crate::Render;
use crate::mode::Mode;
use crate::theme::{Token, paint};
use crate::width::{Align, display_width, pad, truncate};

#[derive(Debug, Clone, Default)]
pub struct KeyValue {
    rows: Vec<(String, String)>,
}

impl KeyValue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn row(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.push(key, value);
        self
    }

    /// Adds the row only when there is a value.
    pub fn opt(self, key: impl Into<String>, value: Option<impl Into<String>>) -> Self {
        match value {
            Some(v) => self.row(key, v),
            None => self,
        }
    }

    pub fn push(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.rows.push((key.into(), value.into()));
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

impl Render for KeyValue {
    fn render(&self, mode: &Mode) -> String {
        if mode.plain {
            return self
                .rows
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{k}: {}",
                        truncate(v, mode.width.saturating_sub(display_width(k) + 2))
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
        }
        let key_w = self.rows.iter().map(|(k, _)| display_width(k)).max().unwrap_or(0);
        let value_w = mode.width.saturating_sub(key_w + 2).max(8);
        self.rows
            .iter()
            .map(|(k, v)| {
                format!(
                    "{}  {}",
                    paint(mode, Token::Muted, &pad(k, key_w, Align::Right)),
                    truncate(v, value_w)
                )
                .trim_end()
                .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::ColorLevel;

    #[test]
    fn aligned_keys() {
        let kv = KeyValue::new().row("user", "An").row("workspace", "Aurora");
        let mode = Mode::fixed(ColorLevel::None, false, 80);
        assert_eq!(kv.render(&mode), "     user  An\nworkspace  Aurora");
        let plain = Mode::fixed(ColorLevel::None, true, 80);
        assert_eq!(kv.render(&plain), "user: An\nworkspace: Aurora");
    }
}
