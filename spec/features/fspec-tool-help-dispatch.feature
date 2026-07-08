@done
@tools
@tool-execution
@rust
@dispatch
@bug-fix
@help-system
@RPC-414
Feature: Fspec tool help unreachable in TUI — Rust dispatcher returns UnknownCommand for command help requests
  """
  New help routing lives in a dedicated module codelet/fspec-core/src/help_dispatch.rs (under 300 LoC); dispatch_command calls it once before the canonical lookup and returns early on Some(result).
  Per-command help resolves kebab command name to its CONFIG via an explicit static name->&CommandHelpConfig table (mirroring help/configs/mod.rs) and renders with format_command_help. args_json is parsed defensively with serde_json; missing/blank/invalid => treated as no args.command.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. dispatch_command MUST recognize a help request BEFORE the canonical command lookup, so help never falls through to UnknownCommand
  #   2. A command string of the form '<command> --help' or '<command> -h' MUST render the per-command usage doc for <command> (the trailing flag is stripped and used only as the help signal)
  #   3. command 'help' with an args.command field MUST render the per-command usage doc for that named command
  #   4. command 'help' with no args.command MUST render general Fspec tool help describing how to get per-command help
  #   5. Per-command help MUST be rendered from the existing help registry (help/configs CONFIG + format_command_help); no new help text is authored and no TS content is duplicated
  #   6. A help request naming a real canonical command that has NO CONFIG MUST return success with a graceful 'no detailed help available' message, NOT an UnknownCommand error
  #   7. A help request naming a string that is not a canonical command MUST return failure with an Unknown fspec command message that names the stripped command (not the raw '<name> --help' string)
  #   8. A normal (non-help) command such as 'create-prefix' MUST be unaffected: help routing must not intercept it, and existing UnknownCommand/NotYetPorted/InvalidArgs behavior for non-help inputs stays byte-identical
  #
  # EXAMPLES:
  #   1. Calling the Fspec tool with command 'create-prefix --help' returns success and the output contains the create-prefix usage header and its positional arguments (prefix, description)
  #   2. Calling the Fspec tool with command 'create-prefix -h' returns the same create-prefix usage doc as the --help form
  #   3. Calling the Fspec tool with command 'help' and args command create-prefix returns the same create-prefix usage doc
  #   4. Calling the Fspec tool with command 'help' and no args returns general Fspec tool help explaining how to get per-command help
  #   5. Calling the Fspec tool with command 'nonexistent-xyz --help' returns failure with an Unknown fspec command message naming nonexistent-xyz
  #   6. Calling the Fspec tool with command 'board --help' (a real command with no CONFIG) returns success with a graceful no-detailed-help message rather than an error
  #   7. Calling the Fspec tool with command 'create-prefix' (no help flag) still dispatches the real create-prefix command unchanged, proving help routing does not intercept normal commands
  #
  # ========================================
  Background: User Story
    As a coding agent using the Fspec tool inside the codelet TUI
    I want to get command usage docs by calling the Fspec tool with a help request
    So that I can learn a command's arguments without leaving the TUI or guessing, exactly as the Fspec tool definition advertises

  Scenario: Embedded --help flag renders the per-command usage doc
    Given the native Rust fspec dispatcher with no JS chunk callback registered
    When I dispatch a command "create-prefix --help"
    Then the dispatch result is successful
    And the output contains the create-prefix usage header
    And the output lists the positional arguments prefix and description

  Scenario: Embedded -h flag renders the same usage doc as --help
    Given the native Rust fspec dispatcher with no JS chunk callback registered
    When I dispatch a command "create-prefix -h"
    Then the dispatch result is successful
    And the output is identical to the output of dispatching "create-prefix --help"

  Scenario: help command with an args command field renders the per-command usage doc
    Given the native Rust fspec dispatcher with no JS chunk callback registered
    When I dispatch a command "help" with args command "create-prefix"
    Then the dispatch result is successful
    And the output is identical to the output of dispatching "create-prefix --help"

  Scenario: help command with no args renders general Fspec tool help
    Given the native Rust fspec dispatcher with no JS chunk callback registered
    When I dispatch a command "help" with no args
    Then the dispatch result is successful
    And the output explains how to get per-command help

  Scenario: Help request for an unknown command fails naming the stripped command
    Given the native Rust fspec dispatcher with no JS chunk callback registered
    When I dispatch a command "nonexistent-xyz --help"
    Then the dispatch result is a failure
    And the error message contains "Unknown fspec command"
    And the error message names "nonexistent-xyz"

  Scenario: Help request for a real command without a CONFIG degrades gracefully
    Given the native Rust fspec dispatcher with no JS chunk callback registered
    When I dispatch a command "board --help"
    Then the dispatch result is successful
    And the output states no detailed help is available for board

  Scenario: Normal command dispatch is not intercepted by help routing
    Given the native Rust fspec dispatcher with no JS chunk callback registered
    When I dispatch a command "create-prefix" with valid create-prefix args
    Then help routing does not intercept the command
    And the command is dispatched through the normal ported or stub path
