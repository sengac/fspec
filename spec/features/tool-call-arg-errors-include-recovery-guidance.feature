@done
@tool-execution
@tool-integration
@rust
@tools
@tool-023
@TOOL-023
Feature: Tool-call arg errors include recovery guidance (dispatcher)

  # Background: when the LLM calls the Fspec tool with wrong or missing
  # arguments, the error text that lands in the tool result is the LLM's ONLY
  # recovery surface. Errors that name neither the tool nor how to call it
  # correctly force the LLM to re-consult the tool definition or guess. This
  # feature makes the Fspec dispatcher layer of arg-related tool-call
  # failures self-explanatory: name the command, say what went wrong, and
  # tell the LLM how to call the tool correctly (a close-name suggestion, a
  # help pointer, or a concrete corrected example).
  #
  # Companion features:
  # spec/features/tool-call-arg-errors-include-recovery-guidance-facades.feature
  # spec/features/tool-call-arg-errors-include-recovery-guidance-all-tools.feature
  #   (the UNIVERSAL layer: every other tool — direct rig tools and all
  #   provider facades — must likewise return self-explanatory arg errors)
  #
  # Research: docs/research/TOOL-023-tool-call-arg-error-surface-research.md
  #
  # Architecture notes:
  # - Fuzzy matching lives in codelet-fspec-core as
  #   `pub fn suggest_command(input: &str) -> Option<&'static str>` over the
  #   162 canonical command names + the Rust-only `foundation-status`
  #   extension (Levenshtein distance <= 3 on hyphen-stripped names; exact
  #   match wins; ties break by shorter name then lexicographic). Kept in
  #   fspec-core because the canonical list is owned there.
  # - Existing error substrings ("Unknown fspec command", "Invalid args for
  #   fspec command", "failed to parse args") are preserved as substrings so
  #   the ~162 per-command port tests keep passing; new guidance is APPENDED
  #   on its own line(s).
  Scenario: Unknown fspec command error suggests the closest canonical command
    Given I dispatch the Fspec command "list-workunit"
    When the command name is not canonical
    Then the dispatcher returns success=false
    And the error message contains the substring "Unknown fspec command: list-workunit"
    And the error message contains a line starting with "Did you mean: list-work-units"
    And the error message tells me to re-call with the suggested name
    And the error message points me to "<command> --help" for full argument reference

  Scenario: Unknown fspec command with no close match gets no suggestion line
    Given I dispatch the Fspec command "zzzqqqxx"
    When the command name is not canonical and closely matches no command
    Then the dispatcher returns success=false
    And the error message contains the substring "Unknown fspec command: zzzqqqxx"
    And the error message does NOT contain the substring "Did you mean"

  Scenario: Unknown fspec command suggests the Rust-only foundation-status extension
    Given I dispatch the Fspec command "foundations-status"
    When the command name is not canonical but closely matches foundation-status
    Then the error message contains a line starting with "Did you mean: foundation-status"

  Scenario: suggest_command returns the closest name for small typos
    Given I ask the fuzzy matcher for "list-work-unitt"
    Then it returns the suggestion "list-work-units"
    And I ask the fuzzy matcher for "show-work-unit"
    And it returns the suggestion "show-work-unit" (exact match on a real command)
    And I ask the fuzzy matcher for "zzzqqqxx"
    And it returns no suggestion

  Scenario: suggest_command ignores empty or oversized input
    Given I ask the fuzzy matcher for ""
    Then it returns no suggestion
    And I ask the fuzzy matcher for a 200-character gibberish string
    And it returns no suggestion for the gibberish input

  Scenario: Malformed inner args on a ported command include a usage hint
    Given I dispatch the ported command "list-work-units" with malformed inner args "{"
    When the dispatcher parses the inner args
    Then the dispatcher returns success=false
    And the error message contains the substring "Invalid args for fspec command list-work-units"
    And the error message contains the substring "failed to parse args"
    And the error message tells me args must be a JSON object string
    And the error message points me to "list-work-units --help" for the full argument reference

  Scenario: Missing required inner field on a ported command includes a usage hint
    Given I dispatch the ported command "show-work-unit" with inner args "{}"
    When the dispatcher parses the inner args and finds the required field missing
    Then the dispatcher returns success=false
    And the error message contains the substring "Invalid args for fspec command show-work-unit"
    And the error message names the missing argument "workUnitId"
    And the error message points me to "show-work-unit --help" for the full argument reference
