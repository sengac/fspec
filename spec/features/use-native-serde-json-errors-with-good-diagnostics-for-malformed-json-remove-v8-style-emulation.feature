@done
@infrastructure
@utils
@RPC-334
Feature: Use native serde_json errors with good diagnostics for malformed JSON (remove V8-style emulation)
  """
  Vendored crate created: codelet/fspec-json-error (codelet-fspec-json-error). Trimmed from format_serde_error 0.3.0 (MIT): serde_json-only, no yaml/toml/colored, no global atomics. FIXED upstream issue #20 (panic on small context_characters — usize underflow from double whitespace subtraction) + removed buggy get_default_contextualize. 6 unit tests + 1 doctest pass; clippy clean under workspace deny-lints. Added to workspace members + workspace.dependencies. Reviewed all upstream GitHub issues: only #20 relevant (fixed); #19 (1.0 prep) addressed by dropping global state; #21/#23/#24/#25 are yaml/toml/ron — N/A since trimmed.
  ParseJson Display reshaped so the multi-line caret snippet reads well: 'Failed to parse <file>: the file may be corrupted or contain invalid JSON.' on the first line, followed by the SerdeError snippet on subsequent lines. Shared helper io::json_error::parse_json_diagnostic(file_label, input, &err) -> FspecCoreError::ParseJson builds the snippet into `reason`; io::json_error::parse_json_reason(input, &err) -> String returns just the snippet for the InvalidArgs/String-wrapping sites (Groups 1,3,4).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When a canonical fspec state file fails to parse, the error names the file AND includes a caret-pointed snippet of the offending line(s) rendered by codelet-fspec-json-error
  #   2. The shared funnel read_or_init_json and the ensure_* read paths route parse errors through one shared helper (io::json_error), so every ensure_*-based command is upgraded at once (Group 0)
  #   3. The 6 commands that prepended 'Unexpected token in JSON:' (auto_advance, record_iteration, workflow_automation, query_work_units, query_metrics, export_work_units) no longer emit that fabricated prefix; they keep their command-specific outer wrapper but the inner body is the serde snippet (Group 1)
  #   4. ParseJson Display stays recognizable: it contains 'Failed to parse <file>' and the corruption-guidance sentence, with the caret snippet following on subsequent lines
  #   5. Lenient/silent parse sites (Group 5: list-hooks, list-schedules, read_work_units_or_empty, coverage readers, validate-tags/hooks, search-*, create-* lookups) remain unchanged — they keep returning empty/None on malformed input
  #   6. When the serde error carries no location, the formatter falls back to the bare serde message (no caret, no panic)
  #
  # EXAMPLES:
  #   1. A work-units.json with an unquoted key `status:` at line 4 column 37 -> error contains 'Failed to parse work-units.json', the corruption sentence, the source line, and a caret above 'key must be a string at line 4 column 37'
  #   2. Running auto-advance against a corrupt work-units.json -> message keeps the 'Failed to auto-advance:' outer wrapper and the serde snippet, and does NOT contain 'Unexpected token in JSON:'
  #   3. A single-line input `{ bad` -> snippet shows ` 1 | { bad` with a caret under column 3 and 'key must be a string at line 1 column 3'
  #   4. list-hooks (Group 5) against a malformed fspec-hooks.json -> still returns an empty hook list, no parse error surfaced
  #
  # ========================================
  Background: User Story
    As a fspec user or agent whose state file got corrupted
    I want to see a caret-pointed diagnostic that names the file and points at the exact malformed token
    So that I can locate and fix the corruption quickly instead of guessing from a bare serde message

  @rust
  @error-handling
  Scenario: Shared funnel surfaces a caret-pointed diagnostic naming the file
    Given a work-units.json whose line 4 contains an unquoted key "status:" at column 37
    When the file is read through the shared read_or_init_json funnel
    Then the error message contains "Failed to parse work-units.json"
    And the error message contains "the file may be corrupted or contain invalid JSON."
    And the error message contains the offending source line
    And the error message contains a caret line with "key must be a string at line 4 column 37"

  @rust
  @error-handling
  @regression
  Scenario: auto-advance keeps its outer wrapper but drops the fabricated V8 prefix
    Given a corrupt work-units.json
    When I run the auto-advance command against it
    Then the error message contains "Failed to auto-advance:"
    And the error message contains the serde caret snippet
    And the error message does not contain "Unexpected token in JSON:"

  @rust
  @error-handling
  Scenario: Single-line input places the caret under the exact error column
    Given a single-line JSON input "{ bad"
    When the input is rendered by the shared diagnostic helper
    Then the snippet contains " 1 | { bad"
    And the snippet contains a caret under column 3
    And the snippet contains "key must be a string at line 1 column 3"

  @rust
  @resilience
  Scenario: Lenient parse sites stay silent on malformed input
    Given a malformed fspec-hooks.json
    When I run the list-hooks command against it
    Then an empty hook list is returned
    And no parse error is surfaced

  @rust
  @edge-case
  Scenario: A serde error with no location falls back to the bare message
    Given a serde_json error that carries no line or column
    When the input is rendered by the shared diagnostic helper
    Then the rendered output is the bare serde message
    And no caret line is produced
