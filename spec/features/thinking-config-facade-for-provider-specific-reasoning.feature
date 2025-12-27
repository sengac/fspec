@done
@high
@tools
@facade-pattern
@provider-abstraction
@TOOL-009
Feature: Thinking Config Facade for Provider-Specific Reasoning Configuration

  """
  ThinkingConfigFacade trait with provider(), request_config(ThinkingLevel), is_thinking_part(part), extract_thinking_text(part) methods. ThinkingLevel enum: Off, Low, Medium, High. Provider implementations: Gemini3ThinkingFacade (uses thinkingLevel enum), Gemini25ThinkingFacade (uses thinkingBudget token count), ClaudeThinkingFacade (uses thinking.budget_tokens). Located in codelet/tools/src/facade/thinking_config.rs. NAPI bindings expose getThinkingConfig() and isThinkingContent() to TypeScript via codelet/napi. See attached plan: thinking-config-facade-implementation-plan.md
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Each ThinkingConfigFacade MUST implement the trait with provider(), request_config(), is_thinking_part(), and extract_thinking_text() methods
  #   2. ThinkingLevel enum MUST provide semantic levels (Off, Low, Medium, High) that map to provider-specific configurations
  #   3. Gemini 3 facade MUST use thinkingLevel enum ('high') instead of thinkingBudget (token count)
  #   4. Gemini 2.5 facade MUST use thinkingBudget (token count) instead of thinkingLevel enum
  #   5. Claude facade MUST use thinking.type='enabled' with budget_tokens for extended thinking
  #   6. Response parsing MUST identify thinking content via provider-specific markers (thought:true for Gemini, thinking blocks for Claude)
  #   7. TypeScript interfaces MUST be exposed via NAPI bindings allowing TypeScript code to configure thinking options
  #
  # EXAMPLES:
  #   1. Gemini3ThinkingFacade with ThinkingLevel::High returns {thinkingConfig: {includeThoughts: true, thinkingLevel: 'high'}}
  #   2. Gemini25ThinkingFacade with ThinkingLevel::High returns {thinkingConfig: {includeThoughts: true, thinkingBudget: 8192}}
  #   3. ClaudeThinkingFacade with ThinkingLevel::High returns {thinking: {type: 'enabled', budget_tokens: 32000}}
  #   4. Gemini response part with thought:true is identified as thinking content by is_thinking_part()
  #   5. TypeScript calls ThinkingConfig.forProvider('gemini-3', ThinkingLevel.High) to get request config JSON
  #   6. ThinkingLevel::Off returns empty object {} for all providers (no thinking configuration)
  #
  # ========================================

  Background: User Story
    As a developer integrating LLM providers
    I want to configure provider-specific thinking/reasoning settings through a common interface
    So that each provider receives correct thinking configuration without duplicating logic across provider implementations

  Scenario: Gemini 3 facade generates thinkingLevel configuration for High level
    Given a Gemini3ThinkingFacade
    And ThinkingLevel::High is requested
    When I call request_config with the level
    Then the result should contain thinkingConfig.thinkingLevel set to "high"
    And the result should contain thinkingConfig.includeThoughts set to true
    And the result should NOT contain thinkingBudget

  Scenario: Gemini 2.5 facade generates thinkingBudget configuration for High level
    Given a Gemini25ThinkingFacade
    And ThinkingLevel::High is requested
    When I call request_config with the level
    Then the result should contain thinkingConfig.thinkingBudget set to 8192
    And the result should contain thinkingConfig.includeThoughts set to true
    And the result should NOT contain thinkingLevel

  Scenario: Claude facade generates thinking configuration with budget_tokens
    Given a ClaudeThinkingFacade
    And ThinkingLevel::High is requested
    When I call request_config with the level
    Then the result should contain thinking.type set to "enabled"
    And the result should contain thinking.budget_tokens set to 32000

  Scenario: Gemini facade identifies thinking parts in response
    Given a Gemini3ThinkingFacade
    And a response part with "thought" field set to true
    When I call is_thinking_part with the part
    Then it should return true
    And extract_thinking_text should return the text content

  Scenario: Gemini facade ignores non-thinking parts
    Given a Gemini3ThinkingFacade
    And a response part without "thought" field
    When I call is_thinking_part with the part
    Then it should return false
    And extract_thinking_text should return None

  Scenario: ThinkingLevel Off returns empty configuration for all providers
    Given any ThinkingConfigFacade implementation
    And ThinkingLevel::Off is requested
    When I call request_config with the level
    Then the result should be an empty object

  Scenario: TypeScript can get thinking configuration via NAPI bindings
    Given the NAPI getThinkingConfig function
    And provider "gemini-3" and ThinkingLevel.High
    When I call getThinkingConfig from TypeScript
    Then I should receive a JSON string with the correct configuration
    And the parsed JSON should match the Rust facade output

  Scenario: TypeScript can check if content is thinking via NAPI bindings
    Given the NAPI isThinkingContent function
    And a Gemini response part JSON string with thought:true
    When I call isThinkingContent from TypeScript
    Then it should return true
