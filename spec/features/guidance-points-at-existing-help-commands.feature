@BUG-582
@done
@cli
@system-reminder
@documentation
@BUG-538
Feature: Guidance points at help commands that exist
  """
  fspec help <group> (specs, work, discovery, metrics, setup, hooks) was a TypeScript CLI feature that the Rust CLI never ported; fspec help <command> and fspec <command> --help work. Three texts still pointed at the groups: the specifying reminder (fspec-core update_work_unit_status/reminders/specifying.rs), the TOML slash command written by init for Gemini CLI and Qwen Code (fspec-core init/templates.rs) and the bootstrap Complete Command Reference step (fspec-core bootstrap_doc.txt). Product owner 2026-11-06 (H-HELP-GROUPS a): the groups are not restored; the texts point at fspec bootstrap, fspec --help and fspec <command> --help. CLI-016 adds the general parse check for every named command. Tests: rust/fspec-core/tests/bug538_guidance_points_at_existing_help_commands.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. fspec help <group> (specs, work, discovery, metrics, setup, hooks) does not exist in the Rust CLI and is not restored (product owner 2026-11-06, H-HELP-GROUPS option a); guidance points at fspec bootstrap, fspec --help or fspec <command> --help instead
  #   2. The specifying reminder ends with 'For more: fspec bootstrap (Example Mapping section)', like the Event Storm reminder's 'For guidance: fspec bootstrap (Event Storm section)'
  #   3. The TOML slash command written by fspec init (Gemini CLI, Qwen Code) lists the same two commands as the markdown one: fspec --sync-version <version>, then fspec bootstrap
  #   4. The bootstrap 'Complete Command Reference' step describes its sections as the command reference grouped by area and points at fspec --help and fspec <command> --help, without listing fspec help <group> commands
  #   5. The general check that every named command parses with the real CLI belongs to CLI-016 (it supersedes the 'add a check' part of this card)
  #
  # EXAMPLES:
  #   1. An agent moves a work unit to specifying and the reminder ends with 'For more: fspec bootstrap (Example Mapping section)', not 'fspec help discovery'
  #   2. fspec init --agent gemini writes .gemini/commands/fspec.toml telling the agent to run fspec --sync-version <version> and fspec bootstrap, and it names no fspec help <group> command
  #   3. The fspec bootstrap output names none of fspec help specs, work, discovery, metrics, setup or hooks, and its Complete Command Reference points at fspec <command> --help
  #
  # ========================================
  Background: User Story
    As a AI agent following fspec guidance
    I want to be pointed at help commands that exist
    So that following a pointer to more help never ends in an unknown command error

  Scenario: The specifying reminder points at the bootstrap Example Mapping section
    Given a work unit in backlog
    When the agent moves it to specifying
    Then the reminder ends with "For more: fspec bootstrap (Example Mapping section)"
    And the reminder does not contain "fspec help discovery"

  Scenario: The TOML slash command lists fspec --sync-version and fspec bootstrap
    Given an empty project
    When fspec init installs the Gemini CLI agent
    Then .gemini/commands/fspec.toml tells the agent to run fspec --sync-version with the fspec version and then fspec bootstrap
    And it names no fspec help group command

  Scenario: The bootstrap command reference points at per-command help
    Given any project
    When the agent runs fspec bootstrap
    Then the Complete Command Reference step points at fspec --help and fspec <command> --help
    And the output names none of fspec help specs, work, discovery, metrics, setup or hooks
