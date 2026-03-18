@BUG-114
Feature: Codex facade maps shell and exec_command to unified exec tool
  """
  Follow the existing CodexShellCommandFacade pattern in codex.rs — ExecToolFacade struct with definition() returning ToolDefinition with JSON schema, and map_params() using param_extract helpers
  The ExecToolFacadeWrapper and InternalExecParams are already implemented in TOOL-016 — this card only creates the two facade structs and registers them in the provider
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. CodexShellFacade must implement ExecToolFacade and map Codex-native shell tool (command: Array<String>, workdir, timeout_ms) to InternalExecParams::Run with command as JSON array and tty=false
  #   2. CodexExecCommandFacade must implement ExecToolFacade and map Codex-native exec_command tool (cmd, workdir, shell, tty, yield_time_ms, max_output_tokens, login) to InternalExecParams::Run
  #   3. CodexShellFacade timeout_ms must be converted to timeout_secs (ms / 1000) since unified exec uses seconds
  #   4. CodexExecCommandFacade must pass tty, yield_time_ms, and max_output_tokens directly through to InternalExecParams::Run (matching unified exec param names)
  #   5. Both facades must be registered in the Codex provider create_rig_agent using ExecToolFacadeWrapper, replacing or supplementing the existing BashToolFacadeWrapper for shell_command
  #   6. Codex-native login, shell, sandbox_permissions, justification, prefix_rule params must be accepted in the schema for model compatibility but silently ignored in map_params
  #   7. exec_command response must include session_id when the process is still running so the model can follow up with write_stdin (BUG-115)
  #   8. Both facade schemas must have additionalProperties: false to match existing Codex facade convention
  #
  # EXAMPLES:
  #   1. Codex model calls shell with command: ['ls', '-la'] and workdir: '/tmp' → facade maps to InternalExecParams::Run with command as JSON array ['ls', '-la'], tty=false, workdir=/tmp
  #   2. Codex model calls shell with command: ['git', 'status'] and timeout_ms: 5000 → facade maps timeout_ms 5000 to timeout_secs 5
  #   3. Codex model calls exec_command with cmd: 'python3', tty: true, yield_time_ms: 5000 → facade maps to InternalExecParams::Run with command as string 'python3', tty=true, yield_time_ms=5000
  #   4. Codex model calls exec_command with only cmd: 'ls' → facade maps with tty=false (default), yield_time_ms=None, max_output_tokens=None
  #   5. Codex model calls shell with missing command field → facade returns ToolError::Validation with tool='shell'
  #   6. Codex model calls exec_command with missing cmd field → facade returns ToolError::Validation with tool='exec_command'
  #   7. Both shell and exec_command facades are registered in Codex create_rig_agent using ExecToolFacadeWrapper and appear in the agent's tool list
  #
  # ========================================
  Background: User Story
    As a Codex LLM
    I want to call shell and exec_command tools with Codex-native schemas
    So that I can execute commands and manage PTY sessions using the tool names I was trained on

  # ==============================
  # shell facade
  # ==============================
  @shell
  Scenario: CodexShellFacade maps shell command array to InternalExecParams::Run
    Given a CodexShellFacade instance
    When the Codex model calls shell with command ["ls", "-la"] and workdir "/tmp"
    Then the facade maps to InternalExecParams::Run with command as JSON array ["ls", "-la"]
    And tty is false
    And workdir is "/tmp"
    And the facade tool name is "shell"
    And the facade provider is "codex"

  @shell
  Scenario: CodexShellFacade converts timeout_ms to timeout_secs
    Given a CodexShellFacade instance
    When the Codex model calls shell with command ["git", "status"] and timeout_ms 5000
    Then the facade maps to InternalExecParams::Run with timeout_secs 5

  @shell
  Scenario: CodexShellFacade without optional params defaults to None
    Given a CodexShellFacade instance
    When the Codex model calls shell with only command ["echo", "hello"]
    Then the facade maps to InternalExecParams::Run with workdir None
    And timeout_secs is None
    And yield_time_ms is None

  @shell
  @validation
  Scenario: CodexShellFacade validates required command parameter
    Given a CodexShellFacade instance
    When the Codex model calls shell with missing command field
    Then the facade returns a validation error for tool "shell" mentioning "command"

  @shell
  @schema
  Scenario: CodexShellFacade schema has additionalProperties false
    Given a CodexShellFacade instance
    When the tool definition schema is inspected
    Then the schema has additionalProperties set to false
    And the required array contains only "command"
    And command property type is "array" with items type "string"

  # ==============================
  # exec_command facade
  # ==============================
  @exec_command
  Scenario: CodexExecCommandFacade maps exec_command with PTY to InternalExecParams::Run
    Given a CodexExecCommandFacade instance
    When the Codex model calls exec_command with cmd "python3" tty true and yield_time_ms 5000
    Then the facade maps to InternalExecParams::Run with command as string "python3"
    And tty is true
    And yield_time_ms is 5000
    And the facade tool name is "exec_command"
    And the facade provider is "codex"

  @exec_command
  Scenario: CodexExecCommandFacade defaults to tty false when not specified
    Given a CodexExecCommandFacade instance
    When the Codex model calls exec_command with only cmd "ls"
    Then the facade maps to InternalExecParams::Run with tty false
    And yield_time_ms is None
    And max_output_tokens is None

  @exec_command
  Scenario: CodexExecCommandFacade maps all optional params
    Given a CodexExecCommandFacade instance
    When the Codex model calls exec_command with cmd "python3" workdir "/app" tty true yield_time_ms 10000 and max_output_tokens 4096
    Then the facade maps to InternalExecParams::Run with command "python3"
    And workdir is "/app"
    And tty is true
    And yield_time_ms is 10000
    And max_output_tokens is 4096

  @exec_command
  @validation
  Scenario: CodexExecCommandFacade validates required cmd parameter
    Given a CodexExecCommandFacade instance
    When the Codex model calls exec_command with missing cmd field
    Then the facade returns a validation error for tool "exec_command" mentioning "cmd"

  @exec_command
  Scenario: CodexExecCommandFacade silently ignores Codex-native approval params
    Given a CodexExecCommandFacade instance
    When the Codex model calls exec_command with cmd "ls" and login true and shell "/bin/bash"
    Then the facade maps to InternalExecParams::Run with command "ls"
    And tty is false

  @exec_command
  @schema
  Scenario: CodexExecCommandFacade schema has additionalProperties false
    Given a CodexExecCommandFacade instance
    When the tool definition schema is inspected
    Then the schema has additionalProperties set to false
    And the required array contains only "cmd"
    And the schema has properties for cmd workdir shell tty yield_time_ms max_output_tokens and login

  # ==============================
  # Integration (verified via codex provider tests)
  # ==============================
  @integration
  Scenario: Both facades are registered in Codex create_rig_agent
    Given a CodexShellFacade and CodexExecCommandFacade instance
    When the Codex tool name list is inspected
    Then "shell" is present and maps command as array type
    And "exec_command" is present and maps cmd as string type
