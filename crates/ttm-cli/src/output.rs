//! Printing: `--json` for scripts (the only thing on stdout then), otherwise people through `ttm_ui::Ui`.

use crate::app::App;
use serde::Serialize;
use std::process::ExitCode;
use ttm_ui::{ErrorView, Ui};

/// Prints `value` as pretty JSON with `--json` (and nothing else on stdout), else draws `human` through the Ui.
pub fn emit<T: Serialize>(app: &App, value: &T, human: impl FnOnce(&Ui)) {
    emit_to(&app.ui, app.json, value, human)
}

/// [`emit`] without an `App` (commands that run before the configuration is loaded).
pub fn emit_to<T: Serialize>(ui: &Ui, json: bool, value: &T, human: impl FnOnce(&Ui)) {
    if json {
        match serde_json::to_string_pretty(value) {
            Ok(text) => ui.raw(&format!("{text}\n")),
            Err(err) => ui.eline(&format!("error: cannot encode JSON: {err}")),
        }
    } else {
        human(ui);
    }
}

/// A spinner on stderr; when none can be drawn (not a terminal, `--plain`), the message is printed once instead so a
/// person reading a log still sees what ttm waits for. Nothing with `--json`.
pub fn waiting(ui: &Ui, message: &str) -> ttm_ui::Spinner {
    let spinner = ui.spinner(message);
    if !spinner.visible() && !ui.mode().json {
        ui.eline(&ui.paint(ttm_ui::Token::Muted, message));
    }
    spinner
}

#[allow(dead_code)] // `agent logs -f`, `inbox --watch`
/// One JSON value per line (`--json` streams: `agent logs -f`, `inbox --watch`).
pub fn emit_line<T: Serialize>(ui: &Ui, value: &T) {
    match serde_json::to_string(value) {
        Ok(text) => ui.raw(&format!("{text}\n")),
        Err(err) => ui.eline(&format!("error: cannot encode JSON: {err}")),
    }
}

/// Errors go to stderr as `✕ <summary>` / `rule` / `Fix` / `Help`; with `--json` a `{"error": {...}}` object also
/// goes to stdout for scripts. A server problem keeps its `status`, `rule`, `field` and `params` there, so scripts
/// can branch on `rule` (`split-limit`, `rev-conflict`, `approval-required`) or read `params.required` of
/// `insufficient_scope`. Returns the exit code: 2 for usage errors (`missing_flag`, `confirmation_required`,
/// `unsupported_feature`, `invalid_flag`), else 1.
pub fn print_error(ui: &Ui, json: bool, err: &anyhow::Error) -> ExitCode {
    if json {
        let body = serde_json::json!({ "error": error_json(err) });
        ui.raw(&format!(
            "{}\n",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        ));
    }
    let view = ErrorView::from_anyhow(err);
    ui.eprint(&view);
    ExitCode::from(view.exit_code())
}

/// `{code, message, status?, rule?, field?, params?}`.
pub fn error_json(err: &anyhow::Error) -> serde_json::Value {
    let api = err.chain().find_map(|e| e.downcast_ref::<ttm_api::ApiError>());
    let usage = err.chain().find_map(|e| e.downcast_ref::<ttm_ui::UsageError>());
    let code = match (api, usage) {
        (Some(api), _) => api.code().to_string(),
        (None, Some(usage)) => usage.rule.to_string(),
        (None, None) => "error".into(),
    };
    let mut out = serde_json::json!({
        "code": code,
        "message": format!("{err:#}"),
    });
    if let Some(usage) = usage {
        out["rule"] = usage.rule.into();
        if let Some(field) = &usage.field {
            out["field"] = field.as_str().into();
        }
    }
    if let Some(api) = api {
        if let Some(status) = api.status() {
            out["status"] = status.into();
        }
        if let Some(rule) = api.rule() {
            out["rule"] = rule.into();
        }
        if let Some(field) = api.field() {
            out["field"] = field.into();
        }
        if let Some(params) = api.params() {
            out["params"] = serde_json::Value::Object(params.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_error_keeps_rule_field_params() {
        let problem = ttm_api::Problem {
            status: 409,
            code: "conflict".into(),
            title: "AUR-12 changed since rev 4.".into(),
            detail: None,
            field: Some("rev".into()),
            rule: Some("rev-conflict".into()),
            params: Some(serde_json::json!({"current": 5}).as_object().unwrap().clone()),
            required: None,
        };
        let err =
            anyhow::Error::new(ttm_api::ApiError::Problem(Box::new(problem))).context("updating AUR-12");
        let v = error_json(&err);
        assert_eq!(
            (
                &v["code"],
                &v["status"],
                &v["rule"],
                &v["field"],
                &v["params"]["current"]
            ),
            (
                &"conflict".into(),
                &409.into(),
                &"rev-conflict".into(),
                &"rev".into(),
                &5.into()
            )
        );
        assert!(v["message"].as_str().unwrap().contains("rule `rev-conflict`"));
        assert_eq!(error_json(&anyhow::anyhow!("boom"))["code"], "error");
    }
}
