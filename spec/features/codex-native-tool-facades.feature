@done
@TOOL-015 @tools @facade-pattern @codelet @provider-abstraction
Feature: Codex-Native Tool Facades - Map Tools to Codex CLI Tool Schemas

  """
  Create codelet/tools/src/facade/codex.rs with CodexShellCommandFacade, CodexReadFileFacade,
  CodexListDirFacade, CodexGrepFilesFacade. Follow existing pattern from zai.rs and file_ops.rs.
  Update CodexProvider::create_rig_agent() to use FacadeToolWrapper for these 4 tools while
  keeping Write, Edit, Glob, AstGrep, AstGrepRefactor as direct tool registrations. Reuse
  existing OpenAIFspecFacade and OpenAIBridgeFacade for Codex.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Codex tool facades MUST implement existing facade traits (BashToolFacade, FileToolFacade, LsToolFacade, SearchToolFacade) to map Codex-native params to internal params
  #   2. Tool names MUST match Codex CLI exactly: shell_command, read_file, list_dir, grep_files (not shell, readFile, ls, etc.)
  #   3. Parameter schemas MUST match Codex CLI definitions from codex-rs/core/src/tools/spec.rs (e.g., read_file uses file_path not path, list_dir uses dir_path not path)
  #   4. Tools without a Codex equivalent (Glob, AstGrep, AstGrepRefactor, Write, Edit) MUST be kept with current naming since Codex has no native equivalent
  #   5. CodexProvider::create_rig_agent() MUST use FacadeToolWrapper for tools with Codex equivalents, replacing direct tool registration
  #
  # EXAMPLES:
  #   1. Codex model calls shell_command with {command: 'ls -la', workdir: '/tmp'} → CodexShellCommandFacade maps to InternalBashParams::Execute{command: 'ls -la'} with cwd='/tmp' → BashTool executes
  #   2. Codex model calls read_file with {file_path: '/src/main.rs', offset: 10, limit: 50} → CodexReadFileFacade maps to InternalFileParams::Read{file_path: '/src/main.rs', offset: Some(10), limit: Some(50)} → ReadTool executes
  #   3. Codex model calls list_dir with {dir_path: '/src', depth: 2} → CodexListDirFacade maps to InternalLsParams::List{path: Some('/src')} → LsTool executes
  #   4. Codex model calls grep_files with {pattern: 'TODO', include: '*.rs', path: '/src'} → CodexGrepFilesFacade maps to InternalSearchParams::Grep{pattern: 'TODO', path: Some('/src')} → GrepTool executes
  #
  # ========================================

  Background: User Story
    As a Codex provider user
    I want to use Codex-native tool names and schemas when the agent makes tool calls
    So that GPT-5.1-codex uses tools correctly since it was trained on these specific tool interfaces

  Scenario: CodexShellCommandFacade maps shell_command to InternalBashParams
    Given a CodexShellCommandFacade instance
    When the Codex model calls shell_command with command "ls -la" and workdir "/tmp"
    Then the facade maps to InternalBashParams::Execute with command "ls -la"
    And the facade tool name is "shell_command"
    And the facade provider is "codex"

  Scenario: CodexReadFileFacade maps read_file to InternalFileParams::Read
    Given a CodexReadFileFacade instance
    When the Codex model calls read_file with file_path "/src/main.rs" offset 10 and limit 50
    Then the facade maps to InternalFileParams::Read with file_path "/src/main.rs" offset 10 and limit 50
    And the facade tool name is "read_file"
    And the facade provider is "codex"

  Scenario: CodexListDirFacade maps list_dir to InternalLsParams::List
    Given a CodexListDirFacade instance
    When the Codex model calls list_dir with dir_path "/src"
    Then the facade maps to InternalLsParams::List with path "/src"
    And the facade tool name is "list_dir"
    And the facade provider is "codex"

  Scenario: CodexGrepFilesFacade maps grep_files to InternalSearchParams::Grep
    Given a CodexGrepFilesFacade instance
    When the Codex model calls grep_files with pattern "TODO" and path "/src"
    Then the facade maps to InternalSearchParams::Grep with pattern "TODO" and path "/src"
    And the facade tool name is "grep_files"
    And the facade provider is "codex"

  Scenario: Codex facades validate required parameters
    Given a CodexShellCommandFacade instance
    When the Codex model calls shell_command with missing command field
    Then the facade returns a validation error
    And the error identifies "shell_command" as the tool
    And the error mentions "command" as the missing field

  Scenario: Codex facades handle optional parameters gracefully
    Given a CodexReadFileFacade instance
    When the Codex model calls read_file with only file_path "/src/main.rs"
    Then the facade maps to InternalFileParams::Read with offset None and limit None

  Scenario: Codex tool schemas use additionalProperties false
    Given all Codex facade instances
    When their tool definitions are inspected
    Then each schema has additionalProperties set to false
    And each schema has the correct required fields
