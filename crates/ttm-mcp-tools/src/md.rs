//! The one Markdown shape of every tool result, mirroring the CLI: a bold title line with a glyph, then tables or
//! key/value lists, then `**Next:**` with the tool to call next. Glyphs and badges come from `ttm_api::vocab`, so
//! the CLI and the MCP tools say the same thing.

use ttm_api::vocab::{self, Phase, Tier};

/// Longest table cell, in characters.
pub const MAX_CELL: usize = 200;

/// Markdown builder.
#[derive(Debug, Clone, Default)]
pub struct Md {
    blocks: Vec<String>,
}

/// `|` escaped, newlines as spaces, cut at [`MAX_CELL`] characters with `…`.
pub fn cell(text: &str) -> String {
    let one_line = text.replace(['\r', '\n'], " ").replace('|', "\\|");
    if one_line.chars().count() > MAX_CELL {
        let cut: String = one_line.chars().take(MAX_CELL - 1).collect();
        format!("{cut}…")
    } else {
        one_line
    }
}

/// The glyph of a workflow category (`○ ● ◐ ✓ ✕ ⏸`).
pub fn glyph(category: &str) -> &'static str {
    vocab::phase_of_category(category).glyph()
}

/// The glyph of a phase.
pub fn phase(phase: Phase) -> &'static str {
    phase.glyph()
}

/// ` ✦ Mei-er` (with a leading space) for a member tier, else empty.
pub fn badge(tier: Option<&str>) -> String {
    tier.and_then(Tier::parse)
        .map(|t| format!(" {}", t.badge()))
        .unwrap_or_default()
}

/// `🤖 name` / `👤 name` plus the Mei-er badge.
pub fn who(kind: &str, name: &str, tier: Option<&str>) -> String {
    let icon = if kind == "agent" {
        vocab::AGENT_BADGE
    } else {
        vocab::PERSON_BADGE
    };
    format!("{icon} {name}{}", badge(tier))
}

impl Md {
    /// Starts with the title line `**<glyph> <text>**`.
    pub fn title(glyph: &str, text: impl AsRef<str>) -> Self {
        let text = text.as_ref().replace('\n', " ");
        let line = if glyph.is_empty() {
            format!("**{text}**")
        } else {
            format!("**{glyph} {text}**")
        };
        Self { blocks: vec![line] }
    }

    /// A table; nothing when there are no rows.
    pub fn table<S: AsRef<str>>(mut self, headers: &[&str], rows: &[Vec<S>]) -> Self {
        if rows.is_empty() {
            return self;
        }
        let mut out = format!(
            "| {} |\n|{}|",
            headers.iter().map(|h| cell(h)).collect::<Vec<_>>().join(" | "),
            headers.iter().map(|_| " --- ").collect::<Vec<_>>().join("|")
        );
        for row in rows {
            out.push_str("\n| ");
            out.push_str(
                &(0..headers.len())
                    .map(|i| row.get(i).map(|c| cell(c.as_ref())).unwrap_or_default())
                    .collect::<Vec<_>>()
                    .join(" | "),
            );
            out.push_str(" |");
        }
        self.blocks.push(out);
        self
    }

    /// `- **k:** v` lines; empty values are skipped.
    pub fn kv<K: AsRef<str>, V: AsRef<str>>(mut self, pairs: &[(K, V)]) -> Self {
        let lines: Vec<String> = pairs
            .iter()
            .filter(|(_, v)| !v.as_ref().trim().is_empty())
            .map(|(k, v)| format!("- **{}:** {}", k.as_ref(), v.as_ref().replace('\n', " ")))
            .collect();
        if !lines.is_empty() {
            self.blocks.push(lines.join("\n"));
        }
        self
    }

    /// A paragraph (kept as is).
    pub fn text(mut self, paragraph: impl AsRef<str>) -> Self {
        let p = paragraph.as_ref().trim_end();
        if !p.is_empty() {
            self.blocks.push(p.to_string());
        }
        self
    }

    /// `**Next:** \`tool\` — why; \`other\` — why`.
    pub fn next(mut self, steps: &[(&str, &str)]) -> Self {
        if steps.is_empty() {
            return self;
        }
        let parts: Vec<String> = steps
            .iter()
            .map(|(tool, why)| {
                if why.is_empty() {
                    format!("`{tool}`")
                } else {
                    format!("`{tool}` — {why}")
                }
            })
            .collect();
        self.blocks.push(format!("**Next:** {}", parts.join("; ")));
        self
    }

    /// Appends the blocks of another builder (its title becomes a bold line inside this result).
    pub fn extend(mut self, other: Md) -> Self {
        self.blocks.extend(other.blocks);
        self
    }

    pub fn build(self) -> String {
        self.blocks.join("\n\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape() {
        let md = Md::title("✓", "AUR-12 moved to Review")
            .table(&["Key", "Title"], &[vec!["AUR-12", "a | b\nc"]])
            .kv(&[("state", "Review"), ("empty", "")])
            .next(&[("factory-get-task", "read it again")])
            .build();
        assert_eq!(
            md,
            "**✓ AUR-12 moved to Review**\n\n| Key | Title |\n| --- | --- |\n| AUR-12 | a \\| b c |\n\n- **state:** Review\n\n\
             **Next:** `factory-get-task` — read it again"
        );
        assert_eq!(cell(&"x".repeat(300)).chars().count(), MAX_CELL);
        assert_eq!(badge(Some("sponsor")), " ✦✦ Sponsor");
        assert_eq!(who("agent", "Dev", None), "🤖 Dev");
        assert_eq!(glyph("qc"), "◐");
    }
}
