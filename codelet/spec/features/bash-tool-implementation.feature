@bash-execution
@tool-execution
@CORE-003
Feature: Bash Tool Implementation

  """
  Implements Tool trait with name='Bash', single 'command' parameter required
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Bash tool must execute commands using tokio process spawning with UTF-8 encoding
  #   2. Command output must be truncated at 30000 characters maximum (OUTPUT_LIMITS.MAX_OUTPUT_CHARS)
  #   3. Lines longer than 2000 characters must be replaced with '[Omitted long line]'
  #   4. Command execution must have a configurable timeout (default 120 seconds)
  #   5. Failed commands must return combined stdout and stderr with error message
  #   6. Truncated output must include warning (format: '... [N lines truncated - output truncated at 30000 chars] ...')
  #   7. BashTool must implement the Tool trait and register in ToolRegistry.with_core_tools()
  #
  # EXAMPLES:
  #   1. Execute 'echo hello' returns 'hello' with newline
  #   2. Execute 'ls /nonexistent' returns error with stderr content
  #   3. Execute command generating 50000 chars truncates to 30000 chars with warning
  #   4. Execute command with line > 2000 chars shows '[Omitted long line]' instead
  #   5. Execute 'sleep 200' with 1 second timeout returns timeout error
  #   6. Runner.new() includes BashTool in available_tools() list
  #   7. runner.execute_tool('Bash', {command: 'pwd'}) returns current directory
  #
  # ========================================

  Background: User Story
    As a AI coding agent
    I want to execute shell commands on the host system
    So that I can build projects, run tests, install dependencies, and perform system operations to assist developers

  Scenario: Execute simple command successfully
    Given the Bash tool is available
    When I execute the Bash tool with command "echo hello"
    Then the output should contain "hello"
    And the result should not be an error

  Scenario: Execute command that fails returns error with stderr
    Given the Bash tool is available
    When I execute the Bash tool with command "ls /nonexistent_directory_12345"
    Then the result should be an error
    And the output should contain error information

  Scenario: Long output is truncated at character limit
    Given the Bash tool is available
    When I execute the Bash tool with a command that generates over 30000 characters
    Then the output should be truncated to at most 30000 characters
    And the output should contain a truncation warning

  Scenario: Long lines are replaced with omission message
    Given the Bash tool is available
    When I execute the Bash tool with a command that outputs a line over 2000 characters
    Then the output should contain "[Omitted long line]"

  Scenario: Command timeout returns error
    Given the Bash tool is available with a 1 second timeout
    When I execute the Bash tool with command "sleep 10"
    Then the result should be an error
    And the output should indicate a timeout occurred

  Scenario: BashTool is registered in default ToolRegistry
    Given a default ToolRegistry
    Then the registry should contain the "Bash" tool
    And the Bash tool should have the correct name and description

  Scenario: Runner can execute Bash tool
    Given a Runner with default tools
    When I execute the Bash tool through the runner with command "pwd"
    Then the output should contain the current working directory
    And the result should not be an error
