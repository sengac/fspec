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

  Scenario: Split interactive.rs into focused submodules
    Given the cli crate contains interactive.rs with 845 lines
    When I refactor interactive.rs into the interactive/ directory
    Then interactive/mod.rs should contain public API and orchestration
    And interactive/repl_loop.rs should contain the main REPL loop logic
    And interactive/stream_handler.rs should contain response streaming
    And interactive/tool_executor.rs should contain tool invocation handling
    And interactive/token_display.rs should contain token usage display
    And no file in interactive/ should exceed 250 lines

  Scenario: Split compaction.rs into focused submodules
    Given the core crate contains compaction.rs with 699 lines
    When I refactor compaction.rs into the compaction/ directory
    Then compaction/mod.rs should contain public re-exports
    And compaction/model.rs should contain TokenTracker and ConversationTurn types
    And compaction/anchor.rs should contain anchor point detection logic
    And compaction/compactor.rs should contain the compaction algorithm
    And compaction/metrics.rs should contain CompactionResult types
    And no file in compaction/ should exceed 200 lines

  Scenario: Split debug_capture.rs into focused submodules
    Given the common crate contains debug_capture.rs with 716 lines
    When I refactor debug_capture.rs into the debug_capture/ directory
    Then debug_capture/mod.rs should contain public API and manager access
    And debug_capture/capture.rs should contain capture type definitions
    And debug_capture/storage.rs should contain file I/O operations
    And debug_capture/formatting.rs should contain JSON and display formatting
    And the DebugCaptureManager should be injectable for testing
    And no file in debug_capture/ should exceed 200 lines

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
