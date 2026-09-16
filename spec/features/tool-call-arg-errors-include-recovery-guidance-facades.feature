@done
@tool-execution
@tool-integration
@rust
@tools
@tool-023
Feature: Tool-call arg errors include recovery guidance (facades)

  # Background: the LLM's most common Fspec tool mistake is sending `args`
  # (or the provider-specific equivalent) as a JSON object instead of a JSON
  # string — LLMs trained on OpenAI/Anthropic function-calling naturally
  # emit objects. Today that surfaces as an opaque serde error
  # ("Invalid arguments: invalid type: map, expected a string at line 1
  # column N") with no tool name and no corrected example. This feature
  # makes the provider facades detect that mistake (and the non-string
  # `project_root` mistake) BEFORE deserialization and emit a dedicated,
  # tool-named validation error with a corrected example.
  #
  # Companion feature (dispatcher layer):
  # spec/features/tool-call-arg-errors-include-recovery-guidance.feature
  #
  # Research: docs/research/TOOL-023-tool-call-arg-error-surface-research.md
  #
  # Architecture notes:
  # - The checks live in each `FspecToolFacade::map_params` implementation
  #   (Claude, OpenAI, Gemini, ZAI) in codelet-tools/src/facade/
  #   fspec_facade.rs, before `serde_json::from_value` runs, so the
  #   dedicated error replaces the opaque serde passthrough.
  # - Unknown command names are passed through unchanged — the did-you-mean
  #   suggestion is produced by the fspec-core dispatcher (companion
  #   feature), keeping the command vocabulary in one place.
  Scenario: Claude facade rejects args sent as a JSON object with a dedicated explanation
    Given I call the Claude Fspec facade with {"command":"board","args":{"status":"backlog"}}
    When the facade maps the provider parameters
    Then the facade returns a validation error
    And the error message names the tool "Fspec"
    And the error message says the "args" parameter must be a string containing JSON, not a JSON object
    And the error message shows a corrected example using args as a JSON string
    And the error message does NOT contain the substring "Invalid arguments"

  Scenario: Claude facade rejects non-string project_root with a type explanation
    Given I call the Claude Fspec facade with {"command":"board","args":"{}","project_root":42}
    When the facade maps the provider parameters
    Then the facade returns a validation error
    And the error message names the tool "Fspec"
    And the error message says the "project_root" parameter must be a string

  Scenario: Gemini facade args sent as an object gets the same dedicated explanation
    Given I call the Gemini Fspec facade with {"command":"board","args":{"status":"backlog"}}
    When the facade maps the provider parameters
    Then the facade returns a validation error
    And the error message names the tool "fspec_command"
    And the error message says the "args" parameter must be a string containing JSON, not a JSON object

  Scenario: ZAI facade arguments sent as an object gets the same dedicated explanation
    Given I call the ZAI Fspec facade with {"command":"board","arguments":{"status":"backlog"}}
    When the facade maps the provider parameters
    Then the facade returns a validation error
    And the error message names the tool "run_fspec"
    And the error message says the "arguments" parameter must be a string containing JSON, not a JSON object

  Scenario: Missing command field error includes a usage example
    Given I call the Gemini Fspec facade with {"args":"{}"}
    When the facade maps the provider parameters and finds command missing
    Then the facade returns a validation error
    And the error message names the tool "fspec_command"
    And the error message includes an example invocation with command and args
    And the error message mentions the "help" command for available commands

  Scenario: Facades pass unknown command names through unchanged
    Given I call the Claude Fspec facade with {"command":"list-workunit","args":"{}"}
    When the facade maps the provider parameters
    Then the facade returns success with command "list-workunit"
    And the facade does NOT add or strip any hint text itself
