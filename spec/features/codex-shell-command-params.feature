@BUG-108
Feature: Codex shell_command facade ignores workdir and timeout_ms params
  """
  Uses the existing codex.rs patterns — add fields to InternalBashParams::Execute, update map_params, update BashToolFacadeWrapper::call match arm
  timeout_ms is stored in the params for future use but not enforced in BashTool yet — BashTool does not currently support per-command timeouts
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. InternalBashParams::Execute must carry optional cwd and timeout_ms fields so facades can pass them through to BashTool
  #   2. CodexShellCommandFacade::map_params must extract workdir and map it to InternalBashParams::Execute { cwd } field
  #   3. CodexShellCommandFacade::map_params must extract timeout_ms and map it to InternalBashParams::Execute { timeout_ms } field
  #   4. BashToolFacadeWrapper::call must pass InternalBashParams cwd to BashArgs.cwd instead of hardcoding None
  #   5. Existing facades (Gemini, ZAI) that only set command must continue working — their InternalBashParams will have cwd=None and timeout_ms=None
  #   6. The shell_command schema must include login (Boolean), sandbox_permissions (String), justification (String), and prefix_rule (Array<String>) parameters for Codex model compatibility — accepted but silently ignored
  #   7. Session isolation effective_cwd (TOOL-013) takes precedence over facade-provided cwd — facade cwd is only used as fallback when no session isolation is active
  #
  # EXAMPLES:
  #   1. Codex model sends {command:'make test', workdir:'/project'} → facade maps to InternalBashParams::Execute{command:'make test', cwd:Some('/project'), timeout_ms:None} → BashToolFacadeWrapper passes cwd to BashArgs
  #   2. Codex model sends {command:'sleep 100', timeout_ms:5000} → facade maps to InternalBashParams::Execute{command:'sleep 100', cwd:None, timeout_ms:Some(5000)}
  #   3. Codex model sends {command:'ls', login:true, sandbox_permissions:'use_default'} → facade accepts these params via the schema but they are not mapped to InternalBashParams fields
  #   4. Gemini model sends {command:'ls'} via GeminiRunShellCommandFacade → InternalBashParams::Execute{command:'ls', cwd:None, timeout_ms:None} — backward compatible
  #   5. Codex model sends {command:'pwd', workdir:'/tmp'} but session isolation effective_cwd is /worktree/abc → BashTool uses /worktree/abc (isolation wins)
  #
  # ========================================
  Background: User Story
    As a Codex model (GPT-5.1-codex)
    I want to send shell_command with workdir and timeout_ms parameters
    So that commands execute in the correct directory with proper timeout limits

  Scenario: CodexShellCommandFacade maps workdir to InternalBashParams cwd
    Given a CodexShellCommandFacade instance
    When the Codex model calls shell_command with command "make test" and workdir "/project"
    Then the facade maps to InternalBashParams::Execute with command "make test" and cwd "/project"
    And timeout_ms is None

  Scenario: CodexShellCommandFacade maps timeout_ms to InternalBashParams
    Given a CodexShellCommandFacade instance
    When the Codex model calls shell_command with command "sleep 100" and timeout_ms 5000
    Then the facade maps to InternalBashParams::Execute with command "sleep 100" and timeout_ms 5000
    And cwd is None

  Scenario: CodexShellCommandFacade maps both workdir and timeout_ms
    Given a CodexShellCommandFacade instance
    When the Codex model calls shell_command with command "npm test" workdir "/app" and timeout_ms 30000
    Then the facade maps to InternalBashParams::Execute with command "npm test" cwd "/app" and timeout_ms 30000

  Scenario: CodexShellCommandFacade without optional params defaults to None
    Given a CodexShellCommandFacade instance
    When the Codex model calls shell_command with only command "echo hello"
    Then the facade maps to InternalBashParams::Execute with command "echo hello"
    And cwd is None
    And timeout_ms is None

  Scenario: Codex shell_command schema includes Codex-native approval params
    Given a CodexShellCommandFacade instance
    When the tool definition schema is inspected
    Then the schema has a "login" property of type "boolean"
    And the schema has a "sandbox_permissions" property of type "string"
    And the schema has a "justification" property of type "string"
    And the schema has a "prefix_rule" property of type "array"

  Scenario: Codex-native approval params are silently ignored in map_params
    Given a CodexShellCommandFacade instance
    When the Codex model calls shell_command with command "ls" and login true and sandbox_permissions "use_default"
    Then the facade maps to InternalBashParams::Execute with command "ls"
    And cwd is None
    And timeout_ms is None

  Scenario: Existing facades remain backward compatible with new InternalBashParams fields
    Given a GeminiRunShellCommandFacade instance
    When the Gemini model calls run_shell_command with command "ls"
    Then the facade maps to InternalBashParams::Execute with command "ls"
    And cwd is None
    And timeout_ms is None
