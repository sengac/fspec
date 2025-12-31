// Feature: spec/features/system-prompt-facade-for-provider-specific-prompt-handling.feature
//
// Tests for SystemPromptFacade trait and provider-specific implementations (TOOL-008).
// Each test maps to a Gherkin scenario with @step comments.

use codelet_tools::facade::{
    ClaudeApiKeySystemPromptFacade, ClaudeOAuthSystemPromptFacade, GeminiSystemPromptFacade,
    OpenAISystemPromptFacade, SystemPromptFacade,
};

// ============================================================================
// Scenario: Claude OAuth facade prepends identity prefix to preamble
// ============================================================================

#[test]
fn test_claude_oauth_facade_prepends_identity_prefix_to_preamble() {
    // @step Given a ClaudeOAuthSystemPromptFacade
    let facade = ClaudeOAuthSystemPromptFacade;

    // @step And a preamble "You are a helpful assistant"
    let preamble = "You are a helpful assistant";

    // @step When I call format_for_api with the preamble
    let result = facade.format_for_api(preamble);

    // @step Then the result should be a JSON array with cache_control
    assert!(result.is_array(), "Result should be a JSON array");
    let arr = result.as_array().unwrap();
    assert!(!arr.is_empty(), "Array should not be empty");

    // Check cache_control is present
    let has_cache_control = arr.iter().any(|block| block.get("cache_control").is_some());
    assert!(has_cache_control, "Should have cache_control field");

    // @step And the text should start with "You are Claude Code"
    let first_block = &arr[0];
    let text = first_block["text"].as_str().unwrap();
    assert!(
        text.starts_with("You are Claude Code"),
        "Text should start with Claude Code prefix, got: {}",
        text
    );

    // @step And the text should contain the original preamble
    // The preamble is in a separate block for OAuth mode
    let all_text: String = arr
        .iter()
        .filter_map(|b| b["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        all_text.contains(preamble),
        "Should contain the original preamble"
    );
}

// ============================================================================
// Scenario: Claude API key facade passes preamble through unchanged
// ============================================================================

#[test]
fn test_claude_api_key_facade_passes_preamble_through_unchanged() {
    // @step Given a ClaudeApiKeySystemPromptFacade
    let facade = ClaudeApiKeySystemPromptFacade;

    // @step And a preamble "You are a helpful assistant"
    let preamble = "You are a helpful assistant";

    // @step When I call format_for_api with the preamble
    let result = facade.format_for_api(preamble);

    // @step Then the result should be a JSON array with cache_control
    assert!(result.is_array(), "Result should be a JSON array");
    let arr = result.as_array().unwrap();
    assert!(!arr.is_empty(), "Array should not be empty");

    // Check cache_control is present
    let has_cache_control = arr.iter().any(|block| block.get("cache_control").is_some());
    assert!(has_cache_control, "Should have cache_control field");

    // @step And the text should NOT start with "You are Claude Code"
    let text = arr[0]["text"].as_str().unwrap();
    assert!(
        !text.starts_with("You are Claude Code"),
        "Text should NOT start with Claude Code prefix"
    );

    // @step And the text should equal the original preamble exactly
    assert_eq!(
        text, preamble,
        "Text should equal original preamble exactly"
    );
}

// ============================================================================
// Scenario: Gemini facade formats preamble as plain string
// ============================================================================

#[test]
fn test_gemini_facade_formats_preamble_as_plain_string() {
    // @step Given a GeminiSystemPromptFacade
    let facade = GeminiSystemPromptFacade;

    // @step And a preamble "You are a helpful assistant"
    let preamble = "You are a helpful assistant";

    // @step When I call format_for_api with the preamble
    let result = facade.format_for_api(preamble);

    // @step Then the result should be a plain string
    assert!(result.is_string(), "Result should be a plain string");

    // @step And the result should start with the preamble
    let text = result.as_str().unwrap();
    assert!(
        text.starts_with(preamble),
        "Result should start with the preamble"
    );

    // @step And the result should contain web tool guidance
    assert!(
        text.contains("Web Search and Browsing"),
        "Result should contain web tool guidance"
    );
    assert!(
        text.contains("google_web_search"),
        "Result should mention google_web_search"
    );
    assert!(
        text.contains("web_fetch"),
        "Result should mention web_fetch"
    );
}

// ============================================================================
// Scenario: OpenAI facade formats preamble as plain string
// ============================================================================

#[test]
fn test_openai_facade_formats_preamble_as_plain_string() {
    // @step Given an OpenAISystemPromptFacade
    let facade = OpenAISystemPromptFacade;

    // @step And a preamble "You are a helpful assistant"
    let preamble = "You are a helpful assistant";

    // @step When I call format_for_api with the preamble
    let result = facade.format_for_api(preamble);

    // @step Then the result should be a plain string
    assert!(result.is_string(), "Result should be a plain string");

    // @step And the result should equal "You are a helpful assistant"
    assert_eq!(result.as_str().unwrap(), preamble);
}

// ============================================================================
// Scenario: Claude facade formats system prompts with cache_control
// ============================================================================

#[test]
fn test_claude_facade_formats_system_prompts_with_cache_control() {
    // @step Given any Claude system prompt facade
    let oauth_facade = ClaudeOAuthSystemPromptFacade;
    let api_key_facade = ClaudeApiKeySystemPromptFacade;

    // @step And a preamble "You are a helpful assistant"
    let preamble = "You are a helpful assistant";

    // @step When I call format_for_api with the preamble
    let oauth_result = oauth_facade.format_for_api(preamble);
    let api_key_result = api_key_facade.format_for_api(preamble);

    // @step Then the result should be a JSON array
    assert!(
        oauth_result.is_array(),
        "OAuth result should be a JSON array"
    );
    assert!(
        api_key_result.is_array(),
        "API key result should be a JSON array"
    );

    // @step And each element should have a cache_control field with type "ephemeral"
    for result in [oauth_result, api_key_result] {
        let arr = result.as_array().unwrap();
        // At least one block should have cache_control
        let has_ephemeral = arr.iter().any(|block| {
            block
                .get("cache_control")
                .and_then(|cc| cc.get("type"))
                .and_then(|t| t.as_str())
                == Some("ephemeral")
        });
        assert!(
            has_ephemeral,
            "Should have cache_control with type 'ephemeral'"
        );
    }
}

// ============================================================================
// Scenario: ClaudeProvider selects correct facade based on token type
// ============================================================================

#[test]
fn test_claude_provider_selects_correct_facade_for_oauth_token() {
    // TOOL-008: Test REAL integration via select_claude_facade()
    use codelet_tools::facade::select_claude_facade;

    // @step Given a ClaudeProvider with OAuth token starting with "cc-"
    // (is_oauth = true simulates OAuth mode)
    let is_oauth = true;

    // @step When the provider selects a system prompt facade
    let facade = select_claude_facade(is_oauth);

    // @step Then it should use ClaudeOAuthSystemPromptFacade
    // Verify by checking that identity_prefix is present (OAuth characteristic)
    let prefix = facade.identity_prefix();
    assert!(prefix.is_some(), "OAuth facade should have identity prefix");
    assert!(
        prefix.unwrap().starts_with("You are Claude Code"),
        "Identity prefix should start with 'You are Claude Code'"
    );

    // Also verify format_for_api produces OAuth format (2 blocks for non-empty preamble)
    let result = facade.format_for_api("test preamble");
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 2, "OAuth mode should produce 2 blocks");
}

// ============================================================================
// Scenario: ClaudeProvider uses API key facade for non-OAuth tokens
// ============================================================================

#[test]
fn test_claude_provider_uses_api_key_facade_for_non_oauth_tokens() {
    // TOOL-008: Test REAL integration via select_claude_facade()
    use codelet_tools::facade::select_claude_facade;

    // @step Given a ClaudeProvider with API key not starting with "cc-"
    // (is_oauth = false simulates API key mode)
    let is_oauth = false;

    // @step When the provider selects a system prompt facade
    let facade = select_claude_facade(is_oauth);

    // @step Then it should use ClaudeApiKeySystemPromptFacade
    // Verify by checking that identity_prefix is absent (API key characteristic)
    let prefix = facade.identity_prefix();
    assert!(
        prefix.is_none(),
        "API key facade should NOT have identity prefix"
    );

    // Also verify format_for_api produces API key format (1 block)
    let result = facade.format_for_api("test preamble");
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 1, "API key mode should produce 1 block");
    assert_eq!(arr[0]["text"], "test preamble");
}

// ============================================================================
// Additional trait method tests
// ============================================================================

#[test]
fn test_facade_provider_method_returns_correct_provider() {
    // Test that each facade returns the correct provider name
    assert_eq!(ClaudeOAuthSystemPromptFacade.provider(), "claude");
    assert_eq!(ClaudeApiKeySystemPromptFacade.provider(), "claude");
    assert_eq!(GeminiSystemPromptFacade.provider(), "gemini");
    assert_eq!(OpenAISystemPromptFacade.provider(), "openai");
}

#[test]
fn test_transform_preamble_applies_provider_specific_transformations() {
    // Claude OAuth should prepend prefix
    let oauth = ClaudeOAuthSystemPromptFacade;
    let transformed = oauth.transform_preamble("Hello");
    assert!(transformed.contains("You are Claude Code"));
    assert!(transformed.contains("Hello"));

    // Claude API key should pass through unchanged
    let api_key = ClaudeApiKeySystemPromptFacade;
    let transformed = api_key.transform_preamble("Hello");
    assert_eq!(transformed, "Hello");

    // Gemini should append web tool guidance
    let gemini = GeminiSystemPromptFacade;
    let transformed = gemini.transform_preamble("Hello");
    assert!(
        transformed.starts_with("Hello"),
        "Gemini should start with original preamble"
    );
    assert!(
        transformed.contains("Web Search and Browsing"),
        "Gemini should append web tool guidance"
    );

    // OpenAI should pass through unchanged
    let openai = OpenAISystemPromptFacade;
    let transformed = openai.transform_preamble("Hello");
    assert_eq!(transformed, "Hello");
}
