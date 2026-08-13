@CLI-017
Feature: CLI-017: Add Anthropic prompt cache control metadata
  As a CLI user with long conversations
  I want to have effective prompt caching with Anthropic Claude
  So that I save on API costs and get faster responses due to cache hits

  Background:
    Given the codelet CLI is configured with Anthropic Claude provider
    And the anthropic-beta header includes "prompt-caching-2024-07-31"

  # Problem statement:
  # Rig sends system prompts as plain strings, but Anthropic's cache_control
  # requires the system field to be an array of content blocks with metadata.
  # Using additional_params to override the system field with the proper array
  # format enables prompt caching.

  # ==========================================
  # SYSTEM PROMPT FORMAT TESTS
  # ==========================================

  Scenario: System prompt is sent as array with cache_control for API key mode
    Given the provider is using API key authentication
    And the preamble is "You are a helpful coding assistant"
    When building the completion request
    Then the system field should be an array of content blocks
    And the first content block should have type "text"
    And the first content block should have cache_control with type "ephemeral"
    And the first content block text should be "You are a helpful coding assistant"

  Scenario: System prompt is sent with correct structure for OAuth mode
    Given the provider is using OAuth authentication
    And the system instructions are "Additional instructions here"
    When building the completion request
    Then the system field should be an array with 2 content blocks
    And the first content block should be the Claude Code prefix without cache_control
    And the second content block should have cache_control with type "ephemeral"
    And the second content block text should be "Additional instructions here"

  Scenario: Additional params override plain string system field
    Given a completion request with preamble set via .preamble()
    When additional_params includes a system array
    Then the system array from additional_params should be used
    And the plain string from preamble should be ignored

  # ==========================================
  # CACHE_CONTROL STRUCTURE TESTS
  # ==========================================

  Scenario: cache_control uses ephemeral type
    Given a system content block with cache_control
    Then the cache_control should have type "ephemeral"
    And the JSON structure should be { "cache_control": { "type": "ephemeral" } }

  Scenario: Content block without cache_control omits the field
    Given the OAuth mode first block (Claude Code prefix)
    When serializing the content block
    Then the cache_control field should be absent
    And the JSON should only have "type" and "text" fields

  # ==========================================
  # HELPER FUNCTION TESTS
  # ==========================================

  Scenario: build_cached_system_prompt creates correct array format
    Given a preamble string "Test preamble"
    And no OAuth prefix is needed
    When calling build_cached_system_prompt
    Then the result should be a serde_json::Value array
    And it should contain one content block
    And that block should have cache_control

  Scenario: build_cached_system_prompt handles OAuth mode correctly
    Given a preamble string "Additional instructions"
    And OAuth mode is active
    When calling build_cached_system_prompt with OAuth prefix
    Then the result should have 2 content blocks
    And block 0 should be the OAuth prefix without cache_control
    And block 1 should be the preamble with cache_control

  # ==========================================
  # INTEGRATION TESTS
  # ==========================================

  Scenario: Completion request includes cache_control in serialized JSON
    Given a completion request built by the provider
    When serializing to JSON for the Anthropic API
    Then the JSON should contain "cache_control" nested in system blocks
    And the structure should match Anthropic's expected format
