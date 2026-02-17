@done
@tui
@session
@NAPI-010
Feature: Refactor StreamChunk to use proper discriminated union types
  """
  Rust Changes:
  - types.rs: Convert StreamChunk from struct to #[napi(discriminant = "type")] enum
  - session_manager.rs: set_status() emits SessionStateChange variant for state transitions
  - output.rs: emit_status() maps to UserNotification for user-visible messages

  TypeScript Changes:
  - AgentView.tsx: Replace string parsing with exhaustive switch on chunk.type
  - SessionStateChange updates internal state only, UserNotification adds to conversation
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. StreamChunk must use #[napi(discriminant = "type")] enum pattern for proper TypeScript discriminated union generation
  #   2. SessionStateChange chunks are internal state machine updates and must NOT be added to conversation
  #   3. UserNotification chunks are user-facing messages and must be displayed in conversation
  #   4. TypeScript handlers must use exhaustive switch statements on chunk.type - no string parsing allowed
  #
  # EXAMPLES:
  #   1. When Rust emits SessionStateChange{state: Compacting}, TypeScript updates isCompacting state but does NOT add to conversation
  #   2. When Rust emits UserNotification{message: 'API rate limit exceeded', severity: Warning}, TypeScript displays it in conversation as a status message
  #   3. CURRENT BUG: Running /compact shows 'compacting' in conversation because Status chunk with status='compacting' slips through string filter
  #
  # ========================================
  Background: User Story
    As a developer maintaining the TUI
    I want to handle StreamChunk events without string parsing
    So that I get compile-time type safety and can't accidentally display internal state changes in the conversation

  Scenario: SessionStateChange chunk updates state without adding to conversation
    Given a StreamChunk handler processes incoming chunks from Rust
    When Rust emits a SessionStateChange chunk with state Compacting
    Then the handler updates isCompacting state to true
    And no message is added to the conversation

  Scenario: UserNotification chunk displays message in conversation
    Given a StreamChunk handler processes incoming chunks from Rust
    When Rust emits a UserNotification chunk with message 'API rate limit exceeded' and severity Warning
    Then a status message 'API rate limit exceeded' is added to the conversation

  Scenario: Compacting state change does not appear in conversation
    Given I have an active session with conversation history
    When I run the /compact command
    Then no 'compacting' message appears in the conversation area
    And the compaction progress is shown only in the input area placeholder

  Scenario: StreamChunk handler uses exhaustive switch without string parsing
    Given the StreamChunk type is defined as a discriminated union with type field
    When the TypeScript handler processes any StreamChunk variant
    Then it uses a switch statement on chunk.type with no string includes or substring matching
