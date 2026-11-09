#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/custom-provider-http-request-response-lifecycle.feature
//!
//! This test file validates the acceptance criteria for PROV-063:
//! `RhaiCustomProvider` + `request_bridge` + `response_bridge` that wire
//! the 7 required Rhai functions from PROV-062 through an async HTTP
//! request/response lifecycle.
//!
//! These tests import symbols that will only exist AFTER PROV-063 is
//! implemented (`RhaiCustomProvider`, the `request_bridge` and
//! `response_bridge` submodules of `codelet_providers::custom`). They
//! therefore fail to compile until the production code lands — this is
//! the red phase.

#[path = "custom_http_test_helpers.rs"]
mod helpers;

use std::sync::Arc;

use codelet_common::{ContentPart, Message, MessageContent};
use codelet_providers::custom::request_bridge::messages_to_rhai;
use codelet_providers::custom::response_bridge::rhai_to_completion_response;
use codelet_providers::custom::{ProviderConfig, RhaiCustomProvider, ScriptLoader};
use codelet_providers::{CompletionResponse, LlmProvider, ProviderError, StopReason};

use helpers::{config_with_full_script, config_with_script, FULL_HAPPY_SCRIPT};
use rhai::{Dynamic, Map};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// =========================================================================
// Helpers local to this test file
// =========================================================================

/// Build a fresh `RhaiCustomProvider` from a config + inline script using a
/// newly-allocated `ScriptLoader`. Panics on construction failure because
/// each test is expected to provide a well-formed script.
fn build_provider(cfg: ProviderConfig, model_alias: &str) -> RhaiCustomProvider {
    let loader = Arc::new(ScriptLoader::with_default_engine());
    RhaiCustomProvider::new(Arc::new(cfg), loader, model_alias.to_string())
        .expect("construct RhaiCustomProvider")
}

// Test functions live in submodules (REFAC-030) so every file stays under 300 lines.
#[path = "custom_http_lifecycle/errors_and_end_to_end.rs"]
mod errors_and_end_to_end;
#[path = "custom_http_lifecycle/provider_and_bridges.rs"]
mod provider_and_bridges;
#[path = "custom_http_lifecycle/request_building.rs"]
mod request_building;
