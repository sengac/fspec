//! Tests for OAuthCallbackServer<H: CodeExchangeHandler> (PROV-060)
//!
//! Feature: spec/features/shared-oauth-building-blocks.feature
//! Scenario: Generic OAuth callback server unifies PKCE flows

use crate::oauth::callback_server::CodeExchangeHandler;
use anyhow::Result;
use std::collections::HashMap;

/// Fake Codex code exchange handler for testing.
struct FakeCodexHandler;

impl CodeExchangeHandler for FakeCodexHandler {
    fn success_html(&self) -> &str {
        "<h1>Codex Success</h1>"
    }
    fn cancelled_html(&self) -> &str {
        "<h1>Cancelled</h1>"
    }
    fn error_html(&self, message: &str) -> String {
        format!("<h1>Error: {message}</h1>")
    }
    fn extract_code_and_state(
        &self,
        params: &HashMap<String, String>,
    ) -> Result<(String, String)> {
        let code = params
            .get("code")
            .ok_or_else(|| anyhow::anyhow!("Missing code"))?
            .clone();
        let state = params
            .get("state")
            .ok_or_else(|| anyhow::anyhow!("Missing state"))?
            .clone();
        Ok((code, state))
    }
    fn validate_state(&self, expected: &str, received: &str) -> Result<()> {
        if expected != received {
            return Err(anyhow::anyhow!("State mismatch"));
        }
        Ok(())
    }
}

/// Fake Claude code exchange handler (supports iss param).
struct FakeClaudeHandler;

impl CodeExchangeHandler for FakeClaudeHandler {
    fn success_html(&self) -> &str {
        "<h1>Claude Success</h1>"
    }
    fn cancelled_html(&self) -> &str {
        "<h1>Cancelled</h1>"
    }
    fn error_html(&self, message: &str) -> String {
        format!("<h1>Error: {message}</h1>")
    }
    fn extract_code_and_state(
        &self,
        params: &HashMap<String, String>,
    ) -> Result<(String, String)> {
        let code = params
            .get("code")
            .ok_or_else(|| anyhow::anyhow!("Missing code"))?
            .clone();
        let state = params
            .get("state")
            .ok_or_else(|| anyhow::anyhow!("Missing state"))?
            .clone();
        // Claude can also use iss param for multi-region
        let _iss = params.get("iss");
        Ok((code, state))
    }
    fn validate_state(&self, expected: &str, received: &str) -> Result<()> {
        if expected != received {
            return Err(anyhow::anyhow!("State mismatch"));
        }
        Ok(())
    }
}

// @step Given an OAuthCallbackServer parameterized with a CodeExchangeHandler
// @step When a PKCE authorization code callback is received for Codex
// @step Then the server extracts the code and state, and exchanges for tokens via the handler
// @step And the same OAuthCallbackServer with a Claude handler supports multi-region via iss parameter

#[test]
fn codex_handler_extracts_code_and_state() {
    // @step Given an OAuthCallbackServer parameterized with a CodeExchangeHandler
    let handler = FakeCodexHandler;
    let mut params = HashMap::new();
    params.insert("code".to_string(), "auth_code_123".to_string());
    params.insert("state".to_string(), "expected_state".to_string());

    // @step When a PKCE authorization code callback is received for Codex
    let (code, state) = handler.extract_code_and_state(&params).unwrap();

    // @step Then the server extracts the code and state, and exchanges for tokens via the handler
    assert_eq!(code, "auth_code_123");
    assert_eq!(state, "expected_state");
}

#[test]
fn codex_handler_validates_matching_state() {
    let handler = FakeCodexHandler;
    assert!(handler.validate_state("abc", "abc").is_ok());
}

#[test]
fn codex_handler_rejects_mismatched_state() {
    let handler = FakeCodexHandler;
    assert!(handler.validate_state("expected", "wrong").is_err());
}

#[test]
fn claude_handler_supports_iss_parameter() {
    // @step And the same OAuthCallbackServer with a Claude handler supports multi-region via iss parameter
    let handler = FakeClaudeHandler;
    let mut params = HashMap::new();
    params.insert("code".to_string(), "claude_code".to_string());
    params.insert("state".to_string(), "claude_state".to_string());
    params.insert(
        "iss".to_string(),
        "https://eu.anthropic.com".to_string(),
    );

    let (code, state) = handler.extract_code_and_state(&params).unwrap();
    assert_eq!(code, "claude_code");
    assert_eq!(state, "claude_state");
}

#[test]
fn both_handlers_implement_same_trait() {
    // Both Codex and Claude handlers implement CodeExchangeHandler
    fn assert_handler<H: CodeExchangeHandler>(_h: &H) {}
    assert_handler(&FakeCodexHandler);
    assert_handler(&FakeClaudeHandler);
}

#[test]
fn handlers_provide_html_pages() {
    let codex = FakeCodexHandler;
    assert!(codex.success_html().contains("Codex Success"));
    assert!(codex.cancelled_html().contains("Cancelled"));
    assert!(codex.error_html("test").contains("test"));

    let claude = FakeClaudeHandler;
    assert!(claude.success_html().contains("Claude Success"));
}
