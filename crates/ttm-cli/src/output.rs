//! Printing: human text or `--json`.

use serde::Serialize;

/// Prints `value` as pretty JSON when `json`, else runs `human`.
pub fn emit<T: Serialize>(json: bool, value: &T, human: impl FnOnce()) {
    if json {
        match serde_json::to_string_pretty(value) {
            Ok(text) => println!("{text}"),
            Err(err) => eprintln!("error: cannot encode JSON: {err}"),
        }
    } else {
        human();
    }
}

/// Errors go to stderr; with `--json` a `{"error": {...}}` object also goes to stdout for scripts. A server problem
/// keeps its `status`, `rule`, `field` and `params` there, so scripts can branch on `rule` (`split-limit`,
/// `rev-conflict`, `approval-required`) or read `params.required` of `insufficient_scope`.
pub fn print_error(json: bool, err: &anyhow::Error) {
    if json {
        let body = serde_json::json!({ "error": error_json(err) });
        println!("{}", serde_json::to_string_pretty(&body).unwrap_or_default());
    }
    eprintln!("error: {err:#}");
}

/// `{code, message, status?, rule?, field?, params?}`.
pub fn error_json(err: &anyhow::Error) -> serde_json::Value {
    let api = err.chain().find_map(|e| e.downcast_ref::<ttm_api::ApiError>());
    let mut out = serde_json::json!({
        "code": api.map(|e| e.code().to_string()).unwrap_or_else(|| "error".into()),
        "message": format!("{err:#}"),
    });
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
