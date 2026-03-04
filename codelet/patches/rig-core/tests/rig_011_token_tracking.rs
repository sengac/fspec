//! Feature: spec/features/rig-core-usage-reasoning-tokens-propagation.feature
//!
//! This test file validates the acceptance criteria defined in the feature file.
//! Scenarios map directly to Gherkin scenarios.
//!
//! Layer 1: completion::Usage struct
//! Layer 2: OpenAI Responses API → completion::Usage
//! Layer 2b: OpenAI Completions API → completion::Usage

#![allow(clippy::unwrap_used, clippy::expect_used)]

// =============================================================================
// Layer 1: rig-core completion::Usage struct
// =============================================================================

// Scenario: rig-core Usage struct includes reasoning_tokens field
#[test]
fn test_usage_struct_includes_reasoning_tokens_field() {
    // @step Given the rig-core completion Usage struct is defined
    let usage = rig::completion::Usage::new();

    // @step When I inspect the Usage struct fields
    // @step Then it should have a reasoning_tokens field of type Option<u64>
    let _reasoning: Option<u64> = usage.reasoning_tokens;

    // @step And the Default impl should set reasoning_tokens to None
    let default_usage = rig::completion::Usage::default();
    assert_eq!(default_usage.reasoning_tokens, None);

    // @step And the new() constructor should set reasoning_tokens to None
    assert_eq!(usage.reasoning_tokens, None);
}

// Scenario: Usage Add impl correctly sums reasoning_tokens
#[test]
fn test_usage_add_sums_reasoning_tokens() {
    // @step Given a Usage with reasoning_tokens Some(100)
    let a = rig::completion::Usage {
        reasoning_tokens: Some(100),
        ..rig::completion::Usage::new()
    };

    // @step And another Usage with reasoning_tokens Some(200)
    let b = rig::completion::Usage {
        reasoning_tokens: Some(200),
        ..rig::completion::Usage::new()
    };

    // @step When the two Usage values are added together
    let result = a + b;

    // @step Then the result should have reasoning_tokens Some(300)
    assert_eq!(result.reasoning_tokens, Some(300));
}

// Scenario: Usage AddAssign impl correctly accumulates reasoning_tokens
#[test]
fn test_usage_add_assign_accumulates_reasoning_tokens() {
    // @step Given a Usage with reasoning_tokens Some(100)
    let mut a = rig::completion::Usage {
        reasoning_tokens: Some(100),
        ..rig::completion::Usage::new()
    };

    // @step When I add-assign a Usage with reasoning_tokens Some(200)
    let b = rig::completion::Usage {
        reasoning_tokens: Some(200),
        ..rig::completion::Usage::new()
    };
    a += b;

    // @step Then the original should have reasoning_tokens Some(300)
    assert_eq!(a.reasoning_tokens, Some(300));
}

// Scenario: Usage Add handles None reasoning_tokens gracefully
#[test]
fn test_usage_add_handles_none_reasoning_tokens() {
    // @step Given a Usage with reasoning_tokens Some(100)
    let a = rig::completion::Usage {
        reasoning_tokens: Some(100),
        ..rig::completion::Usage::new()
    };

    // @step And another Usage with reasoning_tokens None
    let b = rig::completion::Usage {
        reasoning_tokens: None,
        ..rig::completion::Usage::new()
    };

    // @step When the two Usage values are added together
    let result = a + b;

    // @step Then the result should have reasoning_tokens Some(100)
    assert_eq!(result.reasoning_tokens, Some(100));
}

// Scenario: Usage Add handles both None reasoning_tokens
#[test]
fn test_usage_add_handles_both_none_reasoning_tokens() {
    // @step Given a Usage with reasoning_tokens None
    let a = rig::completion::Usage {
        reasoning_tokens: None,
        ..rig::completion::Usage::new()
    };

    // @step And another Usage with reasoning_tokens None
    let b = rig::completion::Usage {
        reasoning_tokens: None,
        ..rig::completion::Usage::new()
    };

    // @step When the two Usage values are added together
    let result = a + b;

    // @step Then the result should have reasoning_tokens None
    assert_eq!(result.reasoning_tokens, None);
}

// =============================================================================
// Layer 2: OpenAI Responses API → completion::Usage
// =============================================================================

use rig::providers::openai::responses_api::{
    InputTokensDetails, OutputTokensDetails, ResponsesUsage,
};

fn make_responses_usage(reasoning_tokens: u64, cached_tokens: u64) -> ResponsesUsage {
    ResponsesUsage {
        input_tokens: 1000,
        input_tokens_details: Some(InputTokensDetails { cached_tokens }),
        output_tokens: 500,
        output_tokens_details: OutputTokensDetails { reasoning_tokens },
        total_tokens: 1500,
    }
}

fn make_responses_completion_response(
    usage: ResponsesUsage,
) -> rig::providers::openai::responses_api::CompletionResponse {
    use rig::message::Text;
    use rig::providers::openai::responses_api::{
        AdditionalParameters, AssistantContent, CompletionResponse, Output, OutputMessage,
        OutputRole, ResponseObject, ResponseStatus,
    };
    CompletionResponse {
        id: "test-id".to_string(),
        object: ResponseObject::Response,
        created_at: 0,
        status: ResponseStatus::Completed,
        error: None,
        incomplete_details: None,
        instructions: None,
        max_output_tokens: None,
        model: "gpt-5.3-codex".to_string(),
        usage: Some(usage),
        output: vec![Output::Message(OutputMessage {
            id: "msg-id".to_string(),
            role: OutputRole::Assistant,
            status: ResponseStatus::Completed,
            content: vec![AssistantContent::OutputText(Text {
                text: "Hello".to_string(),
            })],
        })],
        tools: vec![],
        additional_parameters: AdditionalParameters::default(),
    }
}

// Scenario: OpenAI Responses API non-streaming propagates reasoning tokens into Usage
#[test]
fn test_responses_api_non_streaming_propagates_reasoning_tokens() {
    // @step Given an OpenAI Responses API CompletionResponse with output_tokens_details.reasoning_tokens of 1500
    let usage = make_responses_usage(1500, 0);
    let response = make_responses_completion_response(usage);

    // @step When the response is converted to completion::CompletionResponse via TryFrom
    let result: rig::completion::CompletionResponse<
        rig::providers::openai::responses_api::CompletionResponse,
    > = response.try_into().expect("conversion should succeed");

    // @step Then the Usage should have reasoning_tokens Some(1500)
    assert_eq!(result.usage.reasoning_tokens, Some(1500));
}

// Scenario: OpenAI Responses API non-streaming propagates cache_read_input_tokens
#[test]
fn test_responses_api_non_streaming_propagates_cache_read_input_tokens() {
    // @step Given an OpenAI Responses API CompletionResponse with input_tokens_details.cached_tokens of 8000
    let usage = make_responses_usage(0, 8000);
    let response = make_responses_completion_response(usage);

    // @step When the response is converted to completion::CompletionResponse via TryFrom
    let result: rig::completion::CompletionResponse<
        rig::providers::openai::responses_api::CompletionResponse,
    > = response.try_into().expect("conversion should succeed");

    // @step Then the Usage should have cache_read_input_tokens Some(8000)
    assert_eq!(result.usage.cache_read_input_tokens, Some(8000));
}

// Scenario: OpenAI Responses API streaming propagates reasoning tokens
#[test]
fn test_responses_api_streaming_propagates_reasoning_tokens() {
    use rig::completion::GetTokenUsage;

    // @step Given a StreamingCompletionResponse with usage containing output_tokens_details.reasoning_tokens of 2000
    let streaming_resp =
        rig::providers::openai::responses_api::streaming::StreamingCompletionResponse {
            usage: make_responses_usage(2000, 0),
        };

    // @step When token_usage() is called on the streaming response
    let usage = streaming_resp.token_usage().expect("should return usage");

    // @step Then the returned Usage should have reasoning_tokens Some(2000)
    assert_eq!(usage.reasoning_tokens, Some(2000));
}

// Scenario: OpenAI Responses API streaming propagates cache_read_input_tokens
#[test]
fn test_responses_api_streaming_propagates_cache_read_input_tokens() {
    use rig::completion::GetTokenUsage;

    // @step Given a StreamingCompletionResponse with usage containing input_tokens_details.cached_tokens of 5000
    let streaming_resp =
        rig::providers::openai::responses_api::streaming::StreamingCompletionResponse {
            usage: make_responses_usage(0, 5000),
        };

    // @step When token_usage() is called on the streaming response
    let usage = streaming_resp.token_usage().expect("should return usage");

    // @step Then the returned Usage should have cache_read_input_tokens Some(5000)
    assert_eq!(usage.cache_read_input_tokens, Some(5000));
}

// =============================================================================
// Layer 2b: OpenAI Completions API → completion::Usage
// =============================================================================

// Scenario: OpenAI Completions API Usage struct includes completion_tokens_details
#[test]
fn test_completions_api_usage_struct_includes_completion_tokens_details() {
    use rig::providers::openai::completion::CompletionTokensDetails;

    // @step Given the OpenAI Completions API Usage struct is defined
    let usage = rig::providers::openai::completion::Usage::new();

    // @step When I inspect the Usage struct fields
    // @step Then it should have a completion_tokens_details field of type Option<CompletionTokensDetails>
    let _details: Option<CompletionTokensDetails> = usage.completion_tokens_details.clone();

    // @step And CompletionTokensDetails should have a reasoning_tokens field
    let details = CompletionTokensDetails {
        reasoning_tokens: 42,
    };
    assert_eq!(details.reasoning_tokens, 42);
}

// Scenario: OpenAI Completions API non-streaming propagates reasoning tokens
#[test]
fn test_completions_api_non_streaming_propagates_reasoning_tokens() {
    use rig::providers::openai::completion::{
        AssistantContent, Choice, CompletionResponse, CompletionTokensDetails, Message, Usage,
    };

    // @step Given an OpenAI Completions API CompletionResponse with completion_tokens_details containing reasoning_tokens of 1200
    let response = CompletionResponse {
        id: "test-id".to_string(),
        object: "chat.completion".to_string(),
        created: 0,
        model: "gpt-5-codex".to_string(),
        system_fingerprint: None,
        choices: vec![Choice {
            index: 0,
            message: Message::Assistant {
                content: vec![AssistantContent::Text {
                    text: "Hello".to_string(),
                }],
                refusal: None,
                audio: None,
                name: None,
                tool_calls: vec![],
            },
            logprobs: None,
            finish_reason: "stop".to_string(),
        }],
        usage: Some(Usage {
            prompt_tokens: 1000,
            completion_tokens: Some(500),
            total_tokens: 1500,
            prompt_tokens_details: None,
            completion_tokens_details: Some(CompletionTokensDetails {
                reasoning_tokens: 1200,
            }),
        }),
    };

    // @step When the response is converted to completion::Usage
    let result: rig::completion::CompletionResponse<CompletionResponse> =
        response.try_into().expect("conversion should succeed");

    // @step Then the Usage should have reasoning_tokens Some(1200)
    assert_eq!(result.usage.reasoning_tokens, Some(1200));
}

// Scenario: OpenAI Completions API streaming propagates reasoning tokens
#[test]
fn test_completions_api_streaming_propagates_reasoning_tokens() {
    use rig::completion::GetTokenUsage;
    use rig::providers::openai::completion::streaming::StreamingCompletionResponse;
    use rig::providers::openai::completion::{CompletionTokensDetails, Usage};

    // @step Given an OpenAI Completions API streaming chunk with completion_tokens_details containing reasoning_tokens of 3000
    let streaming_resp = StreamingCompletionResponse {
        usage: Usage {
            prompt_tokens: 2000,
            completion_tokens: Some(1000),
            total_tokens: 3000,
            prompt_tokens_details: None,
            completion_tokens_details: Some(CompletionTokensDetails {
                reasoning_tokens: 3000,
            }),
        },
    };

    // @step When the Usage event is emitted during streaming
    let crate_usage = streaming_resp
        .token_usage()
        .expect("should return usage");

    // @step Then the emitted completion::Usage should have reasoning_tokens Some(3000)
    assert_eq!(crate_usage.reasoning_tokens, Some(3000));
}

// Scenario: OpenAI Completions API GetTokenUsage propagates reasoning tokens
#[test]
fn test_completions_api_get_token_usage_propagates_reasoning_tokens() {
    use rig::completion::GetTokenUsage;
    use rig::providers::openai::completion::{CompletionTokensDetails, Usage};

    // @step Given an OpenAI Completions API Usage with completion_tokens_details containing reasoning_tokens of 800
    let usage = Usage {
        prompt_tokens: 1000,
        completion_tokens: Some(500),
        total_tokens: 1500,
        prompt_tokens_details: None,
        completion_tokens_details: Some(CompletionTokensDetails {
            reasoning_tokens: 800,
        }),
    };

    // @step When token_usage() is called via GetTokenUsage trait
    let crate_usage = usage.token_usage().expect("should return usage");

    // @step Then the returned completion::Usage should have reasoning_tokens Some(800)
    assert_eq!(crate_usage.reasoning_tokens, Some(800));
}
