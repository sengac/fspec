//! RLCD-002 — client: Jev/Simple-Jev v1 classifier boundary.
//!
//! Feature: spec/features/decision-first-class-rig-tool-for-typed-rlcd-decisions-choice-score-noul.feature
//!
//! The `RlcdClient` trait is the model-agnostic boundary every RLCD consumer
//! speaks (the current backend implements the Jev/Simple-Jev v1 protocol:
//! `POST /v1/classifier` with `{model, state, questions}`). `HttpRlcdClient`
//! is the production implementation. Error mapping (per the fail-open
//! policy): 422 => `Validation` (the agent's own fault — surface the detail),
//! 429 => `Busy` (Retry-After), 5xx/timeout => `Server`, connect =>
//! `Unreachable`.

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::rlcd::error::RlcdError;

/// One question, already validated (choice/score/noul semantics enforced by
/// `DecisionTool::validate_args` before this struct is built).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlcdQuestion {
    /// Question id — becomes the answer key (insertion order preserved).
    pub qid: String,
    /// Discriminator: `choice`, `score` or `noul`.
    pub r#type: String,
    /// Prompt text for the question (string; null when none supplied).
    pub instructions: Option<String>,
    /// criteria: choice => object (2..=50), score => array (2..=50),
    /// noul => optional object with only true/false keys.
    pub criteria: Option<serde_json::Value>,
}

/// A successful classifier response (Jev/Simple-Jev v1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlcdResponse {
    /// The model that answered (echo of the request's model).
    pub model: String,
    /// qid -> answer object (`{type, choice, confidence, probabilities}` /
    /// `{type, score, confidence, probabilities, legend}` / `{type, noul}`).
    pub answers: serde_json::Value,
    /// Usage accounting (`input_tokens`, `output_tokens`).
    pub usage: serde_json::Value,
}

/// The model-agnostic RLCD client boundary.
#[async_trait]
pub trait RlcdClient: Send + Sync {
    /// Score the supplied questions against `state` on the backend at
    /// `base_url`, using `model` (must equal the server's loaded model —
    /// the caller echoes it from `/health`).
    async fn classify(
        &self,
        base_url: &str,
        model: &str,
        state: &str,
        questions: &[RlcdQuestion],
        budget: Duration,
    ) -> Result<RlcdResponse, RlcdError>;
}

/// Production `RlcdClient`: speaks Jev/Simple-Jev v1 over HTTP.
#[derive(Debug, Default)]
pub struct HttpRlcdClient;

/// Parse a 422 body into its first actionable detail (verbatim).
fn first_detail(body: &str) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return body.to_string();
    };
    let error = value.get("error").cloned().unwrap_or_default();
    // envelope: {error: {message, details: [{message}, ...]}} — the first
    // detail (if any) is the most specific; otherwise the summary message.
    if let Some(details) = error.get("details").and_then(serde_json::Value::as_array) {
        for detail in details {
            if let Some(message) = detail.get("message").and_then(serde_json::Value::as_str) {
                return message.to_string();
            }
        }
    }
    error
        .get("message")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| body.to_string())
}

/// Parse a `Retry-After` header value into seconds (default 1 when missing).
fn retry_after_secs(header: Option<&String>) -> u64 {
    header
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(1)
}

#[async_trait]
impl RlcdClient for HttpRlcdClient {
    async fn classify(
        &self,
        base_url: &str,
        model: &str,
        state: &str,
        questions: &[RlcdQuestion],
        budget: Duration,
    ) -> Result<RlcdResponse, RlcdError> {
        let endpoint = format!("{}/v1/classifier", base_url.trim_end_matches('/'));
        let client = reqwest::Client::builder()
            .timeout(budget)
            .build()
            .map_err(|e| RlcdError::Unreachable {
                url: endpoint.clone(),
                detail: format!("client build failed: {e}"),
            })?;

        // Jev/Simple-Jev v1 request: {model, state, questions: {qid: {type,
        // instructions, criteria?}}} — insertion order of `questions` is the
        // wire order (the protocol assigns labels / breaks ties by it).
        let mut questions_obj = serde_json::Map::new();
        for q in questions {
            let mut spec = serde_json::Map::new();
            spec.insert("type".to_string(), json!(q.r#type));
            spec.insert(
                "instructions".to_string(),
                json!(q.instructions.clone().unwrap_or_default()),
            );
            if let Some(criteria) = &q.criteria {
                spec.insert("criteria".to_string(), criteria.clone());
            }
            questions_obj.insert(q.qid.clone(), serde_json::Value::Object(spec));
        }
        let body = json!({
            "model": model,
            "state": state,
            "questions": questions_obj,
        });

        let response = client
            .post(&endpoint)
            .json(&body)
            .send()
            .await
            .map_err(|e| RlcdError::Unreachable {
                url: endpoint.clone(),
                detail: if e.is_timeout() {
                    format!("request timed out after {budget:?}")
                } else {
                    e.to_string()
                },
            })?;

        let status = response.status();
        // 429 carries Retry-After — capture it before the body consumes
        // the response.
        let retry_after_header: Option<String> = response
            .headers()
            .get("retry-after")
            .and_then(|h| h.to_str().ok().map(str::to_string));
        let text = response
            .text()
            .await
            .map_err(|e| RlcdError::Server {
                detail: format!("reading the /v1/classifier body failed: {e}"),
            })?;

        if status.is_success() {
            let parsed: RlcdResponse = serde_json::from_str(&text).map_err(|e| {
                RlcdError::Server {
                    detail: format!("malformed classifier response: {e}"),
                }
            })?;
            return Ok(parsed);
        }
        if status.as_u16() == 422 {
            return Err(RlcdError::Validation {
                detail: first_detail(&text),
            });
        }
        if status.as_u16() == 429 {
            let retry_after = retry_after_secs(retry_after_header.as_ref());
            return Err(RlcdError::Busy { retry_after });
        }
        Err(RlcdError::Server {
            detail: format!("server responded {status}: {}", first_detail(&text)),
        })
    }
}
