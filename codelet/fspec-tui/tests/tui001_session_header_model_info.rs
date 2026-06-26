//! Feature: spec/features/session-header-model-info.feature
//!
//! TUI-001 — `get_model_info` data-feed parity for the ratatui SessionHeader.
//! Covers both the pure `resolve_model_info` catalog-lookup helper (friendly
//! name + capability flags + graceful fallback) and the `size_badge_label`
//! value-selection helper (compaction threshold preferred over context window).
//! Fully offline — `ModelRegistry::from_response` from inline models.dev JSON,
//! no cache, no network, no tokio runtime — mirroring the RPC-073 approach.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::agent::header_build::size_badge_label;
use codelet_providers::models::{ModelRegistry, ModelsDevResponse};
use codelet_sessions::cloud_models::resolve_model_info;

fn registry_from(json: &str) -> ModelRegistry {
    let resp: ModelsDevResponse = serde_json::from_str(json).expect("valid models.dev json");
    ModelRegistry::from_response(resp)
}

fn anthropic_registry() -> ModelRegistry {
    registry_from(
        r#"{
            "anthropic": {
                "id": "anthropic",
                "name": "Anthropic",
                "env": ["ANTHROPIC_API_KEY"],
                "models": {
                    "claude-opus-4-8": {
                        "id": "claude-opus-4-8",
                        "name": "Claude Opus 4.8",
                        "release_date": "2026-01-01",
                        "reasoning": true,
                        "tool_call": true,
                        "modalities": {"input": ["text", "image"], "output": ["text"]},
                        "limit": {"context": 200000, "output": 64000}
                    }
                }
            }
        }"#,
    )
}

fn google_registry() -> ModelRegistry {
    registry_from(
        r#"{
            "google": {
                "id": "google",
                "name": "Google",
                "env": ["GEMINI_API_KEY"],
                "models": {
                    "gemini-2.5-pro": {
                        "id": "gemini-2.5-pro",
                        "name": "Gemini 2.5 Pro",
                        "tool_call": true,
                        "limit": {"context": 1048576, "output": 65536}
                    }
                }
            }
        }"#,
    )
}

/// Scenario: Known catalog model resolves friendly name and capability flags
#[test]
fn known_catalog_model_resolves_friendly_name_and_capability_flags() {
    // @step Given a model registry containing the anthropic model "claude-opus-4-8" with name "Claude Opus 4.8", reasoning true and vision true
    let registry = anthropic_registry();
    // @step When resolve_model_info runs for provider "anthropic" and model "claude-opus-4-8"
    let info = resolve_model_info(
        Some(&registry),
        "anthropic",
        "claude-opus-4-8",
        200_000,
        192_000,
    );
    // @step Then the resolved display_name is "Claude Opus 4.8"
    assert_eq!(info.display_name, "Claude Opus 4.8");
    // @step And the resolved supports_reasoning is true
    assert!(info.supports_reasoning);
    // @step And the resolved supports_vision is true
    assert!(info.supports_vision);
}

/// Scenario: Gemini provider slug is mapped to google before lookup
#[test]
fn gemini_provider_slug_is_mapped_to_google_before_lookup() {
    // @step Given a model registry containing the google model "gemini-2.5-pro" with name "Gemini 2.5 Pro"
    let registry = google_registry();
    // @step When resolve_model_info runs for provider "gemini" and model "gemini-2.5-pro"
    let info = resolve_model_info(Some(&registry), "gemini", "gemini-2.5-pro", 1_048_576, 0);
    // @step Then the resolved display_name is "Gemini 2.5 Pro"
    assert_eq!(info.display_name, "Gemini 2.5 Pro");
}

/// Scenario: Unknown model falls back to the raw slug with no capability flags
#[test]
fn unknown_model_falls_back_to_raw_slug_with_no_capability_flags() {
    // @step Given a model registry that does not contain the model "totally-unknown-model"
    let registry = anthropic_registry();
    // @step When resolve_model_info runs for provider "anthropic" and model "totally-unknown-model"
    let info = resolve_model_info(
        Some(&registry),
        "anthropic",
        "totally-unknown-model",
        200_000,
        0,
    );
    // @step Then the resolved display_name is "totally-unknown-model"
    assert_eq!(info.display_name, "totally-unknown-model");
    // @step And the resolved supports_reasoning is false
    assert!(!info.supports_reasoning);
    // @step And the resolved supports_vision is false
    assert!(!info.supports_vision);
}

/// Scenario: Size badge uses the compaction threshold when it is greater than zero
#[test]
fn size_badge_uses_compaction_threshold_when_greater_than_zero() {
    // @step Given a ModelInfo with compaction_threshold 192000 and context_window 200000
    let compaction_threshold = 192_000u32;
    let context_window = 200_000u32;
    // @step When the size badge value is computed for the left header line
    let label = size_badge_label(compaction_threshold, context_window);
    // @step Then the size badge shows "192k"
    assert_eq!(label.as_deref(), Some("192k"));
}

/// Scenario: Size badge falls back to the context window when compaction threshold is zero
#[test]
fn size_badge_falls_back_to_context_window_when_compaction_threshold_is_zero() {
    // @step Given a ModelInfo with compaction_threshold 0 and context_window 200000
    let compaction_threshold = 0u32;
    let context_window = 200_000u32;
    // @step When the size badge value is computed for the left header line
    let label = size_badge_label(compaction_threshold, context_window);
    // @step Then the size badge shows "200k"
    assert_eq!(label.as_deref(), Some("200k"));
}
