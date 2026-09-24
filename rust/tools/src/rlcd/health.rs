//! RLCD-001 — one /health probe (Jev/Simple-Jev v1 ready shape).
//!
//! Feature: spec/features/rlcd-service-supervisor.feature
//!
//! Extracted from `service.rs` to keep that module under the 300-line
//! ceiling (RLCD-001 architecture note). `service` re-exports it so
//! consumers keep the same import path.

use std::time::Duration;

use serde_json::Value;

use crate::rlcd::error::RlcdError;

/// One /health probe: `GET {url}/health`.
///
/// Returns the model name reported by the server — ALWAYS from the response
/// (protocol rule: request.model must echo it; never hardcode). `Err` for
/// connection failures, non-2xx responses, malformed payloads, a non-ready
/// status, or an empty model field.
pub async fn health_check(url: &str, timeout: Duration) -> Result<String, RlcdError> {
    let endpoint = format!("{}/health", url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| RlcdError::Unreachable {
            url: endpoint.clone(),
            detail: format!("client build failed: {e}"),
        })?;
    let response = match client.get(&endpoint).send().await {
        Ok(r) => r,
        Err(e) => {
            return Err(RlcdError::Unreachable {
                url: endpoint.clone(),
                detail: e.to_string(),
            });
        }
    };
    let status_code = response.status();
    let body = match response.text().await {
        Ok(b) => b,
        Err(e) => {
            return Err(RlcdError::Health {
                detail: format!("reading /health body failed: {e}"),
            });
        }
    };
    if !status_code.is_success() {
        return Err(RlcdError::Health {
            detail: format!("/health responded {status_code}"),
        });
    }
    let value: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            return Err(RlcdError::Health {
                detail: format!("/health body is not JSON: {e}"),
            });
        }
    };
    let status = value.get("status").and_then(Value::as_str).unwrap_or("");
    let model = value.get("model").and_then(Value::as_str).unwrap_or("").to_string();
    if status != "ready" || model.is_empty() {
        return Err(RlcdError::Health {
            detail: format!(
                "/health not ready (status={status:?}, model present={})",
                !model.is_empty()
            ),
        });
    }
    Ok(model)
}
