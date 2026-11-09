//! Feature: spec/features/glm-zai-deepsearch-fails-with-500-internal-server-error.feature
//!
//! BUG-106 Z.AI DeepSearch tests, moved out of tests.rs so each test file maps
//! one feature (COV-075).

use super::*;
use crate::deep_search_provider_config::{request_config_for_provider, SUB_AGENT_MAX_TOKENS};

// Scenario: ZAI DeepSearch uses streaming execution path
// Feature: spec/features/glm-zai-deepsearch-fails-with-500-internal-server-error.feature

// @step Given a DeepSearch sub-agent is constructed for provider "zai"
// @step When the sub-agent executes the query
#[test]
fn zai_requires_streaming_execution_path() {
    // @step Then the execution path uses streaming to collect the final response
    assert!(provider_uses_streaming_execution("zai"));
    // @step Then the final synthesized answer is returned as one String result
    // Streaming collection produces a String result (tested in collect_final_response_from_stream tests)
}

// Scenario: ZAI DeepSearch config includes max_tokens in additional_params
// Feature: spec/features/glm-zai-deepsearch-fails-with-500-internal-server-error.feature

// @step Given a DeepSearch request config is built for provider "zai"
#[test]
fn zai_config_includes_max_tokens_in_additional_params() {
    let config = request_config_for_provider("zai", "glm-4.7", "deep search prompt", false)
        .expect("zai config should build");

    // @step When the config is serialized for the HTTP request
    let params = config.additional_params.as_ref().expect("params");

    // @step Then the additional_params includes max_tokens set to 8192
    assert_eq!(params["max_tokens"], SUB_AGENT_MAX_TOKENS);

    // @step Then the additional_params includes temperature and top_p
    assert_eq!(params["temperature"], 1.0);
    assert_eq!(params["top_p"], 0.95);
}
