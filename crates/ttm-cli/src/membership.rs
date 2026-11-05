//! Mei-er in the CLI (F35): a gold thank-you line for members in `login`/`whoami`, an anniversary wish, and one muted
//! support line in `whoami` only for people who are not members. No other command ever mentions it.

use serde::Serialize;
use time::Date;
use ttm_api::Membership;
use ttm_api::vocab::Tier;
use ttm_ui::{Badge, Token, Ui};

pub const SUPPORT_URL: &str = "https://toantranmei.com/mei-er";

/// `meta` of `whoami --json` / `login --json`.
#[derive(Debug, Serialize)]
pub struct Meta {
    pub membership: Option<Membership>,
}

/// The thank-you of each tier.
pub fn thanks(tier: Tier) -> &'static str {
    match tier {
        Tier::Supporter => "Thank you for supporting toantranmei.",
        Tier::Meier => "Thank you for being a Mei-er. You keep this going.",
        Tier::Sponsor => "Thank you, Sponsor. This work exists because of you.",
    }
}

/// `YYYY-MM-DD…` → date.
fn date_of(rfc3339: &str) -> Option<Date> {
    let year: i32 = rfc3339.get(..4)?.parse().ok()?;
    let month: u8 = rfc3339.get(5..7)?.parse().ok()?;
    let day: u8 = rfc3339.get(8..10)?.parse().ok()?;
    Date::from_calendar_date(year, time::Month::try_from(month).ok()?, day).ok()
}

/// Whole years since `since` when today is the same month and day, at least one year later.
pub fn anniversary(since: Date, today: Date) -> Option<i32> {
    let years = today.year() - since.year();
    (years >= 1 && since.month() == today.month() && since.day() == today.day()).then_some(years)
}

/// Lines to print after the result of `login`/`whoami`. `whoami` also shows the support line to non-members.
pub fn lines(ui: &Ui, membership: Option<&Membership>, today: Date, whoami: bool) -> Vec<String> {
    let Some(m) = membership else {
        return if whoami {
            vec![ui.paint(Token::Muted, &format!("Support toantranmei → {SUPPORT_URL}"))]
        } else {
            vec![]
        };
    };
    let Some(tier) = m.tier() else {
        return vec![];
    };
    let since = m.since.as_deref().and_then(date_of);
    let badge = ui.render(&Badge::member(tier));
    let mut first = match since {
        Some(d) => format!(
            "{badge} {}",
            ui.paint(Token::Gold, &format!("since {}-{:02}", d.year(), d.month() as u8))
        ),
        None => badge,
    };
    first.push_str(&format!("  {}", thanks(tier)));
    let mut out = vec![first];
    if let Some(n) = since.and_then(|d| anniversary(d, today)) {
        out.push(ui.paint(Token::Gold, &format!("Happy {n}-year Mei-er anniversary!")));
    }
    out
}

/// Today in UTC.
pub fn today() -> Date {
    time::OffsetDateTime::now_utc().date()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ttm_ui::{ColorLevel, Mode};

    fn m(tier: &str, since: &str) -> Membership {
        Membership {
            tier: tier.into(),
            status: Some("active".into()),
            since: Some(since.into()),
            until: None,
            badge: None,
        }
    }

    fn day(s: &str) -> Date {
        date_of(s).unwrap()
    }

    #[test]
    fn tiers_anniversary_and_non_members() {
        let (ui, _) = Ui::buffered(Mode::fixed(ColorLevel::None, false, 80));
        let l = lines(
            &ui,
            Some(&m("mei-er", "2026-11-01T00:00:00Z")),
            day("2026-11-05"),
            true,
        );
        assert_eq!(
            l,
            ["✦ Mei-er since 2026-11  Thank you for being a Mei-er. You keep this going."]
        );
        let l = lines(
            &ui,
            Some(&m("sponsor", "2024-11-05T00:00:00Z")),
            day("2026-11-05"),
            false,
        );
        assert_eq!(l[1], "Happy 2-year Mei-er anniversary!");
        let l = lines(
            &ui,
            Some(&m("supporter", "2026-01-02T00:00:00Z")),
            day("2026-11-05"),
            false,
        );
        assert!(l[0].starts_with("· Supporter since 2026-01  Thank you for supporting"));
        assert_eq!(
            lines(&ui, None, day("2026-11-05"), true),
            [format!("Support toantranmei → {SUPPORT_URL}")]
        );
        assert!(
            lines(&ui, None, day("2026-11-05"), false).is_empty(),
            "only whoami"
        );
        let (plain, _) = Ui::buffered(Mode::fixed(ColorLevel::None, true, 80));
        let l = lines(
            &plain,
            Some(&m("mei-er", "2026-11-01T00:00:00Z")),
            day("2026-11-05"),
            true,
        );
        assert!(l[0].starts_with("[Mei-er] since 2026-11"));
        assert_eq!(
            anniversary(day("2026-11-05"), day("2026-11-05")),
            None,
            "same year"
        );
    }
}
