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

/// Errors go to stderr; with `--json` a `{"error": {...}}` object also goes to stdout for scripts.
pub fn print_error(json: bool, err: &anyhow::Error) {
    if json {
        let code = err
            .downcast_ref::<ttm_api::ApiError>()
            .map(|e| e.code().to_string())
            .unwrap_or_else(|| "error".into());
        let body = serde_json::json!({ "error": { "code": code, "message": format!("{err:#}") } });
        println!("{}", serde_json::to_string_pretty(&body).unwrap_or_default());
    }
    eprintln!("error: {err:#}");
}
