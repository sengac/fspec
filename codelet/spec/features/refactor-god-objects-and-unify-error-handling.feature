@core
@architecture
@refactoring
@REFAC-013
Feature: Refactor God Objects and Unify Error Handling

  """
  Detailed refactoring plan documented in spec/docs/REFAC-013-architectural-refactoring-plan.md with phased approach: (1) Split god objects, (2) Unify error handling, (3) Extract provider patterns, (4) Additional improvements
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. No source file should exceed 250 lines of code
  #   2. Each module must have a single, clear responsibility
  #   3. All tools must use the unified ToolError type
  #   4. All providers must use the unified ProviderError type
  #   5. Public API must remain backwards compatible
  #   6. Provider implementations must use shared adapter patterns to reduce duplication
  #
  # EXAMPLES:
  #   1. interactive.rs (845 lines) is split into interactive/mod.rs, repl_loop.rs, stream_handler.rs, tool_executor.rs, token_display.rs - each under 250 lines
  #   2. BashTool returns ToolError::Timeout { tool: bash, seconds: 30 } instead of BashError::TimeoutError(30)
  #   3. ClaudeProvider implements ProviderAdapter trait, reusing detect_env_credential() default for API key detection
  #
  # ========================================

  Background: User Story
    As a developer maintaining the codelet codebase
    I want to have well-organized, focused modules with consistent error handling
    So that the codebase is easier to understand, test, and extend

  Scenario: Unify tool error types
    Given each tool has its own error enum (BashError, ReadError, WriteError, etc.)
    When I create a unified ToolError type in tools/src/error.rs
    Then all tools should return ToolError instead of individual error enums
    And ToolError should include the tool name for debugging
    And ToolError should have an is_retryable() method
    And the individual error enums should be removed

  Scenario: Unify provider error types
    Given providers use anyhow::Result everywhere
    When I create a unified ProviderError type in providers/src/error.rs
    Then all providers should return typed ProviderError instead of anyhow errors
    And ProviderError should distinguish authentication, API, and rate limit errors
    And rate limit errors should enable automatic retry logic

  Scenario: Extract common provider patterns into adapter trait
    Given provider implementations have 40% code duplication
    When I create a ProviderAdapter trait with default implementations
    Then ClaudeProvider should implement ProviderAdapter
    And OpenAIProvider should implement ProviderAdapter
    And GeminiProvider should implement ProviderAdapter
    And CodexProvider should implement ProviderAdapter
    And duplicated auth detection logic should use detect_env_credential() default
    And provider implementations should be reduced by at least 30%

  Scenario: Verify no code is lost during file splitting
    Given a source file with a known line count
    When the file is split into multiple focused modules
    Then the total line count of all split files should equal or exceed the original
    And cargo build should succeed with no missing symbols
    And cargo test should pass with no behavioral regressions
    And cargo clippy should report no dead code warnings

  Scenario: Public API remains backwards compatible
    Given the existing public API is used by external code
    When all refactoring is complete
    Then all existing imports from cli, core, providers, tools crates should still work
    And cargo build should succeed without API breakage warnings
    And cargo test should pass with no regressions
