//! NAPI bindings for ThinkingConfigFacade
//!
//! Exposes thinking configuration functionality to TypeScript/Node.js.
//! Feature: spec/features/thinking-config-facade-for-provider-specific-reasoning.feature

use codelet_tools::facade::{
    ClaudeThinkingFacade, Gemini25ThinkingFacade, Gemini3ThinkingFacade, ThinkingConfigFacade,
    ThinkingLevel,
};
use napi_derive::napi;

/// TypeScript-friendly thinking level enum
#[napi]
pub enum JsThinkingLevel {
    /// Disable thinking/reasoning entirely
    Off,
    /// Minimal thinking (fast responses)
    Low,
    /// Balanced thinking (default for most tasks)
    Medium,
    /// Maximum thinking (complex reasoning tasks)
    High,
}

impl From<JsThinkingLevel> for ThinkingLevel {
    fn from(level: JsThinkingLevel) -> Self {
        match level {
            JsThinkingLevel::Off => ThinkingLevel::Off,
            JsThinkingLevel::Low => ThinkingLevel::Low,
            JsThinkingLevel::Medium => ThinkingLevel::Medium,
            JsThinkingLevel::High => ThinkingLevel::High,
        }
    }
}

/// Get thinking configuration JSON for a provider at a specific level.
///
/// # Arguments
/// * `provider` - Provider identifier: "gemini-3", "gemini-2.5", "claude", etc.
/// * `level` - Thinking intensity level
///
/// # Returns
/// JSON string containing the provider-specific thinking configuration.
///
/// # Example
/// ```typescript
/// import { getThinkingConfig, JsThinkingLevel } from '@anthropic/codelet-napi';
///
/// const config = JSON.parse(getThinkingConfig('gemini-3', JsThinkingLevel.High));
/// // { thinkingConfig: { includeThoughts: true, thinkingLevel: "high" } }
/// ```
#[napi]
pub fn get_thinking_config(provider: String, level: JsThinkingLevel) -> napi::Result<String> {
    let level: ThinkingLevel = level.into();

    let config = match provider.as_str() {
        "gemini-3" | "gemini-3-pro" | "gemini-3-flash" | "gemini-3-pro-preview"
        | "gemini-3-flash-preview" => Gemini3ThinkingFacade.request_config(level),
        "gemini-2.5" | "gemini-2.5-pro" | "gemini-2.5-flash" | "gemini-2.5-pro-preview"
        | "gemini-2.5-flash-preview" => Gemini25ThinkingFacade.request_config(level),
        "claude" | "claude-3" | "claude-opus" | "claude-sonnet" | "claude-3-opus"
        | "claude-3-sonnet" | "claude-3.5-sonnet" | "claude-3.5-haiku" => {
            ClaudeThinkingFacade.request_config(level)
        }
        // Unknown provider - return empty config (no thinking)
        _ => serde_json::json!({}),
    };

    serde_json::to_string(&config).map_err(|e| {
        napi::Error::new(
            napi::Status::GenericFailure,
            format!("Failed to serialize config: {}", e),
        )
    })
}

/// Check if a response part contains thinking content.
///
/// # Arguments
/// * `provider` - Provider identifier: "gemini-3", "gemini-2.5", "claude", etc.
/// * `part_json` - JSON string of the response part
///
/// # Returns
/// true if the part contains thinking/reasoning content, false otherwise.
///
/// # Example
/// ```typescript
/// import { isThinkingContent } from '@anthropic/codelet-napi';
///
/// const part = JSON.stringify({ thought: true, text: "Let me think..." });
/// const isThinking = isThinkingContent('gemini-3', part);
/// // true
/// ```
#[napi]
pub fn is_thinking_content(provider: String, part_json: String) -> napi::Result<bool> {
    let part: serde_json::Value = serde_json::from_str(&part_json).map_err(|e| {
        napi::Error::new(
            napi::Status::InvalidArg,
            format!("Invalid JSON: {}", e),
        )
    })?;

    let is_thinking = match provider.as_str() {
        "gemini-3" | "gemini-3-pro" | "gemini-3-flash" | "gemini-3-pro-preview"
        | "gemini-3-flash-preview" => Gemini3ThinkingFacade.is_thinking_part(&part),
        "gemini-2.5" | "gemini-2.5-pro" | "gemini-2.5-flash" | "gemini-2.5-pro-preview"
        | "gemini-2.5-flash-preview" => Gemini25ThinkingFacade.is_thinking_part(&part),
        "claude" | "claude-3" | "claude-opus" | "claude-sonnet" | "claude-3-opus"
        | "claude-3-sonnet" | "claude-3.5-sonnet" | "claude-3.5-haiku" => {
            ClaudeThinkingFacade.is_thinking_part(&part)
        }
        // Unknown provider - not thinking content
        _ => false,
    };

    Ok(is_thinking)
}

/// Extract thinking text from a response part.
///
/// # Arguments
/// * `provider` - Provider identifier: "gemini-3", "gemini-2.5", "claude", etc.
/// * `part_json` - JSON string of the response part
///
/// # Returns
/// The thinking text if present, null otherwise.
#[napi]
pub fn extract_thinking_text(provider: String, part_json: String) -> napi::Result<Option<String>> {
    let part: serde_json::Value = serde_json::from_str(&part_json).map_err(|e| {
        napi::Error::new(
            napi::Status::InvalidArg,
            format!("Invalid JSON: {}", e),
        )
    })?;

    let text = match provider.as_str() {
        "gemini-3" | "gemini-3-pro" | "gemini-3-flash" | "gemini-3-pro-preview"
        | "gemini-3-flash-preview" => Gemini3ThinkingFacade.extract_thinking_text(&part),
        "gemini-2.5" | "gemini-2.5-pro" | "gemini-2.5-flash" | "gemini-2.5-pro-preview"
        | "gemini-2.5-flash-preview" => Gemini25ThinkingFacade.extract_thinking_text(&part),
        "claude" | "claude-3" | "claude-opus" | "claude-sonnet" | "claude-3-opus"
        | "claude-3-sonnet" | "claude-3.5-sonnet" | "claude-3.5-haiku" => {
            ClaudeThinkingFacade.extract_thinking_text(&part)
        }
        // Unknown provider - no text
        _ => None,
    };

    Ok(text)
}
