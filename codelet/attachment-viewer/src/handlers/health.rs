//! `GET /health` handler — parity with the TS `/health` endpoint.

/// Respond with a plain-text `ok` health indicator.
pub async fn health() -> &'static str {
    "ok"
}
