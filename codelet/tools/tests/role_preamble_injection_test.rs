// Feature: spec/features/role-system-prompt-injection.feature
//
// Tests for BUG-120: Role injection into LLM system prompt.
// The run_with_provider! macro reads session.get_role() and passes the result
// as `preamble` to create_rig_agent().  Each provider's create_rig_agent
// transforms the preamble via its SystemPromptFacade (or equivalent) so the
// role text ends up in the effective system prompt.
//
// These tests verify that every provider's facade correctly embeds role text
// when present, omits it when absent, and that the preamble→facade pipeline
// handles transitions (set → clear, change) correctly.
//
// NOTE: Role text values must be unique strings that do NOT appear anywhere
// in the fspec workflow guidance or provider base prompts (e.g. "architect"
// appears in fspec guidance — never use common software terms as test roles).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use codelet_tools::facade::{
    build_gemini_system_prompt, prepend_fspec_guidance,
    ClaudeApiKeySystemPromptFacade, ClaudeOAuthSystemPromptFacade,
    GeminiSystemPromptFacade, OpenAISystemPromptFacade, SystemPromptFacade,
};

/// Unique role text that cannot appear in any system prompt boilerplate.
const ROLE_A: &str = "xq7-paleontologist-reviewer";
const ROLE_B: &str = "zk9-marine-biologist-auditor";

// ============================================================================
// Scenario: Role is passed as preamble to create_rig_agent
// ============================================================================
//
// Tests that every provider's facade includes the role text in the effective
// system prompt when preamble is Some("role text").

#[test]
fn test_role_text_included_in_claude_oauth_system_prompt_via_preamble() {
    // @step Given a session with role set to "You are a security reviewer"
    let role_text = "You are a security reviewer";

    // @step When the agent loop creates a new agent for a turn via run_with_provider
    // Simulates: let role_preamble = session.get_role(); → Some(role_text)
    // Then: provider.create_rig_agent(id, role_preamble.as_deref(), ...) calls facade
    let facade = ClaudeOAuthSystemPromptFacade;
    let effective_preamble = facade.transform_preamble(role_text);

    // @step Then create_rig_agent receives the role text as the preamble parameter
    // @step And the system prompt includes "You are a security reviewer"
    assert!(
        effective_preamble.contains(role_text),
        "Effective preamble should contain role text. Got: {effective_preamble}"
    );
}

#[test]
fn test_role_text_included_in_claude_api_key_system_prompt_via_preamble() {
    // @step Given a session with role set to "You are a security reviewer"
    let role_text = "You are a security reviewer";

    // @step When the agent loop creates a new agent for a turn via run_with_provider
    let facade = ClaudeApiKeySystemPromptFacade;
    let effective_preamble = facade.transform_preamble(role_text);

    // @step Then create_rig_agent receives the role text as the preamble parameter
    // @step And the system prompt includes "You are a security reviewer"
    assert!(
        effective_preamble.contains(role_text),
        "Effective preamble should contain role text. Got: {effective_preamble}"
    );
}

#[test]
fn test_role_text_included_in_gemini_system_prompt_via_preamble() {
    // @step Given a session with role set to "You are a security reviewer"
    let role_text = "You are a security reviewer";

    // @step When the agent loop creates a new agent for a turn via run_with_provider
    // Gemini uses build_gemini_system_prompt(model, preamble) instead of a direct
    // facade.transform_preamble — test both paths.
    let facade = GeminiSystemPromptFacade;
    let effective_preamble = facade.transform_preamble(role_text);

    // @step Then create_rig_agent receives the role text as the preamble parameter
    // @step And the system prompt includes "You are a security reviewer"
    assert!(
        effective_preamble.contains(role_text),
        "Effective preamble should contain role text. Got: {effective_preamble}"
    );

    // Also verify through the concrete Gemini system prompt builder
    let gemini_prompt = build_gemini_system_prompt("gemini-2.0-flash", Some(role_text));
    assert!(
        gemini_prompt.contains(role_text),
        "Gemini system prompt should contain role text. Got: {gemini_prompt}"
    );
}

#[test]
fn test_role_text_included_in_openai_system_prompt_via_preamble() {
    // @step Given a session with role set to "You are a security reviewer"
    let role_text = "You are a security reviewer";

    // @step When the agent loop creates a new agent for a turn via run_with_provider
    // OpenAI uses prepend_fspec_guidance(preamble) via OpenAISystemPromptFacade
    let facade = OpenAISystemPromptFacade;
    let effective_preamble = facade.transform_preamble(role_text);

    // @step Then create_rig_agent receives the role text as the preamble parameter
    // @step And the system prompt includes "You are a security reviewer"
    assert!(
        effective_preamble.contains(role_text),
        "Effective preamble should contain role text. Got: {effective_preamble}"
    );
}

#[test]
fn test_role_text_included_in_zai_system_prompt_via_fspec_guidance() {
    // @step Given a session with role set to "You are a security reviewer"
    let role_text = "You are a security reviewer";

    // @step When the agent loop creates a new agent for a turn via run_with_provider
    // ZAI uses the same prepend_fspec_guidance path as OpenAI
    let effective_preamble = prepend_fspec_guidance(role_text);

    // @step Then create_rig_agent receives the role text as the preamble parameter
    // @step And the system prompt includes "You are a security reviewer"
    assert!(
        effective_preamble.contains(role_text),
        "ZAI effective preamble should contain role text. Got: {effective_preamble}"
    );
}

// ============================================================================
// Scenario: No role results in None preamble
// ============================================================================
//
// When session.get_role() returns None, the macro passes None to
// create_rig_agent. Each provider maps None → "" (or skips preamble
// entirely). The system prompt should contain only facade defaults with
// no custom role text.

#[test]
fn test_no_role_produces_default_facade_preamble_claude_oauth() {
    // @step Given a session with no role set
    // Role is None → role_preamble.as_deref() → None → provider unwraps to ""

    // @step When the agent loop creates a new agent for a turn via run_with_provider
    let facade = ClaudeOAuthSystemPromptFacade;
    let default_preamble = facade.transform_preamble("");

    // @step Then create_rig_agent receives None as the preamble parameter
    // @step And the system prompt contains only facade defaults
    assert!(
        default_preamble.contains("You are Claude Code"),
        "Default preamble should contain Claude Code prefix"
    );
    assert!(
        !default_preamble.contains(ROLE_A),
        "Default preamble should not contain any custom role text"
    );
}

#[test]
fn test_no_role_produces_default_facade_preamble_gemini() {
    // @step Given a session with no role set
    // @step When the agent loop creates a new agent for a turn via run_with_provider
    let gemini_prompt = build_gemini_system_prompt("gemini-2.0-flash", None);

    // @step Then create_rig_agent receives None as the preamble parameter
    // @step And the system prompt contains only facade defaults
    assert!(
        gemini_prompt.contains("software engineering"),
        "Default Gemini prompt should contain base system prompt"
    );
    assert!(
        !gemini_prompt.contains(ROLE_A),
        "Default Gemini prompt should not contain any custom role text"
    );
}

#[test]
fn test_no_role_produces_default_facade_preamble_openai() {
    // @step Given a session with no role set
    // @step When the agent loop creates a new agent for a turn via run_with_provider
    // OpenAI/ZAI: preamble.unwrap_or("") → prepend_fspec_guidance("")
    let default_preamble = prepend_fspec_guidance("");

    // @step Then create_rig_agent receives None as the preamble parameter
    // @step And the system prompt contains only facade defaults
    assert!(
        !default_preamble.is_empty(),
        "Default preamble should not be empty (contains fspec guidance)"
    );
    assert!(
        !default_preamble.contains(ROLE_A),
        "Default preamble should not contain any custom role text"
    );
}

// ============================================================================
// Scenario: Cleared role reverts to None preamble
// ============================================================================
//
// Exercises the full pipeline: role is set → facade includes it → role is
// cleared → facade no longer includes it.  Tests the actual facade output
// to prove the old role text disappears from the system prompt.

#[test]
fn test_cleared_role_reverts_facade_to_defaults() {
    // @step Given a session with role set to "architect"
    let facade = ClaudeOAuthSystemPromptFacade;
    let with_role = facade.transform_preamble(ROLE_A);
    assert!(
        with_role.contains(ROLE_A),
        "Preamble should contain role while role is set"
    );

    // @step When the role is cleared via clear_role
    // clear_role() sets the RwLock to None → get_role() returns None →
    // role_preamble.as_deref() is None → provider receives None → uses ""

    // @step And the agent loop creates a new agent for the next turn
    let cleared = facade.transform_preamble("");

    // @step Then create_rig_agent receives None as the preamble parameter
    assert!(
        !cleared.contains(ROLE_A),
        "Cleared preamble should NOT contain role text '{ROLE_A}'"
    );
    assert!(
        cleared.contains("You are Claude Code"),
        "Cleared preamble should still contain default Claude Code prefix"
    );
}

#[test]
fn test_cleared_role_reverts_gemini_to_defaults() {
    // @step Given a session with role set to "architect"
    let with_role = build_gemini_system_prompt("gemini-2.0-flash", Some(ROLE_A));
    assert!(with_role.contains(ROLE_A));

    // @step When the role is cleared via clear_role
    // @step And the agent loop creates a new agent for the next turn
    let cleared = build_gemini_system_prompt("gemini-2.0-flash", None);

    // @step Then create_rig_agent receives None as the preamble parameter
    assert!(
        !cleared.contains(ROLE_A),
        "Cleared Gemini prompt should NOT contain role text '{ROLE_A}'"
    );
}

#[test]
fn test_cleared_role_reverts_openai_to_defaults() {
    // @step Given a session with role set
    let with_role = prepend_fspec_guidance(ROLE_A);
    assert!(with_role.contains(ROLE_A));

    // @step When the role is cleared via clear_role
    // @step And the agent loop creates a new agent for the next turn
    let cleared = prepend_fspec_guidance("");

    // @step Then create_rig_agent receives None as the preamble parameter
    assert!(
        !cleared.contains(ROLE_A),
        "Cleared OpenAI preamble should NOT contain role text '{ROLE_A}'"
    );
}

// ============================================================================
// Scenario: Role change takes effect on next turn
// ============================================================================
//
// The run_with_provider! macro calls get_role() at the START of each turn.
// A role change via set_role("new") means the next turn's get_role() returns
// Some("new"), and the old role text is absent from the system prompt.

#[test]
fn test_role_change_replaces_old_role_in_claude_preamble() {
    // @step Given a session with role set to "architect"
    let facade = ClaudeOAuthSystemPromptFacade;
    let first_preamble = facade.transform_preamble(ROLE_A);
    assert!(first_preamble.contains(ROLE_A));

    // @step When the role is changed to "tester"
    // set_role(ROLE_B) → get_role() returns Some(ROLE_B) on next turn

    // @step And the agent loop creates a new agent for the next turn
    let second_preamble = facade.transform_preamble(ROLE_B);

    // @step Then create_rig_agent receives "tester" as the preamble parameter
    // @step And the system prompt includes "tester"
    assert!(
        second_preamble.contains(ROLE_B),
        "System prompt should contain new role '{ROLE_B}'. Got: {second_preamble}"
    );
    assert!(
        !second_preamble.contains(ROLE_A),
        "System prompt should NOT contain old role '{ROLE_A}'"
    );
}

#[test]
fn test_role_change_replaces_old_role_in_openai_preamble() {
    // @step Given a session with role set to "architect"
    let first = prepend_fspec_guidance(ROLE_A);
    assert!(first.contains(ROLE_A));

    // @step When the role is changed to "tester"
    // @step And the agent loop creates a new agent for the next turn
    let second = prepend_fspec_guidance(ROLE_B);

    // @step Then create_rig_agent receives "tester" as the preamble parameter
    // @step And the system prompt includes "tester"
    assert!(second.contains(ROLE_B));
    assert!(
        !second.contains(ROLE_A),
        "OpenAI preamble should NOT contain old role '{ROLE_A}'"
    );
}

#[test]
fn test_role_change_replaces_old_role_in_gemini_prompt() {
    // @step Given a session with role set to "architect"
    let first = build_gemini_system_prompt("gemini-2.0-flash", Some(ROLE_A));
    assert!(first.contains(ROLE_A));

    // @step When the role is changed to "tester"
    // @step And the agent loop creates a new agent for the next turn
    let second = build_gemini_system_prompt("gemini-2.0-flash", Some(ROLE_B));

    // @step Then create_rig_agent receives "tester" as the preamble parameter
    // @step And the system prompt includes "tester"
    assert!(second.contains(ROLE_B));
    assert!(
        !second.contains(ROLE_A),
        "Gemini prompt should NOT contain old role '{ROLE_A}'"
    );
}

// ============================================================================
// Scenario: Spawned subordinate with role has preamble set on first turn
// ============================================================================
//
// handle_spawn calls session.set_role(role_str) BEFORE the subordinate's
// agent loop starts.  The first call to run_with_provider! in the
// subordinate reads get_role() → Some(role) → passes as preamble.
// We verify the full facade pipeline produces a system prompt with the role.

#[test]
fn test_spawned_subordinate_role_flows_through_claude_facade() {
    // @step Given a supervisor session
    // @step When the supervisor spawns a subordinate with role "test-writer"
    // handle_spawn: session.set_role("test-writer".to_string())
    let role = "test-writer";

    // @step Then the subordinate session has role "test-writer" stored
    // Verified by session.get_role() == Some("test-writer")

    // @step And the subordinate's first agent turn passes "test-writer" as preamble to create_rig_agent
    // Claude OAuth path:
    let facade = ClaudeOAuthSystemPromptFacade;
    let effective = facade.transform_preamble(role);
    assert!(
        effective.contains("test-writer"),
        "Subordinate's Claude preamble should contain 'test-writer'. Got: {effective}"
    );
    // Also verify the API format (cache_control blocks) includes the role
    let api_format = facade.format_for_api(role);
    let all_text: String = api_format.as_array().unwrap()
        .iter()
        .filter_map(|b| b["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        all_text.contains("test-writer"),
        "Subordinate's Claude API format should contain 'test-writer'"
    );
}

#[test]
fn test_spawned_subordinate_role_flows_through_gemini_builder() {
    // @step Given a supervisor session
    // @step When the supervisor spawns a subordinate with role "test-writer"
    let role = "test-writer";

    // @step Then the subordinate session has role "test-writer" stored
    // @step And the subordinate's first agent turn passes "test-writer" as preamble to create_rig_agent
    // Gemini path:
    let prompt = build_gemini_system_prompt("gemini-2.0-flash", Some(role));
    assert!(
        prompt.contains("test-writer"),
        "Subordinate's Gemini prompt should contain 'test-writer'. Got: {prompt}"
    );
}

#[test]
fn test_spawned_subordinate_role_flows_through_openai_facade() {
    // @step Given a supervisor session
    // @step When the supervisor spawns a subordinate with role "test-writer"
    let role = "test-writer";

    // @step Then the subordinate session has role "test-writer" stored
    // @step And the subordinate's first agent turn passes "test-writer" as preamble to create_rig_agent
    // OpenAI/ZAI path:
    let effective = prepend_fspec_guidance(role);
    assert!(
        effective.contains("test-writer"),
        "Subordinate's OpenAI preamble should contain 'test-writer'. Got: {effective}"
    );
}

// ============================================================================
// Additional: Role in format_for_api (Claude cache_control blocks)
// ============================================================================

#[test]
fn test_role_text_appears_in_claude_oauth_format_for_api() {
    // Verify role text makes it through to the actual API payload (cache_control blocks)
    let role_text = "You are a documentation writer";
    let facade = ClaudeOAuthSystemPromptFacade;
    let api_format = facade.format_for_api(role_text);

    // The API format should be a JSON array with cache_control blocks
    assert!(api_format.is_array(), "Should be a JSON array");
    let arr = api_format.as_array().unwrap();

    // Collect all text content
    let all_text: String = arr
        .iter()
        .filter_map(|b| b["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        all_text.contains(role_text),
        "API format should contain role text. Got: {all_text}"
    );
}

#[test]
fn test_role_text_appears_in_claude_api_key_format_for_api() {
    let role_text = "You are a documentation writer";
    let facade = ClaudeApiKeySystemPromptFacade;
    let api_format = facade.format_for_api(role_text);

    // API key mode returns a JSON array too
    assert!(api_format.is_array(), "Should be a JSON array");
    let arr = api_format.as_array().unwrap();

    let all_text: String = arr
        .iter()
        .filter_map(|b| b["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        all_text.contains(role_text),
        "API format should contain role text. Got: {all_text}"
    );
}
