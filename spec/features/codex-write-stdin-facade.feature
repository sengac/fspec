@BUG-115
Feature: Codex facade maps write_stdin to unified exec tool

  """
  Follow the existing CodexExecCommandFacade pattern in codex.rs — ExecToolFacade struct with definition() returning ToolDefinition with JSON schema, and map_params() using param_extract helpers
  The ExecToolFacadeWrapper and InternalExecParams Write/Poll variants are already implemented in TOOL-016 — this card only creates the facade struct and registers it in the Codex provider
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. CodexWriteStdinFacade must implement ExecToolFacade and map Codex-native write_stdin tool (session_id: Number, chars: String, yield_time_ms: Number, max_output_tokens: Number) to InternalExecParams::Write or InternalExecParams::Poll
  #   2. When chars is non-empty, facade maps to InternalExecParams::Write with session_id (number→string), input=chars, yield_time_ms, max_output_tokens
  #   3. When chars is empty string or absent, facade maps to InternalExecParams::Poll with session_id, yield_time_ms, max_output_tokens (empty write = poll)
  #   4. session_id must be converted from Codex Number type to unified exec String type (e.g. 4237 → "4237")
  #   5. session_id is required — missing or null session_id returns ToolError::Validation with tool='write_stdin'
  #   6. The facade must be registered in Codex create_rig_agent using ExecToolFacadeWrapper, alongside existing shell and exec_command facades
  #   7. write_stdin schema must have additionalProperties: false to match existing Codex facade convention
  #
  # EXAMPLES:
  #   1. Codex model calls write_stdin with session_id=4237 and chars='print(42)\n' → facade maps to InternalExecParams::Write with session_id="4237", input="print(42)\n"
  #   2. Codex model calls write_stdin with session_id=4237 and chars='' (empty) → facade maps to InternalExecParams::Poll with session_id="4237"
  #   3. Codex model calls write_stdin with session_id=4237 and no chars field → facade maps to InternalExecParams::Poll (absent = empty = poll)
  #   4. Codex model calls write_stdin with session_id=4237, chars='exit()\n', yield_time_ms=5000, max_output_tokens=1024 → facade passes all optional params through
  #   5. Codex model calls write_stdin without session_id → facade returns ToolError::Validation with tool='write_stdin' mentioning 'session_id'
  #   6. write_stdin facade is registered via ExecToolFacadeWrapper in Codex create_rig_agent and appears in the agent tool list
  #
  # ========================================

  Background: User Story
    As a Codex LLM agent
    I want to call write_stdin to send input to a running PTY session
    So that I can interact with long-running processes like Python REPLs and debuggers

  # =========================================================================
  # Write action mapping (non-empty chars)
  # =========================================================================

  Scenario: CodexWriteStdinFacade maps non-empty chars to InternalExecParams::Write
    Given a CodexWriteStdinFacade instance
    When the Codex model calls write_stdin with session_id 4237 and chars "print(42)\n"
    Then the facade maps to InternalExecParams::Write with session_id "4237" and input "print(42)\n"
    And yield_time_ms is None
    And max_output_tokens is None

  Scenario: CodexWriteStdinFacade passes all optional params through
    Given a CodexWriteStdinFacade instance
    When the Codex model calls write_stdin with session_id 4237 chars "exit()\n" yield_time_ms 5000 and max_output_tokens 1024
    Then the facade maps to InternalExecParams::Write with session_id "4237" and input "exit()\n"
    And yield_time_ms is 5000
    And max_output_tokens is 1024

  # =========================================================================
  # Poll action mapping (empty or absent chars)
  # =========================================================================

  Scenario: CodexWriteStdinFacade maps empty chars to InternalExecParams::Poll
    Given a CodexWriteStdinFacade instance
    When the Codex model calls write_stdin with session_id 4237 and chars ""
    Then the facade maps to InternalExecParams::Poll with session_id "4237"
    And yield_time_ms is None
    And max_output_tokens is None

  Scenario: CodexWriteStdinFacade maps absent chars to InternalExecParams::Poll
    Given a CodexWriteStdinFacade instance
    When the Codex model calls write_stdin with session_id 4237 and no chars field
    Then the facade maps to InternalExecParams::Poll with session_id "4237"

  # =========================================================================
  # Session ID type conversion
  # =========================================================================

  Scenario: CodexWriteStdinFacade converts numeric session_id to string
    Given a CodexWriteStdinFacade instance
    When the Codex model calls write_stdin with session_id 99 and chars "hello"
    Then the facade maps to InternalExecParams::Write with session_id "99" and input "hello"

  # =========================================================================
  # Validation
  # =========================================================================

  Scenario: CodexWriteStdinFacade validates required session_id parameter
    Given a CodexWriteStdinFacade instance
    When the Codex model calls write_stdin without session_id
    Then the facade returns a validation error for tool "write_stdin" mentioning "session_id"

  Scenario: CodexWriteStdinFacade rejects null session_id
    Given a CodexWriteStdinFacade instance
    When the Codex model calls write_stdin with session_id null
    Then the facade returns a validation error for tool "write_stdin" mentioning "session_id"

  # =========================================================================
  # Schema
  # =========================================================================

  Scenario: CodexWriteStdinFacade schema has additionalProperties false
    Given a CodexWriteStdinFacade instance
    When the tool definition schema is inspected
    Then the schema has additionalProperties set to false
    And the schema has "session_id" in the required array
    And the tool name is "write_stdin"
    And the provider is "codex"

  # =========================================================================
  # Registration
  # =========================================================================

  @integration
  Scenario: write_stdin facade is registered in Codex create_rig_agent
    Given a CodexWriteStdinFacade instance registered via ExecToolFacadeWrapper
    When the Codex agent tool list is inspected
    Then the tool list contains "write_stdin"
