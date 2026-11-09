@done
@cli
@help-system
@validation
@BUG-566
Feature: Help and feature steps carry no TypeScript-era leftovers
  """
  The help configs were captured from the TypeScript CLI, whose formatter read keys the configs did not set: common errors declared 'solution' (printed 'Fix: undefined'), patterns declared without a description or as {title, commands} (printed 'undefined' lines). Feature steps still named commands that were renamed or never ported (fspec generate-tags). Fixes: CommonPattern.description is Option (no line when absent); each fix is the TS original's solution text from git history (103a73072^); {title, commands} patterns print the title and the commands; help fixtures follow their configs (CLI-016). The checks are exact, in rust/fspec/src/help_and_step_leftovers_tests.rs (a bin unit test, it needs the binary-private Cli and reuses CLI-016's mention extraction): no help config text field is empty or 'undefined', no --help prints 'undefined', and every 'fspec <command>' mention in the steps and doc strings of non-@deprecated scenarios names a registered command.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. No text field of any CommandHelpConfig is empty or the literal 'undefined', and no command's --help prints the word 'undefined' (exact check, a bin unit test next to CLI-016's contract)
  #   2. A common error's fix is the TypeScript original's 'solution' text (the TS formatter read 'fix', so it printed 'Fix: undefined'); recovered from git history (103a73072^:src/commands/*-help.ts), and CLI-016's contract checks every fspec mention in it
  #   3. A common pattern's description is optional: patterns the TS configs declared with only {pattern, example} have none and print no description line; patterns declared as {title, commands} print the title as the pattern and the commands, one per line, as the example
  #   4. Help fixtures follow their configs (CLI-016); scenarios that asserted 'Fix: undefined' in help now assert the real fix text
  #   5. In every scenario and background not tagged @deprecated (on the scenario or the feature), each 'fspec <command>' mention in a step or its doc string names a registered command; mentions are extracted with CLI-016's exact rules
  #   6. Renamed commands are fixed in the steps (generate-tags -> generate-tags-md, generate-foundation -> generate-foundation-md, show-dependencies -> dependencies); scenarios describing commands fspec never had in Rust are tagged @deprecated; live scenarios that run a removed command on purpose name it without the 'fspec ' prefix
  #
  # EXAMPLES:
  #   1. fspec add-attachment --help prints 'Fix: Create the work unit first with ...' instead of 'Fix: undefined'
  #   2. a help pattern without a description prints no 'undefined' line (the search-scenarios help this example named was removed by CLI-019)
  #   3. fspec update-step --help prints each common pattern's name and example with no 'undefined' description line
  #   4. generate-tags-md.feature's step 'When I run `fspec generate-tags`' fails the check naming the file, the line and 'generate-tags'; after the fix it reads 'fspec generate-tags-md'
  #   5. work-unit-estimation-and-metrics.feature's 'Record 45k tokens consumed' (fspec record-tokens, never ported) is tagged @deprecated and skipped by the check
  #
  # QUESTIONS (ANSWERED):
  #   Q: scenarios describing commands fspec never had in Rust (record-tokens, audit-scenarios, validate-event-storm, suggest-tags-from-events, sync-tags-with-event-storm, migrate-to-jsonl, build-tool, the TS help interceptor's unknown-command fallback) were tagged @deprecated (conservative, reversible) rather than deleted. Should they be deleted instead?
  #   A: Not decided by the product owner; the helper took the conservative, reversible option (2026-11-07): tag them @deprecated so the feature keeps documenting the old behaviour and the check skips them. Deleting them is left to the product owner
  #
  # ASSUMPTIONS:
  #   1. Not decided by the product owner; the helper took the conservative, reversible option (2026-11-07): tag them @deprecated so the feature keeps documenting the old behaviour and the check skips them. Deleting them is left to the product owner
  #
  # ========================================
  Background: User Story
    As a AI agent or developer reading fspec help and feature files
    I want to see only real fixes, patterns and command names, never TypeScript-era leftovers
    So that I can follow the help and the scenarios without running commands that do not exist

  Scenario: No help config field holds the literal 'undefined' or is empty
    Given every command's CommandHelpConfig and the help every command prints
    When each text field and each printed help is checked
    Then no field is empty or the literal 'undefined' and no help prints 'undefined'

  Scenario: An 'undefined' common-error fix is reported naming the command and the field
    Given a help config for "demo" whose first common error has the fix "undefined"
    When its fields are checked
    Then the problems name "commonErrors[0].fix is the literal 'undefined'" and "whenToUse is empty"

  Scenario: Feature steps name only registered fspec commands
    Given every feature file under spec/features
    When each fspec mention in the steps of its non-deprecated scenarios is checked against the registry
    Then every mention names a registered command

  Scenario: A step naming a renamed command fails while a deprecated scenario is skipped
    Given a feature whose scenario step runs the renamed command generate-tags and whose @deprecated scenario runs the never-ported command record-tokens
    When the feature's steps are checked
    Then one problem names line 4 and the unknown command 'generate-tags'
