//! `Badge`: who someone is (`🤖` agent, `👤` person) and Mei-er membership (`✦ Mei-er`, gold).

use crate::Render;
use crate::mode::Mode;
use crate::theme::{Token, paint};
use ttm_api::vocab::{self, Tier};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Badge {
    Agent,
    Person,
    Member(Tier),
    Text(Token, String),
}

impl Badge {
    pub fn agent() -> Self {
        Badge::Agent
    }

    pub fn person() -> Self {
        Badge::Person
    }

    /// The Mei-er badge of `tier`, in gold.
    pub fn member(tier: Tier) -> Self {
        Badge::Member(tier)
    }

    /// The badge of a `membership.tier` string from the API (`None` for unknown tiers or no membership).
    pub fn of_tier(tier: Option<&str>) -> Option<Self> {
        tier.and_then(Tier::parse).map(Badge::Member)
    }

    pub fn text(token: Token, label: impl Into<String>) -> Self {
        Badge::Text(token, label.into())
    }
}

impl Render for Badge {
    fn render(&self, mode: &Mode) -> String {
        match (self, mode.plain) {
            (Badge::Agent, false) => vocab::AGENT_BADGE.to_string(),
            (Badge::Agent, true) => vocab::AGENT_PLAIN.to_string(),
            (Badge::Person, false) => vocab::PERSON_BADGE.to_string(),
            (Badge::Person, true) => vocab::PERSON_PLAIN.to_string(),
            (Badge::Member(tier), false) => paint(mode, Token::Gold, tier.badge()),
            (Badge::Member(tier), true) => tier.plain().to_string(),
            (Badge::Text(token, label), _) => paint(mode, *token, label),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::ColorLevel;

    #[test]
    fn plain_words() {
        let plain = Mode::fixed(ColorLevel::None, true, 80);
        let fancy = Mode::fixed(ColorLevel::None, false, 80);
        assert_eq!(Badge::agent().render(&plain), "[agent]");
        assert_eq!(Badge::agent().render(&fancy), "🤖");
        assert_eq!(Badge::member(Tier::Meier).render(&fancy), "✦ Mei-er");
        assert_eq!(
            Badge::of_tier(Some("sponsor")).unwrap().render(&plain),
            "[Sponsor]"
        );
        assert!(Badge::of_tier(Some("gold-plated")).is_none());
    }
}
