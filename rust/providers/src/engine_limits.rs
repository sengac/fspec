//! BUG-547: the running context window a local inference engine reports for
//! a model, asked only when a profile declares none.
//!
//! Feature: spec/features/a-missing-profile-model-limit-is-resolved-at-selection.feature
//! Research: spec/attachments/BUG-547/engine-limit-research.md
//!
//! Only the engine's running value is used (vLLM/SGLang `max_model_len`,
//! llama.cpp/Strata `n_ctx`, LM Studio's loaded `context_length`, TGI
//! `max_total_tokens`, Ollama `/api/ps` `context_length`); training-context
//! values are not. No engine reports a max output cap, and TensorRT-LLM and
//! LMDeploy report nothing, so they are not asked. A value that is not a
//! whole positive token count (0, -1, null, a fraction) is not usable.

use std::time::Duration;

use serde_json::Value;

use crate::local_reasoning::LocalEngine;

/// A limit an engine reported, with where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineLimit {
    /// The reported token count (always positive).
    pub tokens: usize,
    /// Engine, endpoint and field, e.g.
    /// `llama.cpp GET /props default_generation_settings.n_ctx`.
    pub source: String,
}

/// Where `engine` reports a model's running context window, as
/// `(endpoint path, field description)`; `None` for an engine that reports
/// none.
fn context_window_endpoint(engine: LocalEngine) -> Option<(&'static str, &'static str)> {
    match engine {
        LocalEngine::Vllm | LocalEngine::Sglang => Some(("/v1/models", "data[].max_model_len")),
        LocalEngine::LlamaCpp | LocalEngine::Strata => {
            Some(("/props", "default_generation_settings.n_ctx"))
        }
        LocalEngine::LmStudio => Some((
            "/api/v1/models",
            "models[].loaded_instances[].config.context_length",
        )),
        LocalEngine::Tgi => Some(("/info", "max_total_tokens")),
        LocalEngine::Ollama => Some(("/api/ps", "models[].context_length")),
        LocalEngine::TensorRtLlm | LocalEngine::Lmdeploy => None,
    }
}

/// Ask `engine` at `base_url` for `model`'s running context window. `Err`
/// carries why nothing usable was reported.
pub async fn context_window_report(
    engine: LocalEngine,
    base_url: &str,
    api_key: Option<&str>,
    model: &str,
) -> Result<EngineLimit, String> {
    let (path, field) = context_window_endpoint(engine)
        .ok_or_else(|| format!("{} does not report a context window", engine.name()))?;
    // BUG-562: a llama.cpp router answers the bare GET /props with its own
    // props (n_ctx 0) and routes GET /props?model=<id> to that model's
    // server; a single-model server ignores the query. The lookup never makes
    // a router load the model (BUG-512's rule).
    let query: &[(&str, &str)] = match engine {
        LocalEngine::LlamaCpp => &[("model", model), ("autoload", "false")],
        _ => &[],
    };
    let body = fetch(&format!("{}{path}", root(base_url)), query, api_key)
        .await
        .map_err(|reason| format!("{} GET {path}: {reason}", engine.name()))?;
    let value = reported_context_window(engine, &body, model).ok_or_else(|| {
        format!(
            "{} GET {path} reports no {field} for {model}",
            engine.name()
        )
    })?;
    let tokens = usable_tokens(value).ok_or_else(|| {
        format!(
            "{} GET {path} reports {field} {value} for {model}, which is not a token count",
            engine.name()
        )
    })?;
    Ok(EngineLimit {
        tokens,
        source: format!("{} GET {path} {field}", engine.name()),
    })
}

/// The reported field for `model` in an engine's response body.
fn reported_context_window<'a>(
    engine: LocalEngine,
    body: &'a Value,
    model: &str,
) -> Option<&'a Value> {
    match engine {
        LocalEngine::Vllm | LocalEngine::Sglang => {
            entry(body.get("data")?, &["id"], model)?.get("max_model_len")
        }
        LocalEngine::LlamaCpp | LocalEngine::Strata => {
            body.pointer("/default_generation_settings/n_ctx")
        }
        LocalEngine::LmStudio => lm_studio_context_length(body, model),
        LocalEngine::Tgi => body.get("max_total_tokens"),
        LocalEngine::Ollama => {
            entry(body.get("models")?, &["name", "model"], model)?.get("context_length")
        }
        LocalEngine::TensorRtLlm | LocalEngine::Lmdeploy => None,
    }
}

/// The entry of `list` whose `keys` names `model`.
fn entry<'a>(list: &'a Value, keys: &[&str], model: &str) -> Option<&'a Value> {
    list.as_array()?.iter().find(|entry| {
        keys.iter()
            .any(|key| entry.get(*key).and_then(Value::as_str) == Some(model))
    })
}

/// LM Studio: the loaded instance named `model`, else the first loaded
/// instance of the model keyed `model`.
fn lm_studio_context_length<'a>(body: &'a Value, model: &str) -> Option<&'a Value> {
    let models = body.get("models")?.as_array()?;
    let instances = |entry: &'a Value| {
        entry
            .get("loaded_instances")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
    };
    let by_instance = models
        .iter()
        .flat_map(instances)
        .find(|instance| instance.get("id").and_then(Value::as_str) == Some(model));
    let instance = match by_instance {
        Some(instance) => instance,
        None => instances(entry(body.get("models")?, &["key"], model)?).next()?,
    };
    instance.pointer("/config/context_length")
}

/// A whole positive token count; anything else is unusable.
fn usable_tokens(value: &Value) -> Option<usize> {
    value
        .as_u64()
        .filter(|tokens| *tokens > 0)
        .and_then(|tokens| usize::try_from(tokens).ok())
}

async fn fetch(url: &str, query: &[(&str, &str)], api_key: Option<&str>) -> Result<Value, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|error| error.without_url().to_string())?;
    let mut request = client.get(url);
    if !query.is_empty() {
        request = request.query(query);
    }
    if let Some(key) = api_key {
        request = request.bearer_auth(key);
    }
    let response = request
        .send()
        .await
        .map_err(|error| error.without_url().to_string())?;
    let status = response.status();
    if !status.is_success() {
        // BUG-562: the server's own reason (a llama.cpp router refusing a
        // model it has not loaded says so in error.message).
        let reason = response
            .json::<Value>()
            .await
            .ok()
            .as_ref()
            .and_then(server_error_message)
            .map(|message| format!(": {message}"))
            .unwrap_or_default();
        return Err(format!("HTTP {status}{reason}"));
    }
    response
        .json::<Value>()
        .await
        .map_err(|error| format!("the response is not JSON: {}", error.without_url()))
}

/// The server's own reason in an error response: `error.message` (llama.cpp,
/// LM Studio) or a plain `error` string (Ollama).
fn server_error_message(body: &Value) -> Option<&str> {
    let error = body.get("error")?;
    error
        .as_str()
        .or_else(|| error.get("message").and_then(Value::as_str))
        .filter(|message| !message.is_empty())
}

/// The server root: the base URL without a trailing `/v1`.
fn root(base_url: &str) -> &str {
    let trimmed = base_url.trim_end_matches('/');
    trimmed.strip_suffix("/v1").unwrap_or(trimmed)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn only_whole_positive_counts_are_usable() {
        assert_eq!(usable_tokens(&json!(65536)), Some(65536));
        for unusable in [json!(0), json!(-1), json!(null), json!(1.5), json!("8192")] {
            assert_eq!(usable_tokens(&unusable), None, "{unusable}");
        }
    }

    #[test]
    fn each_engine_reads_its_running_field() {
        let vllm = json!({"data": [{"id": "other", "max_model_len": 1}, {"id": "m", "max_model_len": 131072}]});
        assert_eq!(
            reported_context_window(LocalEngine::Vllm, &vllm, "m"),
            Some(&json!(131072))
        );
        let ollama =
            json!({"models": [{"name": "m:latest", "model": "m", "context_length": 8192}]});
        assert_eq!(
            reported_context_window(LocalEngine::Ollama, &ollama, "m"),
            Some(&json!(8192))
        );
        let lm_studio = json!({"models": [{"key": "m", "max_context_length": 262144,
            "loaded_instances": [{"id": "m", "config": {"context_length": 4096}}]}]});
        assert_eq!(
            reported_context_window(LocalEngine::LmStudio, &lm_studio, "m"),
            Some(&json!(4096))
        );
        let tgi = json!({"max_total_tokens": 4096, "max_input_tokens": 4095});
        assert_eq!(
            reported_context_window(LocalEngine::Tgi, &tgi, "m"),
            Some(&json!(4096))
        );
        assert_eq!(context_window_endpoint(LocalEngine::TensorRtLlm), None);
        assert_eq!(context_window_endpoint(LocalEngine::Lmdeploy), None);
    }
}
