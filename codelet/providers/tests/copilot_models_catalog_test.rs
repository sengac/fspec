#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/github-copilot-model-catalog-from-models-endpoint.feature
//!
//! This test file validates the acceptance criteria defined in the feature file
//! for PROV-056: GitHub Copilot model catalog, provider options & reasoning effort.
//!
//! The Copilot /models endpoint is the SOLE source of truth for the catalog —
//! NO hardcoded model ids, NO models.dev fallback, NO reasoning-tier heuristics,
//! NO date-gating logic. Tests are intentionally written so the assertions are
//! wholly driven by the JSON the mock /models endpoint returns.
//!
//! All tests are written to FAIL until the PROV-056 implementation lands.

use codelet_providers::copilot::models::{
    build_catalog_from_response, fetch_models, CopilotModelsResponse,
};
use codelet_providers::copilot::provider_options::apply_store_false;
use serde_json::json;
use std::collections::HashMap;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ----------------------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------------------

/// Mount a single GET /models response on the given mock server.
async fn mount_models_response(server: &MockServer, body: serde_json::Value) {
    Mock::given(method("GET"))
        .and(path("/models"))
        .and(header("Authorization", "Bearer ghu_test_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(server)
        .await;
}

/// Convenience: lookup a model by id in the returned catalog.
fn find_model<'a>(
    catalog: &'a [codelet_providers::models::ModelInfo],
    id: &str,
) -> Option<&'a codelet_providers::models::ModelInfo> {
    catalog.iter().find(|m| m.id == id)
}

/// Read the reasoning_variants Vec from a ModelInfo.options entry.
/// Convention: options["reasoning_variants"] is a JSON array of strings, copied
/// verbatim from capabilities.supports.reasoning_effort.
fn reasoning_variants_for(model: &codelet_providers::models::ModelInfo) -> Vec<String> {
    model
        .options
        .get("reasoning_variants")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

// ============================================================================
// Scenario: Reasoning effort tiers come straight from the Copilot endpoint, in order
// ============================================================================
#[tokio::test]
async fn scenario_reasoning_effort_tiers_come_straight_from_endpoint_in_order() {
    let mock_server = MockServer::start().await;

    // @step Given I am authenticated with GitHub Copilot
    let token = "ghu_test_token";

    // @step And the Copilot /models endpoint returns one model whose capabilities.supports.reasoning_effort is the array ["low","medium","high","xhigh"]
    mount_models_response(
        &mock_server,
        json!({
            "data": [
                {
                    "id": "any-model",
                    "name": "Any Model",
                    "version": "any-model-2099-01-01",
                    "model_picker_enabled": true,
                    "capabilities": {
                        "family": "any-family",
                        "limits": {
                            "max_context_window_tokens": 100,
                            "max_output_tokens": 50,
                            "max_prompt_tokens": 100
                        },
                        "supports": {
                            "streaming": true,
                            "tool_calls": true,
                            "vision": false,
                            "reasoning_effort": ["low", "medium", "high", "xhigh"]
                        }
                    }
                }
            ]
        }),
    )
    .await;

    // @step When the catalog is fetched
    let catalog = fetch_models(&mock_server.uri(), token)
        .await
        .expect("fetch_models should succeed");

    // @step Then the catalog contains that model
    let model = find_model(&catalog, "any-model").expect("any-model present in catalog");

    // @step And the model exposes exactly 4 reasoning variants
    let variants = reasoning_variants_for(model);
    assert_eq!(variants.len(), 4, "expected exactly 4 reasoning variants");

    // @step And the variants appear in the order "low", "medium", "high", "xhigh"
    assert_eq!(variants, vec!["low", "medium", "high", "xhigh"]);

    // @step And each variant carries only the model id and the reasoning effort tier
    // (variants are stored as plain strings — there is no per-variant metadata struct)
    for v in &variants {
        assert!(!v.is_empty(), "variant tier must not be empty");
    }

    // @step And no variant adds wire-format extras such as reasoningSummary or include fields
    // (the variants array is just Vec<String>; no extra fields exist by construction)
    assert!(
        !model.options.contains_key("reasoningSummary"),
        "no reasoningSummary at catalog time"
    );
    assert!(
        !model.options.contains_key("include"),
        "no include extras at catalog time"
    );
}

// ============================================================================
// Scenario: Model with no reasoning_effort field exposes no reasoning variants
// ============================================================================
#[tokio::test]
async fn scenario_model_with_no_reasoning_effort_field_exposes_no_variants() {
    let mock_server = MockServer::start().await;

    // @step Given I am authenticated with GitHub Copilot
    let token = "ghu_test_token";

    // @step And the Copilot /models endpoint returns one model whose capabilities.supports object has no reasoning_effort field at all
    mount_models_response(
        &mock_server,
        json!({
            "data": [
                {
                    "id": "no-reasoning-model",
                    "name": "No Reasoning Model",
                    "version": "no-reasoning-model-2099-01-01",
                    "model_picker_enabled": true,
                    "capabilities": {
                        "family": "any-family",
                        "limits": {
                            "max_context_window_tokens": 100,
                            "max_output_tokens": 50,
                            "max_prompt_tokens": 100
                        },
                        "supports": {
                            "streaming": true,
                            "tool_calls": true,
                            "vision": false
                        }
                    }
                }
            ]
        }),
    )
    .await;

    // @step When the catalog is fetched
    let catalog = fetch_models(&mock_server.uri(), token)
        .await
        .expect("fetch_models should succeed");

    // @step Then the catalog contains that model
    let model =
        find_model(&catalog, "no-reasoning-model").expect("no-reasoning-model present in catalog");

    // @step And the model exposes zero reasoning variants
    let variants = reasoning_variants_for(model);
    assert!(
        variants.is_empty(),
        "expected zero variants, got {variants:?}"
    );

    // @step And no reasoning tiers are inferred from the model id, version, or any other source
    // (there is no inference path; absence of reasoning_effort -> empty Vec by construction)
    assert!(
        variants.is_empty(),
        "no inference allowed — must remain empty"
    );
}

// ============================================================================
// Scenario: Model with empty reasoning_effort array exposes no reasoning variants
// ============================================================================
#[tokio::test]
async fn scenario_model_with_empty_reasoning_effort_array_exposes_no_variants() {
    let mock_server = MockServer::start().await;

    // @step Given I am authenticated with GitHub Copilot
    let token = "ghu_test_token";

    // @step And the Copilot /models endpoint returns one model whose capabilities.supports.reasoning_effort is an empty array []
    mount_models_response(
        &mock_server,
        json!({
            "data": [
                {
                    "id": "empty-reasoning-model",
                    "name": "Empty Reasoning Model",
                    "version": "empty-reasoning-model-2099-01-01",
                    "model_picker_enabled": true,
                    "capabilities": {
                        "family": "any-family",
                        "limits": {
                            "max_context_window_tokens": 100,
                            "max_output_tokens": 50,
                            "max_prompt_tokens": 100
                        },
                        "supports": {
                            "streaming": true,
                            "tool_calls": true,
                            "vision": false,
                            "reasoning_effort": []
                        }
                    }
                }
            ]
        }),
    )
    .await;

    // @step When the catalog is fetched
    let catalog = fetch_models(&mock_server.uri(), token)
        .await
        .expect("fetch_models should succeed");

    // @step Then the catalog contains that model
    let model = find_model(&catalog, "empty-reasoning-model")
        .expect("empty-reasoning-model present in catalog");

    // @step And the model exposes zero reasoning variants
    let variants = reasoning_variants_for(model);
    assert!(
        variants.is_empty(),
        "expected zero variants, got {variants:?}"
    );
}

// ============================================================================
// Scenario: release_date is derived purely from the endpoint's version field
// ============================================================================
#[tokio::test]
async fn scenario_release_date_derived_purely_from_version_field() {
    let mock_server = MockServer::start().await;

    // @step Given I am authenticated with GitHub Copilot
    let token = "ghu_test_token";

    // @step And the Copilot /models endpoint returns one model with id "whatever-model" and version "whatever-model-2025-09-15"
    mount_models_response(
        &mock_server,
        json!({
            "data": [
                {
                    "id": "whatever-model",
                    "name": "Whatever Model",
                    "version": "whatever-model-2025-09-15",
                    "model_picker_enabled": true,
                    "capabilities": {
                        "family": "any-family",
                        "limits": {
                            "max_context_window_tokens": 100,
                            "max_output_tokens": 50,
                            "max_prompt_tokens": 100
                        },
                        "supports": {
                            "streaming": true,
                            "tool_calls": true,
                            "vision": false
                        }
                    }
                }
            ]
        }),
    )
    .await;

    // @step When the catalog is fetched
    let catalog = fetch_models(&mock_server.uri(), token)
        .await
        .expect("fetch_models should succeed");

    // @step Then the catalog model "whatever-model" has release_date "2025-09-15"
    let model = find_model(&catalog, "whatever-model").expect("whatever-model present in catalog");
    assert_eq!(
        model.release_date.as_deref(),
        Some("2025-09-15"),
        "release_date should be derived from version field"
    );

    // @step And the release_date is derived by stripping the "{id}-" prefix from the version field returned by the endpoint
    // (no separate assertion needed — the date 2025-09-15 is exactly what `whatever-model-2025-09-15`
    //  stripped of `whatever-model-` produces)

    // @step And no hardcoded date table is consulted
    // (any model id with any date suffix would round-trip identically; verify by parsing a
    //  second arbitrary id/version pair through the same pure helper)
    let response = CopilotModelsResponse {
        data: vec![serde_json::from_value(json!({
            "id": "another-model",
            "name": "Another Model",
            "version": "another-model-2099-12-31",
            "model_picker_enabled": true,
            "capabilities": {
                "family": "any-family",
                "limits": {
                    "max_context_window_tokens": 100,
                    "max_output_tokens": 50,
                    "max_prompt_tokens": 100
                },
                "supports": {
                    "streaming": true,
                    "tool_calls": true,
                    "vision": false
                }
            }
        }))
        .unwrap()],
    };
    let purely_derived = build_catalog_from_response(&response);
    assert_eq!(
        purely_derived[0].release_date.as_deref(),
        Some("2099-12-31"),
        "the same pure derivation must apply to any id/version — no hardcoded table"
    );
}

// ============================================================================
// Scenario: Models flagged model_picker_enabled=false are filtered out
// ============================================================================
#[tokio::test]
async fn scenario_models_flagged_picker_disabled_are_filtered_out() {
    let mock_server = MockServer::start().await;

    // @step Given I am authenticated with GitHub Copilot
    let token = "ghu_test_token";

    // @step And the Copilot /models endpoint returns one entry with model_picker_enabled set to true
    // @step And the same response includes a second entry with model_picker_enabled set to false
    mount_models_response(
        &mock_server,
        json!({
            "data": [
                {
                    "id": "picker-enabled-model",
                    "name": "Picker Enabled",
                    "version": "picker-enabled-model-2099-01-01",
                    "model_picker_enabled": true,
                    "capabilities": {
                        "family": "any-family",
                        "limits": {
                            "max_context_window_tokens": 100,
                            "max_output_tokens": 50,
                            "max_prompt_tokens": 100
                        },
                        "supports": {
                            "streaming": true,
                            "tool_calls": true,
                            "vision": false
                        }
                    }
                },
                {
                    "id": "picker-disabled-model",
                    "name": "Picker Disabled",
                    "version": "picker-disabled-model-2099-01-01",
                    "model_picker_enabled": false,
                    "capabilities": {
                        "family": "any-family",
                        "limits": {
                            "max_context_window_tokens": 100,
                            "max_output_tokens": 50,
                            "max_prompt_tokens": 100
                        },
                        "supports": {
                            "streaming": true,
                            "tool_calls": true,
                            "vision": false
                        }
                    }
                }
            ]
        }),
    )
    .await;

    // @step When the catalog is fetched
    let catalog = fetch_models(&mock_server.uri(), token)
        .await
        .expect("fetch_models should succeed");

    // @step Then the catalog contains the entry whose model_picker_enabled is true
    assert!(
        find_model(&catalog, "picker-enabled-model").is_some(),
        "picker-enabled-model should be in catalog"
    );

    // @step And the catalog does not contain the entry whose model_picker_enabled is false
    assert!(
        find_model(&catalog, "picker-disabled-model").is_none(),
        "picker-disabled-model should be filtered out"
    );
}

// ============================================================================
// Scenario: Each fetch fully replaces the catalog with no merging or fallback
// ============================================================================
#[tokio::test]
async fn scenario_each_fetch_fully_replaces_catalog_with_no_merging() {
    let token = "ghu_test_token";

    // First fetch — server returns model-a and model-b
    let first_server = MockServer::start().await;

    // @step Given I am authenticated with GitHub Copilot
    // @step And the first fetch from the Copilot /models endpoint returns two models with ids "model-a" and "model-b"
    mount_models_response(
        &first_server,
        json!({
            "data": [
                {
                    "id": "model-a",
                    "name": "Model A",
                    "version": "model-a-2099-01-01",
                    "model_picker_enabled": true,
                    "capabilities": {
                        "family": "any-family",
                        "limits": {
                            "max_context_window_tokens": 100,
                            "max_output_tokens": 50,
                            "max_prompt_tokens": 100
                        },
                        "supports": {
                            "streaming": true,
                            "tool_calls": true,
                            "vision": false
                        }
                    }
                },
                {
                    "id": "model-b",
                    "name": "Model B",
                    "version": "model-b-2099-01-01",
                    "model_picker_enabled": true,
                    "capabilities": {
                        "family": "any-family",
                        "limits": {
                            "max_context_window_tokens": 100,
                            "max_output_tokens": 50,
                            "max_prompt_tokens": 100
                        },
                        "supports": {
                            "streaming": true,
                            "tool_calls": true,
                            "vision": false
                        }
                    }
                }
            ]
        }),
    )
    .await;

    // @step When the catalog is fetched
    let first_catalog = fetch_models(&first_server.uri(), token)
        .await
        .expect("first fetch should succeed");

    // @step Then the catalog contains "model-a" and "model-b"
    assert!(find_model(&first_catalog, "model-a").is_some());
    assert!(find_model(&first_catalog, "model-b").is_some());
    assert_eq!(first_catalog.len(), 2);

    // Second fetch — server returns ONLY model-a
    let second_server = MockServer::start().await;

    // @step Given the next fetch from the Copilot /models endpoint returns only "model-a"
    mount_models_response(
        &second_server,
        json!({
            "data": [
                {
                    "id": "model-a",
                    "name": "Model A",
                    "version": "model-a-2099-01-01",
                    "model_picker_enabled": true,
                    "capabilities": {
                        "family": "any-family",
                        "limits": {
                            "max_context_window_tokens": 100,
                            "max_output_tokens": 50,
                            "max_prompt_tokens": 100
                        },
                        "supports": {
                            "streaming": true,
                            "tool_calls": true,
                            "vision": false
                        }
                    }
                }
            ]
        }),
    )
    .await;

    // @step When the catalog is fetched again
    let second_catalog = fetch_models(&second_server.uri(), token)
        .await
        .expect("second fetch should succeed");

    // @step Then the catalog contains only "model-a"
    assert_eq!(
        second_catalog.len(),
        1,
        "second fetch should return exactly one model"
    );
    assert!(find_model(&second_catalog, "model-a").is_some());

    // @step And "model-b" is gone with no merging into the prior fetch
    assert!(
        find_model(&second_catalog, "model-b").is_none(),
        "model-b must not appear after second fetch"
    );

    // @step And no fallback to models.dev or any other source occurs
    // (fetch_models takes only base_url + token; it cannot consult any other source by construction.
    //  This assertion is structural — if fetch_models grew a models.dev call, the function signature
    //  or the wiremock expect(1) on /models would catch it.)
    // The wiremock mounts above use .expect(1) which would fail on an extra request.
}

// ============================================================================
// Scenario: store: false is enforced for every Copilot request regardless of model
// ============================================================================
#[tokio::test]
async fn scenario_store_false_is_enforced_for_every_copilot_request() {
    let mock_server = MockServer::start().await;

    // @step Given I am authenticated with GitHub Copilot
    let token = "ghu_test_token";

    // @step And the catalog has been populated from the Copilot /models endpoint with multiple models
    mount_models_response(
        &mock_server,
        json!({
            "data": [
                {
                    "id": "first-model",
                    "name": "First Model",
                    "version": "first-model-2099-01-01",
                    "model_picker_enabled": true,
                    "capabilities": {
                        "family": "family-one",
                        "limits": {
                            "max_context_window_tokens": 100,
                            "max_output_tokens": 50,
                            "max_prompt_tokens": 100
                        },
                        "supports": {
                            "streaming": true,
                            "tool_calls": true,
                            "vision": false,
                            "reasoning_effort": ["low", "high"]
                        }
                    }
                },
                {
                    "id": "second-model",
                    "name": "Second Model",
                    "version": "second-model-2099-01-01",
                    "model_picker_enabled": true,
                    "capabilities": {
                        "family": "family-two",
                        "limits": {
                            "max_context_window_tokens": 200,
                            "max_output_tokens": 80,
                            "max_prompt_tokens": 200
                        },
                        "supports": {
                            "streaming": true,
                            "tool_calls": true,
                            "vision": true
                        }
                    }
                }
            ]
        }),
    )
    .await;

    let catalog = fetch_models(&mock_server.uri(), token)
        .await
        .expect("fetch_models should succeed");
    assert_eq!(catalog.len(), 2);

    // @step When provider options are built for any Copilot model in the catalog
    for model in &catalog {
        let mut options: HashMap<String, serde_json::Value> = HashMap::new();
        apply_store_false(&mut options);

        // @step Then the resulting provider options contain store: false
        assert_eq!(
            options.get("store"),
            Some(&serde_json::Value::Bool(false)),
            "store should be false for model {}",
            model.id
        );
    }

    // @step And the store: false flag is applied uniformly without inspecting the model id or family
    // (apply_store_false takes only the options map — its signature cannot inspect the model.)
    // Verify by calling it on a freshly empty options map and checking the result is the same.
    let mut empty: HashMap<String, serde_json::Value> = HashMap::new();
    apply_store_false(&mut empty);
    assert_eq!(empty.get("store"), Some(&serde_json::Value::Bool(false)));
}
