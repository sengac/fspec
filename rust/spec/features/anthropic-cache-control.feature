Feature: CLI-017: Anthropic Prompt Cache Control Metadata
  As a CLI user with Anthropic provider
  I want system prompts to have cache_control metadata
  So that prompt caching is utilized effectively for lower latency and cost

  Background:
    Given the Claude provider is configured
    And the anthropic-beta header includes "prompt-caching-2024-07-31"

  # BLOCKED: rig 0.25.0 sends system prompts as plain strings
  #
  # Root Cause:
  # In rig-core-0.25.0/src/providers/anthropic/completion.rs lines 720-735:
  #   struct AnthropicCompletionRequest {
  #       system: String,  // <-- Plain string, not array with cache_control
  #       ...
  #   }
  #
  # Anthropic API expects for cache control:
  #   {
  #     "system": [
  #       {"type": "text", "text": "...", "cache_control": {"type": "ephemeral"}}
  #     ]
  #   }
  #
  # But rig sends:
  #   {
  #     "system": "..."  // Plain string
  #   }
  #
  # Resolution Options:
  # 1. Submit PR to rig to support structured system prompts
  # 2. Wait for rig update
  # 3. Fork rig (not recommended)
  #
  # Status: BLOCKED pending rig PR

  @blocked @rig-limitation
  Scenario: System prompt includes cache_control metadata
    Given an Anthropic completion request is being built
    When the system prompt is set
    Then the system field should be an array of content blocks
    And each block should have "cache_control": {"type": "ephemeral"}

  @blocked @rig-limitation
  Scenario: First user message includes cache_control metadata
    Given an Anthropic completion request is being built
    When user messages are added
    Then the first user message content should include cache_control
    And the cache_control type should be "ephemeral"

  # This scenario DOES work - we have the beta header
  Scenario: Beta header enables prompt caching
    Given the ClaudeProvider is initialized
    Then the anthropic-beta header should include "prompt-caching-2024-07-31"
    And the provider should report supports_caching() = true
