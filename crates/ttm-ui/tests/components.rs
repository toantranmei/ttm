//! Snapshots of every component in 3 modes (truecolor, `NO_COLOR`, `--plain`) × 2 widths (80, 120 columns).
//!
//! Review changes with `cargo insta review`.

use ttm_api::vocab::{Phase, Tier};
use ttm_ui::{
    Badge, Callout, Cell, Change, Col, ColorLevel, Diff, ErrorView, Header, KeyValue, Mode, Render, Status,
    Table, Token, UsageError,
};

const MODES: [(&str, ColorLevel, bool); 3] = [
    ("color", ColorLevel::TrueColor, false),
    ("nocolor", ColorLevel::None, false),
    ("plain", ColorLevel::None, true),
];

/// Six snapshots of `draw`: `<name>__<mode>_<width>`.
fn matrix(name: &str, draw: impl Fn(&Mode) -> String) {
    for (label, color, plain) in MODES {
        for width in [80, 120] {
            let mode = Mode::fixed(color, plain, width);
            insta::assert_snapshot!(format!("{name}__{label}_{width}"), draw(&mode));
        }
    }
}

#[test]
fn header() {
    matrix("header", |m| {
        [
            Header::new("Aurora · rev 43")
                .summary("12 tasks · 3 in progress · 1 waiting for approval")
                .render(m),
            Header::new("Dự án Cá nhân").render(m),
            Header::new(format!("Very long project name {}", "x".repeat(150)))
                .summary(format!(
                    "A summary that is far longer than the terminal {}",
                    "y".repeat(150)
                ))
                .render(m),
        ]
        .join("\n\n")
    });
}

/// Tasks with Vietnamese, CJK and emoji titles that must be cut with `…`.
const TASKS: [(&str, &str); 7] = [
    (
        "in-progress",
        "Đăng nhập bằng Google cho người dùng mới, kèm xác minh email",
    ),
    ("qc", "登录页面的设计和实现以及更多的测试用例和文档说明"),
    (
        "approval",
        "Ship the release 🚀 with notes 🤖🤖🤖🤖🤖🤖🤖🤖🤖🤖🤖🤖🤖🤖🤖🤖",
    ),
    ("done", "Setup"),
    (
        "blocked",
        "Payment webhook retries are failing on the staging server since Monday",
    ),
    ("paused", "Paused work"),
    ("inbox", "New idea"),
];

#[test]
fn table() {
    matrix("table", |m| {
        let mut table = Table::new(&[
            Col::new(""),
            Col::new("KEY"),
            Col::new("TITLE").flex(3).min(10),
            Col::new("ROLE").max(8),
            Col::new("ASSIGNEE").flex(1).min(6),
            Col::new("EST").right(),
        ]);
        for (i, (category, title)) in TASKS.iter().enumerate() {
            table.push(vec![
                Cell::new(Status::of_category(category).render(m)),
                Cell::token(format!("AUR-{}", 12 + i), Token::Ink),
                Cell::new(*title),
                Cell::new(if i % 2 == 0 { "developer" } else { "qa" }),
                Cell::new(format!("{} Nguyễn Văn An", Badge::person().render(m))),
                Cell::new(format!("{}", i + 1)),
            ]);
        }
        [
            table.render(m),
            Table::new(&[Col::new("KEY")]).empty_text("No tasks.").render(m),
        ]
        .join("\n\n")
    });
}

#[test]
fn status_and_badges() {
    matrix("status_badge", |m| {
        let phases: Vec<String> = Phase::ALL.iter().map(|p| Status::phase(*p).render(m)).collect();
        let checks = [Status::ok(), Status::warn(), Status::fail()].map(|s| s.render(m));
        let badges = [
            Badge::agent(),
            Badge::person(),
            Badge::member(Tier::Supporter),
            Badge::member(Tier::Meier),
            Badge::member(Tier::Sponsor),
            Badge::text(Token::Purple, "review"),
        ]
        .map(|b| b.render(m));
        format!(
            "phases: {}\nwaiting: {}\nchecks: {}\nbadges: {}",
            phases.join(" "),
            Status::of_category("approval").render(m),
            checks.join(" "),
            badges.join(" ")
        )
    });
}

#[test]
fn key_value() {
    matrix("key_value", |m| {
        KeyValue::new()
            .row("user", "An <an@example.com>")
            .row("host", "https://toantranmei.com")
            .row("profile", "default")
            .row(
                "workspace",
                format!("Team Aurora (w2) {}", Badge::member(Tier::Meier).render(m)),
            )
            .row("note", format!("Một giá trị rất dài {}", "z".repeat(150)))
            .render(m)
    });
}

#[test]
fn callout() {
    matrix("callout", |m| {
        [
            Callout::success("Signed in as An")
                .detail("host: https://toantranmei.com")
                .next("ttm project init")
                .render(m),
            Callout::warning("2 conflicts with the server")
                .next("ttm sync --resolve")
                .render(m),
            Callout::error("Run failed").detail("exit 1").render(m),
            Callout::info("No workspace chosen")
                .next("ttm workspace switch <name>")
                .render(m),
        ]
        .join("\n\n")
    });
}

#[test]
fn diff() {
    matrix("diff", |m| {
        let old = "{\n  \"mcpServers\": {\n    \"other\": {}\n  }\n}\n";
        let new =
            "{\n  \"mcpServers\": {\n    \"other\": {},\n    \"ttm\": {\"command\": \"ttm-mcp\"}\n  }\n}\n";
        [
            Diff::text(old, new, "claude_desktop_config.json").render(m),
            Diff::changes(&[
                Change::added("state qc").detail("QC · role qa"),
                Change::removed("transition ready → done"),
                Change::changed("rule auto-assign").detail("trigger moved"),
            ])
            .render(m),
            Diff::changes(&[]).render(m),
        ]
        .join("\n\n")
    });
}

#[test]
fn errors() {
    matrix("error", |m| {
        let problem = ttm_api::Problem {
            status: 409,
            code: "conflict".into(),
            title: "AUR-12 changed since rev 4.".into(),
            detail: None,
            field: Some("rev".into()),
            rule: Some("rev-conflict".into()),
            params: None,
            required: None,
        };
        let api =
            anyhow::Error::new(ttm_api::ApiError::Problem(Box::new(problem))).context("updating AUR-12");
        let usage = anyhow::Error::new(UsageError::missing_flag("--workflow", true));
        let confirm = anyhow::Error::new(UsageError::confirmation_required("Publishing the workflow"));
        let plain = anyhow::anyhow!("cannot read .ttm/project.toml");
        [api, usage, confirm, plain]
            .iter()
            .map(|e| ErrorView::from_anyhow(e).render(m))
            .collect::<Vec<_>>()
            .join("\n\n")
    });
}
