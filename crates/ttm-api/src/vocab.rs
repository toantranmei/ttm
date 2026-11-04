//! Words and glyphs shared by every human-facing output of ttm: the CLI (`ttm-ui`) and the Markdown of the MCP tools
//! (`ttm-mcp-tools`, vendored into the server). No terminal dependency lives here, so the server can use it too.

/// Where a task stands, whatever the workflow calls its state. One glyph and one plain word each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    Todo,
    Doing,
    Review,
    Done,
    Blocked,
    Paused,
}

impl Phase {
    pub const ALL: [Phase; 6] = [
        Phase::Todo,
        Phase::Doing,
        Phase::Review,
        Phase::Done,
        Phase::Blocked,
        Phase::Paused,
    ];

    /// `○ ● ◐ ✓ ✕ ⏸`.
    pub fn glyph(self) -> &'static str {
        match self {
            Phase::Todo => "○",
            Phase::Doing => "●",
            Phase::Review => "◐",
            Phase::Done => "✓",
            Phase::Blocked => "✕",
            Phase::Paused => "⏸",
        }
    }

    /// What `--plain` prints instead of the glyph.
    pub fn word(self) -> &'static str {
        match self {
            Phase::Todo => "todo",
            Phase::Doing => "doing",
            Phase::Review => "review",
            Phase::Done => "done",
            Phase::Blocked => "blocked",
            Phase::Paused => "paused",
        }
    }
}

/// The phase of a workflow category (`inbox`, `refinement`, `ready`, `ux-design`, `in-progress`, `design-review`,
/// `qc`, `approval`, `done`) or of `blocked`/`paused`. Older names (`backlog`, `review`, `todo`, `doing`) are
/// understood as well; `_` and case do not matter. Anything unknown is `Todo`.
pub fn phase_of_category(category: &str) -> Phase {
    let key = category.trim().to_ascii_lowercase().replace('_', "-");
    match key.as_str() {
        "in-progress" | "ux-design" | "doing" | "running" => Phase::Doing,
        "design-review" | "qc" | "approval" | "review" | "waiting" => Phase::Review,
        "done" | "succeeded" | "closed" => Phase::Done,
        "blocked" | "failed" | "error" => Phase::Blocked,
        "paused" | "on-hold" => Phase::Paused,
        _ => Phase::Todo,
    }
}

/// True for the categories that wait on someone else (shown in yellow rather than the review purple).
pub fn is_waiting_category(category: &str) -> bool {
    matches!(
        category.trim().to_ascii_lowercase().replace('_', "-").as_str(),
        "approval" | "waiting"
    )
}

/// Membership tier of toantranmei.com (Mei-er programme).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tier {
    Supporter,
    Meier,
    Sponsor,
}

impl Tier {
    /// `supporter`, `mei-er` (also `meier`), `sponsor`.
    pub fn parse(value: &str) -> Option<Tier> {
        match value.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "supporter" => Some(Tier::Supporter),
            "mei-er" | "meier" | "member" => Some(Tier::Meier),
            "sponsor" => Some(Tier::Sponsor),
            _ => None,
        }
    }

    /// `· Supporter`, `✦ Mei-er`, `✦✦ Sponsor` (gold in the terminal).
    pub fn badge(self) -> &'static str {
        match self {
            Tier::Supporter => "· Supporter",
            Tier::Meier => "✦ Mei-er",
            Tier::Sponsor => "✦✦ Sponsor",
        }
    }

    /// `[Supporter]`, `[Mei-er]`, `[Sponsor]` for `--plain`.
    pub fn plain(self) -> &'static str {
        match self {
            Tier::Supporter => "[Supporter]",
            Tier::Meier => "[Mei-er]",
            Tier::Sponsor => "[Sponsor]",
        }
    }

    /// Name without decoration.
    pub fn name(self) -> &'static str {
        match self {
            Tier::Supporter => "Supporter",
            Tier::Meier => "Mei-er",
            Tier::Sponsor => "Sponsor",
        }
    }
}

pub const AGENT_BADGE: &str = "🤖";
pub const PERSON_BADGE: &str = "👤";
pub const AGENT_PLAIN: &str = "[agent]";
pub const PERSON_PLAIN: &str = "[person]";

/// Help center root of the error codes page.
pub const HELP_ERRORS: &str = "https://toantranmei.com/help/ttm/ma-loi";

/// `https://toantranmei.com/help/ttm/ma-loi#<rule>`.
pub fn help_url(rule: &str) -> String {
    format!("{HELP_ERRORS}#{}", rule.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categories_map_to_phases() {
        for c in ["inbox", "refinement", "ready", "backlog"] {
            assert_eq!(phase_of_category(c), Phase::Todo, "{c}");
        }
        for c in ["ux-design", "in-progress", "In_Progress"] {
            assert_eq!(phase_of_category(c), Phase::Doing, "{c}");
        }
        for c in ["design-review", "qc", "approval", "review"] {
            assert_eq!(phase_of_category(c), Phase::Review, "{c}");
        }
        assert_eq!(phase_of_category("done"), Phase::Done);
        assert_eq!(phase_of_category("blocked"), Phase::Blocked);
        assert_eq!(phase_of_category("paused"), Phase::Paused);
        assert_eq!(phase_of_category("whatever"), Phase::Todo);
        assert!(is_waiting_category("approval"));
        assert!(!is_waiting_category("qc"));
    }

    #[test]
    fn glyphs_words_badges() {
        let glyphs: Vec<_> = Phase::ALL.iter().map(|p| p.glyph()).collect();
        assert_eq!(glyphs, ["○", "●", "◐", "✓", "✕", "⏸"]);
        let words: Vec<_> = Phase::ALL.iter().map(|p| p.word()).collect();
        assert_eq!(words, ["todo", "doing", "review", "done", "blocked", "paused"]);
        assert_eq!(Tier::Meier.badge(), "✦ Mei-er");
        assert_eq!(Tier::Sponsor.plain(), "[Sponsor]");
        assert_eq!(Tier::parse("mei_er"), Some(Tier::Meier));
        assert_eq!(
            help_url("rev-conflict"),
            "https://toantranmei.com/help/ttm/ma-loi#rev-conflict"
        );
    }
}
