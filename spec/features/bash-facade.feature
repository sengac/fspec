@TOOL-004 @high @tools @facade @gemini
Feature: Bash Facade

  """
  GeminiRunShellCommandFacade in codelet/tools/src/facade/bash.rs implements ToolFacade trait.
  Uses tool name 'run_shell_command' and maps {command} to BashArgs.
  Wrapped with BashToolFacadeWrapper and added to GeminiProvider.create_rig_agent().
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. GeminiRunShellCommandFacade MUST use tool name 'run_shell_command'
  #   2. GeminiRunShellCommandFacade MUST use flat JSON schema with {command: {type: 'string'}}
  #   3. GeminiRunShellCommandFacade MUST map Gemini {command} to BashArgs {command}
  #   4. GeminiRunShellCommandFacade MUST be wrapped with BashToolFacadeWrapper for rig integration
  #
  # EXAMPLES:
  #   1. Gemini sends {command: 'ls -la'} to tool 'run_shell_command' → facade maps to BashArgs{command: 'ls -la'} → BashTool executes
  #   2. GeminiRunShellCommandFacade provides flat schema {type: 'object', properties: {command: {type: 'string'}}, required: ['command']} - no oneOf
  #
  # ========================================

  Background: User Story
    As a LLM agent using Gemini provider
    I want to execute shell commands with Gemini-native tool names and flat schemas
    So that Gemini understands my tool calls without schema compatibility issues

  # Example 1: Gemini sends {command: 'ls -la'} → facade maps to BashArgs
  Scenario: Map Gemini run_shell_command parameters to internal BashTool format
    Given a GeminiRunShellCommandFacade is registered
    When Gemini sends parameters {command: 'ls -la'} to tool 'run_shell_command'
    Then the facade maps to BashArgs with command 'ls -la'
    And the base BashTool executes with the mapped parameters

  # Example 2: Flat schema without oneOf
  Scenario: GeminiRunShellCommandFacade provides flat JSON schema
    Given a GeminiRunShellCommandFacade is created
    When I request the tool definition
    Then the schema has type 'object' with properties containing {command: {type: 'string'}}
    And the schema does not contain 'oneOf' or nested action objects

  # Rule 4: Wrapper integration with rig
  Scenario: BashToolFacadeWrapper integrates with rig Tool trait
    Given a BashToolFacadeWrapper wrapping GeminiRunShellCommandFacade
    When I call name() on the wrapper
    Then it returns "run_shell_command"
    And when I call definition() it returns a flat schema with command parameter
