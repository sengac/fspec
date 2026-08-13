@done
@cli
@rust
@validation
@RPC-329
Feature: Gherkin raw parser-error text + top-of-file line number diverge from cucumber (validate et al.)
  """
  A shared formatter format_parse_error_cucumber(content, &ParseError) -> (usize line, String message) lives in rust/fspec-core/src/io/gherkin.rs. gherkin-0.16 ParseError exposes only Display ('Error at L:C: {expected:?}') with private fields, so the formatter re-derives the cucumber message from the source content. SCOPE: the no-Feature-keyword class only. When the content has no 'Feature:' line, emit 'Parser errors:' + one '(line:col): expected: #EOF, #Language, #TagLine, #FeatureLine, #Comment, #Empty, got <text>' entry per non-blank/non-comment/non-tag line (col = indent+1) and report line 0. validate.rs delegates its parse-error branch to this formatter; get_suggestion then fires Add-Feature-keyword. Files WITH a Feature keyword that fail later are out of scope and keep gherkin-0.16-derived text.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A shared cucumber-compatible parse-error formatter lives in io/gherkin.rs and is reused by every command that surfaces a gherkin parse error (validate, add-scenario, etc.) — one source of truth
  #   2. For a malformed file with NO Feature keyword, the parse error is reported on Line 0 (parity with TS location?.line || 0), NOT gherkin-0.16's Line 1
  #   3. For a file with NO Feature keyword, the cucumber-compatible message begins with 'Parser errors:' then emits one line '(<line>:<col>): expected: #EOF, #Language, #TagLine, #FeatureLine, #Comment, #Empty, got '<trimmed line text>'' per non-blank, non-comment, non-tag source line (col = leading whitespace + 1)
  #   4. Because the reformatted message now contains 'expected' and '#FeatureLine', get_suggestion fires and the validate output includes 'Suggestion: Add Feature keyword at the beginning of the file' (parity with TS)
  #
  # EXAMPLES:
  #   1. Validate a feature file 'Scenario: orphaned\n  Given x\n  Then y' -> Line 0, message 'Parser errors:\n(1:1): expected: ..., got 'Scenario: orphaned'\n(2:3): ... got 'Given x'\n(3:3): ... got 'Then y'', Suggestion: Add Feature keyword..., exit 1
  #   2. Validate a file 'When something\nGiven nothing' (no indentation) -> Line 0, two entries '(1:1): ... got 'When something'' and '(2:1): ... got 'Given nothing'', Suggestion: Add Feature keyword...
  #   3. Unit test: format_parse_error_cucumber maps a no-Feature-keyword content + gherkin ParseError into (0, cucumber-vocab message) byte-identical to the captured TS fixture
  #
  # ASSUMPTIONS:
  #   1. Malformed files that DO contain a Feature keyword but fail later remain out of scope: gherkin-0.16's private ParseError fields and divergent recovery algorithm prevent faithful reconstruction; their raw text stays gherkin-0.16-derived (documented carry-over)
  #
  # ========================================
  Background: User Story
    As a developer running fspec validate (and other gherkin-parse-error surfaces) on the Rust binary
    I want to see Gherkin parse errors for malformed feature files reported with the same line number and cucumber token vocabulary as the TypeScript @cucumber/gherkin implementation
    So that the Rust port is byte-compatible with the TS reference for the no-Feature-keyword malformed-file class

  Scenario: Validate a no-Feature-keyword file reports cucumber vocabulary on Line 0
    Given a feature file whose content is 'Scenario: orphaned' then '  Given x' then '  Then y' with no Feature keyword
    When I dispatch the validate command against that single file
    Then the rendered output marks the file invalid with a 'Line 0:' detail
    Then the rendered output contains 'Parser errors:'
    Then the rendered output contains "(1:1): expected: #EOF, #Language, #TagLine, #FeatureLine, #Comment, #Empty, got 'Scenario: orphaned'"
    Then the rendered output contains "(2:3): expected: #EOF, #Language, #TagLine, #FeatureLine, #Comment, #Empty, got 'Given x'"
    Then the rendered output contains the line 'Suggestion: Add Feature keyword at the beginning of the file'
    Then the dispatcher result reports an exit code of 1

  Scenario: Validate a no-Feature-keyword file with unindented step keywords
    Given a feature file whose content is 'When something' then 'Given nothing' with no leading whitespace and no Feature keyword
    When I dispatch the validate command against that single file
    Then the rendered output marks the file invalid with a 'Line 0:' detail
    Then the rendered output contains "(1:1): expected: #EOF, #Language, #TagLine, #FeatureLine, #Comment, #Empty, got 'When something'"
    Then the rendered output contains "(2:1): expected: #EOF, #Language, #TagLine, #FeatureLine, #Comment, #Empty, got 'Given nothing'"
    Then the rendered output contains the line 'Suggestion: Add Feature keyword at the beginning of the file'

  Scenario: A file with a Feature keyword that fails later stays out of scope
    Given a feature file that begins with a valid 'Feature:' line but then contains a malformed construct the parser rejects
    When I dispatch the validate command against that single file
    Then the rendered output marks the file invalid
    Then the rendered output does NOT contain 'Parser errors:'
    Then the dispatcher result reports an exit code of 1
