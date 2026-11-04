//! Wizard: flags win over questions, `--yes` takes defaults, missing flags fail without a terminal, and the
//! equivalent command lists every answer.

use ttm_ui::{Answer, Ask, Choice, ColorLevel, ErrorView, Mode, ScriptedTerm, Ui, Wizard};

fn ui() -> (Ui, ttm_ui::SharedBuf) {
    Ui::buffered(Mode::fixed(ColorLevel::None, false, 80))
}

fn workflows() -> Vec<Choice<String>> {
    ["professional", "balanced", "kanban"]
        .map(|w| Choice::new(w.to_string(), w).hint("workflow"))
        .to_vec()
}

fn rule_of(err: &anyhow::Error) -> (Option<String>, Option<String>, u8) {
    let v = ErrorView::from_anyhow(err);
    let code = v.exit_code();
    (v.rule, v.field, code)
}

#[allow(clippy::ptr_arg)] // validators take `&T`
fn key_check(v: &String) -> Result<(), String> {
    if v.len() >= 2 && v.chars().all(|c| c.is_ascii_uppercase()) {
        Ok(())
    } else {
        Err("use 2 or more capital letters, e.g. AUR".into())
    }
}

#[test]
fn flag_wins_over_question() {
    let (ui, _) = ui();
    let term = ScriptedTerm::new([]);
    let mut w = Wizard::with_term(&ui, false, true, Box::new(term.clone()));
    let name = w
        .text(Ask::new("Project name", "--name").value(Some("Web todo".into())))
        .unwrap();
    let wf = w
        .select(
            Ask::new("Workflow", "--workflow").value(Some("kanban".into())),
            &workflows(),
        )
        .unwrap();
    assert_eq!((name.as_str(), wf.as_str()), ("Web todo", "kanban"));
    assert!(term.asked().is_empty(), "nothing asked: {:?}", term.asked());
}

#[test]
fn flag_value_is_validated() {
    let (ui, _) = ui();
    let mut w = Wizard::with_term(&ui, false, true, Box::new(ScriptedTerm::new([])));
    let err = w
        .text(
            Ask::new("Key", "--key")
                .value(Some("aur".into()))
                .validate(key_check),
        )
        .unwrap_err();
    assert_eq!(
        rule_of(&err),
        (Some("invalid_flag".into()), Some("--key".into()), 2)
    );
}

#[test]
fn yes_takes_the_default_or_fails() {
    let (ui, _) = ui();
    let mut w = Wizard::with_term(&ui, true, true, Box::new(ScriptedTerm::new([])));
    let wf = w
        .select(
            Ask::new("Workflow", "--workflow").default("balanced".to_string()),
            &workflows(),
        )
        .unwrap();
    assert_eq!(wf, "balanced");
    let err = w.text(Ask::new("Task key prefix", "--key")).unwrap_err();
    assert_eq!(
        rule_of(&err),
        (Some("missing_flag".into()), Some("--key".into()), 2)
    );
    assert!(w.confirm("Create the project?").unwrap(), "--yes confirms");
}

#[test]
fn no_terminal_needs_flags() {
    let (ui, _) = ui();
    let mut w = Wizard::with_term(&ui, false, false, Box::new(ScriptedTerm::new([])));
    let err = w
        .select(
            Ask::new("Workflow", "--workflow").default("balanced".to_string()),
            &workflows(),
        )
        .unwrap_err();
    let view = ErrorView::from_anyhow(&err);
    assert_eq!(view.rule.as_deref(), Some("missing_flag"));
    assert_eq!(view.fix.as_deref(), Some("pass --workflow or run in a terminal"));
    let err = w.confirm("Publish version 3?").unwrap_err();
    let view = ErrorView::from_anyhow(&err);
    assert_eq!(view.rule.as_deref(), Some("confirmation_required"));
    assert_eq!(view.next.as_deref(), Some("rerun with --yes"));
    assert_eq!(view.exit_code(), 2);
}

#[test]
fn asks_in_a_terminal_and_retries_invalid_answers() {
    let (ui, buf) = ui();
    let term = ScriptedTerm::new([
        Answer::Text("aur".into()),
        Answer::Text("AUR".into()),
        Answer::Select(2),
        Answer::Multi(vec![0, 2]),
        Answer::Confirm(true),
    ]);
    let mut w = Wizard::with_term(&ui, false, true, Box::new(term.clone()));
    let key = w
        .text(Ask::new("Task key prefix", "--key").validate(key_check))
        .unwrap();
    let wf = w
        .select(Ask::new("Workflow", "--workflow"), &workflows())
        .unwrap();
    let skills = w
        .multi_select(Ask::new("Skills", "--skills"), &workflows())
        .unwrap();
    assert!(w.confirm("Create it?").unwrap());
    assert_eq!((key.as_str(), wf.as_str()), ("AUR", "kanban"));
    assert_eq!(skills, ["professional", "kanban"]);
    assert_eq!(
        term.asked(),
        [
            "Task key prefix",
            "Task key prefix",
            "Workflow",
            "Skills",
            "Create it?"
        ]
    );
    assert!(
        buf.stderr().contains("✕ use 2 or more capital letters"),
        "{}",
        buf.stderr()
    );
}

#[test]
fn secrets_come_from_the_environment() {
    let (ui, _) = ui();
    let mut w = Wizard::with_term(&ui, true, false, Box::new(ScriptedTerm::new([])))
        .with_env(|name| (name == "LINEAR_TOKEN").then(|| "s3cret".to_string()));
    let secret = w
        .secret_from_env("Secret", "--secret-env", Some("LINEAR_TOKEN"))
        .unwrap();
    assert_eq!(secret, "s3cret");
    assert!(
        w.equivalent("mcp-server add")
            .contains("--secret-env LINEAR_TOKEN")
    );
    assert!(!w.equivalent("mcp-server add").contains("s3cret"));
    let err = w
        .secret_from_env("Secret", "--secret-env", Some("UNSET"))
        .unwrap_err();
    assert_eq!(rule_of(&err).0.as_deref(), Some("missing_flag"));
}

#[test]
fn equivalent_line() {
    let (ui, buf) = ui();
    let term = ScriptedTerm::new([Answer::Text("My app".into()), Answer::Select(0)]);
    let mut w = Wizard::with_term(&ui, false, true, Box::new(term));
    w.text(Ask::new("Project name", "--name")).unwrap();
    w.text(Ask::new("Task key prefix", "--key").value(Some("AUR".into())))
        .unwrap();
    w.select(Ask::new("Workflow", "--workflow"), &workflows())
        .unwrap();
    w.record("--runner", Some("none".into()));
    let expected = "ttm project init --name 'My app' --key AUR --workflow professional --runner none --yes";
    assert_eq!(w.equivalent("project init"), expected);
    w.print_equivalent("project init");
    assert_eq!(buf.stderr(), format!("Equivalent: {expected}\n"));

    // Everything from flags: nothing to print.
    let (ui, buf) = self::ui();
    let mut w = Wizard::with_term(&ui, false, true, Box::new(ScriptedTerm::new([])));
    w.text(Ask::new("Project name", "--name").value(Some("x".into())))
        .unwrap();
    w.print_equivalent("project init");
    assert_eq!(buf.stderr(), "");
}
