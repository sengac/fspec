//! PROV-094: sandbox building blocks for subscription scripts —
//! `http::post_status` (an HTTP error status is a result, not a throw) and
//! `time::now_secs` (token expiry).

use rhai::{Dynamic, Map, Module};

use super::engine::RhaiModule;

fn runtime_error(message: String) -> Box<rhai::EvalAltResult> {
    Box::new(rhai::EvalAltResult::ErrorRuntime(
        message.into(),
        rhai::Position::NONE,
    ))
}

/// `http::post_status(url, body, headers) -> #{status, body}`: like
/// `http::post`, but a 4xx/5xx response is returned with its status and
/// body (e.g. Codex device polling answers 403/404 while pending).
/// Transport failures and non-string header values are errors.
pub(crate) fn register_post_status(module: &mut Module) {
    module.set_native_fn(
        "post_status",
        |url: String, body: String, headers: Map| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
            let mut request = ureq::post(&url);
            for (key, value) in &headers {
                let value = value.clone().into_string().map_err(|kind| {
                    runtime_error(format!(
                        "http::post_status: header {key} is a {kind}, not a string"
                    ))
                })?;
                request = request.set(key.as_str(), &value);
            }
            let response = match request.send_string(&body) {
                Ok(response) => response,
                Err(ureq::Error::Status(_, response)) => response,
                Err(error) => {
                    return Err(runtime_error(format!("HTTP POST failed: {error}")));
                }
            };
            let status = i64::from(response.status());
            let body = response
                .into_string()
                .map_err(|e| runtime_error(format!("Failed to read response body: {e}")))?;
            let mut result = Map::new();
            result.insert("status".into(), Dynamic::from(status));
            result.insert("body".into(), Dynamic::from(body));
            Ok(Dynamic::from_map(result))
        },
    );
}

/// `time::now_secs() -> int`: seconds since the Unix epoch.
pub(crate) fn build_time_module() -> RhaiModule {
    let mut module = Module::new();
    module.set_native_fn("now_secs", || -> Result<i64, Box<rhai::EvalAltResult>> {
        let elapsed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| runtime_error(format!("time::now_secs: clock before the epoch: {e}")))?;
        i64::try_from(elapsed.as_secs())
            .map_err(|e| runtime_error(format!("time::now_secs: out of range: {e}")))
    });
    RhaiModule {
        name: "time".to_string(),
        module,
    }
}
