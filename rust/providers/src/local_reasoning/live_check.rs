//! The live check (MODEL-016): where the engine reports a model's levels
//! (llama.cpp `GET /props`, Ollama `POST /api/show`, LM Studio
//! `GET /api/v1/models`), the report is compared with the declaration. The
//! declaration is authoritative: the check only reports a mismatch, or that
//! it could not be made, and never changes what is sent (product owner,
//! 2026-11-04). On a llama.cpp router the check reads the selected model's
//! own props (BUG-512).

use std::collections::BTreeSet;
use std::time::Duration;

use serde_json::{json, Value};

use super::{LocalEngine, LocalReasoning};
use crate::models::ReasoningCapability;

/// What the live check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveCheckReport {
    /// The engine reports the declared levels.
    Agrees,
    /// The engine reports other levels; the message names the model, the
    /// declared levels and the reported ones.
    Mismatch(String),
    /// The check could not be made; the message names the model and the
    /// endpoint.
    Failed(String),
}

impl LiveCheckReport {
    /// The notice the user is shown; `None` when the levels agree.
    pub fn notice(&self) -> Option<&str> {
        match self {
            Self::Agrees => None,
            Self::Mismatch(message) | Self::Failed(message) => Some(message),
        }
    }
}

/// A pending live check of one profile model.
#[derive(Debug, Clone)]
pub struct LiveCheck {
    engine: LocalEngine,
    base_url: String,
    api_key: Option<String>,
    model: String,
    capability: ReasoningCapability,
}

/// A level as compared: Ollama `false` and LM Studio `off` are the off
/// form, `true` and `on` are On, and other names compare literally.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Level {
    Off,
    On,
    Name(String),
}

fn reported_level(value: &Value) -> Result<(Level, String), String> {
    match value {
        Value::Bool(false) => Ok((Level::Off, "false".to_string())),
        Value::Bool(true) => Ok((Level::On, "true".to_string())),
        Value::String(name) if name == "off" => Ok((Level::Off, name.clone())),
        Value::String(name) if name == "on" => Ok((Level::On, name.clone())),
        Value::String(name) => Ok((Level::Name(name.clone()), name.clone())),
        other => Err(format!("it reports {other}, which is not a level")),
    }
}

fn listed(words: &[String]) -> String {
    match words {
        [] => "no levels".to_string(),
        words => words.join(", "),
    }
}

impl LiveCheck {
    /// The live check of `model` on `base_url`; `None` when its engine
    /// reports no levels (vLLM, SGLang, Strata, TGI, TensorRT-LLM and
    /// LMDeploy have no such endpoint).
    pub fn for_model(
        base_url: &str,
        api_key: Option<&str>,
        model: &str,
        reasoning: &LocalReasoning,
    ) -> Option<Self> {
        matches!(
            reasoning.engine,
            LocalEngine::LlamaCpp | LocalEngine::Ollama | LocalEngine::LmStudio
        )
        .then(|| Self {
            engine: reasoning.engine,
            base_url: base_url.to_string(),
            api_key: api_key.filter(|key| !key.is_empty()).map(String::from),
            model: model.to_string(),
            capability: reasoning.capability.clone(),
        })
    }

    /// The endpoint the engine reports levels on.
    pub fn endpoint(&self) -> &'static str {
        match self.engine {
            LocalEngine::Ollama => "POST /api/show",
            LocalEngine::LmStudio => "GET /api/v1/models",
            _ => "GET /props",
        }
    }

    /// Read the engine's report and compare it with the declaration.
    pub async fn run(&self) -> LiveCheckReport {
        let model = &self.model;
        let endpoint = self.endpoint();
        let reported = match self.reported().await {
            Ok(reported) => reported,
            Err(reason) => {
                return LiveCheckReport::Failed(format!(
                    "the live check of {model}'s thinking levels failed: {endpoint} on \
                     {}: {reason}; its declared levels still apply",
                    origin(&self.base_url)
                ))
            }
        };
        let (declared_levels, declared_words) = self.declared();
        let reported_levels: BTreeSet<Level> =
            reported.iter().map(|(level, _)| level.clone()).collect();
        if declared_levels == reported_levels {
            return LiveCheckReport::Agrees;
        }
        let reported_words: Vec<String> = reported.into_iter().map(|(_, word)| word).collect();
        LiveCheckReport::Mismatch(format!(
            "{model} declares the thinking levels {} but {} reports {} on {endpoint}; the \
             declaration applies",
            listed(&declared_words),
            self.engine.name(),
            listed(&reported_words)
        ))
    }

    /// The declared levels as compared, and as the report lists them. A
    /// declared none or a toggle's Off is the off form, except against
    /// llama.cpp, whose probe never reports none (research §4).
    fn declared(&self) -> (BTreeSet<Level>, Vec<String>) {
        let mut levels = BTreeSet::new();
        let mut words = Vec::new();
        for effort in self.capability.efforts() {
            words.push(effort.clone());
            levels.insert(match effort.as_str() {
                "none" => Level::Off,
                name => Level::Name(name.to_string()),
            });
        }
        if self.capability.has_toggle() {
            levels.insert(Level::Off);
        }
        if self.capability.takes_on() {
            levels.insert(Level::On);
            words.extend(["On".to_string(), "Off".to_string()]);
        }
        if self.engine == LocalEngine::LlamaCpp {
            levels.remove(&Level::Off);
        }
        (levels, words)
    }

    /// The levels the engine reports for the model.
    async fn reported(&self) -> Result<Vec<(Level, String)>, String> {
        let root = root(&self.base_url);
        let body = match self.engine {
            LocalEngine::Ollama => {
                self.fetch(
                    reqwest::Method::POST,
                    &format!("{root}/api/show"),
                    Some(json!({"model": self.model})),
                )
                .await?
            }
            LocalEngine::LmStudio => {
                self.fetch(reqwest::Method::GET, &format!("{root}/api/v1/models"), None)
                    .await?
            }
            // BUG-512: a llama.cpp router answers the bare GET /props with
            // its own props and routes GET /props?model=<id> to that model's
            // server; a single-model server ignores the query. The check
            // only reports, so it never makes a router load the model.
            _ => {
                self.fetch_with_query(
                    reqwest::Method::GET,
                    &format!("{root}/props"),
                    &[("model", self.model.as_str()), ("autoload", "false")],
                    None,
                )
                .await?
            }
        };
        let values = match self.engine {
            LocalEngine::Ollama => body
                .pointer("/thinking/values")
                .and_then(Value::as_array)
                .ok_or("the response has no thinking values for the model")?
                .clone(),
            LocalEngine::LmStudio => lm_studio_options(&body, &self.model)?,
            _ => body
                .pointer("/chat_template_caps/reasoning_effort_levels")
                .and_then(Value::as_array)
                .ok_or("the response has no chat_template_caps.reasoning_effort_levels")?
                .clone(),
        };
        values.iter().map(reported_level).collect()
    }

    async fn fetch(
        &self,
        method: reqwest::Method,
        url: &str,
        body: Option<Value>,
    ) -> Result<Value, String> {
        self.fetch_with_query(method, url, &[], body).await
    }

    async fn fetch_with_query(
        &self,
        method: reqwest::Method,
        url: &str,
        query: &[(&str, &str)],
        body: Option<Value>,
    ) -> Result<Value, String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|error| error.without_url().to_string())?;
        let mut request = client.request(method, url);
        if !query.is_empty() {
            request = request.query(query);
        }
        if let Some(key) = &self.api_key {
            request = request.bearer_auth(key);
        }
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request
            .send()
            .await
            .map_err(|error| error.without_url().to_string())?;
        let status = response.status();
        if !status.is_success() {
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
}

/// The server's own reason in an error response: `error.message` (llama.cpp
/// and LM Studio) or a plain `error` string (Ollama). BUG-512: a llama.cpp
/// router refuses a model it has not loaded or does not know this way.
fn server_error_message(body: &Value) -> Option<&str> {
    let error = body.get("error")?;
    error
        .as_str()
        .or_else(|| error.get("message").and_then(Value::as_str))
        .filter(|message| !message.is_empty())
}

/// LM Studio's `capabilities.reasoning.allowed_options` for `model`, matched
/// by its key or a loaded instance id. A model without a reasoning entry
/// reports no levels.
fn lm_studio_options(body: &Value, model: &str) -> Result<Vec<Value>, String> {
    let entry = body
        .get("models")
        .and_then(Value::as_array)
        .ok_or("the response has no models list")?
        .iter()
        .find(|entry| {
            entry.get("key").and_then(Value::as_str) == Some(model)
                || entry
                    .get("loaded_instances")
                    .and_then(Value::as_array)
                    .is_some_and(|instances| {
                        instances.iter().any(|instance| {
                            instance.get("id").and_then(Value::as_str) == Some(model)
                        })
                    })
        })
        .ok_or("the model is not reported")?;
    match entry.pointer("/capabilities/reasoning") {
        None => Ok(Vec::new()),
        Some(reasoning) => Ok(reasoning
            .get("allowed_options")
            .and_then(Value::as_array)
            .ok_or("its capabilities.reasoning has no allowed_options")?
            .clone()),
    }
}

/// The server root: the base URL without a trailing `/v1`.
fn root(base_url: &str) -> &str {
    let trimmed = base_url.trim_end_matches('/');
    trimmed.strip_suffix("/v1").unwrap_or(trimmed)
}

/// The endpoint origin, without path, query or credentials.
fn origin(base_url: &str) -> String {
    match reqwest::Url::parse(base_url) {
        Ok(url) if url.has_host() => url.origin().ascii_serialization(),
        _ => "an invalid endpoint".to_string(),
    }
}
