@VAL-006
@cli
@validation
@tag-management
Feature: validate-tags default output shows only failures, with opt-in --verbose and --summary flags
  """
  Architecture notes:
  - validateTagsCommand Commander action signature changes to accept an options
  object { verbose?: boolean; summary?: boolean } (plus the existing file arg).
  - registerValidateTagsCommand adds `.option('--verbose', ...)` and
  `.option('--summary', ...)`.
  - Output flags do NOT affect the exported `validateTags(options)` return value;
  only the CLI printing branch changes. Programmatic callers see identical results.
  - BREAKING CHANGE (tiny scope): any script that greps validate-tags output for
  "✓ All tags in " must now pass --verbose. This is explicitly documented in the
  help text and release notes. Rationale: the default flooded chat_history when
  AI agents invoked this tool, causing PromptCancelled cascades. Quiet-by-default
  is the right default for any CLI meant to be consumed by AI agents.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Default behavior: print ONLY ✗ violation blocks plus the final summary counts. No per-file ✓ lines by default.
  #   2. --verbose restores the old behavior: print one ✓ line per passing file plus every ✗ violation block plus summary.
  #   3. --summary suppresses ALL per-file output (both ✓ and ✗) and prints only the final summary counts.
  #   4. Exit code unchanged: 0 when all valid, 1 when any invalid, 2 on unexpected error — regardless of output flags.
  #   5. When --summary and --verbose are combined, --summary wins (quietest flag dominates).
  #
  # EXAMPLES:
  #   1. No flags, all-valid tree: no per-file output, just "✓ N files passed"
  #   2. No flags, some failures: only ✗ blocks for failing files + summary counts
  #   3. --verbose, all-valid tree: one ✓ line per file + summary
  #   4. --summary, some failures: exactly two count lines
  #   5. No flags, single valid file: no output at all (exit 0)
  #   6. --summary --verbose combined: summary wins
  #
  # ========================================
  Background: User Story
    As a AI agent using fspec via tool-calling
    I want to run validate-tags and get back only what I need (failures by default, optional full verbosity with --verbose, just counts with --summary)
    So that the tool result stays small (~0-5 lines on a clean tree) instead of flooding chat_history with ~3000 ✓ lines, preventing PromptCancelled cascades on the next LLM turn

  Scenario: Default behavior on an all-valid tree prints no per-file ✓ lines
    Given a project with 3 feature files that all have valid registered tags
    When I run `fspec validate-tags` with no flags
    Then the command should exit with code 0
    And the output should NOT contain any line starting with "✓ All tags in "
    And the output should contain exactly one line: "✓ 3 files passed"

  Scenario: Default behavior with some failures prints only violation blocks plus summary
    Given a project with 5 feature files
    And 3 of those files have valid registered tags
    And 2 of those files contain an unregistered tag "@nonexistent"
    When I run `fspec validate-tags` with no flags
    Then the command should exit with code 1
    And the output should NOT contain any line starting with "✓ All tags in "
    And the output should contain exactly 2 "✗ <file> has tag violations:" blocks
    And the output should contain the line "✓ 3 files passed"
    And the output should contain the line "✗ 2 files have tag violations"

  Scenario: --verbose on an all-valid tree restores the old one-line-per-file output
    Given a project with 3 feature files that all have valid registered tags
    When I run `fspec validate-tags --verbose`
    Then the command should exit with code 0
    And the output should contain exactly 3 lines starting with "✓ All tags in "
    And the output should contain the line "✓ 3 files passed"

  Scenario: --summary prints only the two summary count lines when some files fail
    Given a project with 5 feature files
    And 3 of those files have valid registered tags
    And 2 of those files contain an unregistered tag "@nonexistent"
    When I run `fspec validate-tags --summary`
    Then the command should exit with code 1
    And the output should NOT contain any line starting with "✓ All tags in "
    And the output should NOT contain any line starting with "✗ " followed by a file path
    And the output should contain the line "✓ 3 files passed"
    And the output should contain the line "✗ 2 files have tag violations"

  Scenario: Default behavior on a single valid file produces no output
    Given a single feature file "spec/features/foo.feature" with valid registered tags
    When I run `fspec validate-tags spec/features/foo.feature`
    Then the command should exit with code 0
    And the output should be empty

  Scenario: --summary combined with --verbose behaves identically to --summary alone
    Given a project with 2 feature files
    And 1 of those files has valid registered tags
    And 1 of those files contains an unregistered tag "@nonexistent"
    When I run `fspec validate-tags --summary --verbose`
    Then the command should exit with code 1
    And the output should NOT contain any line starting with "✓ All tags in "
    And the output should NOT contain any "✗ <file> has tag violations:" block
    And the output should contain exactly these two lines:
      | ✓ 1 files passed              |
      | ✗ 1 files have tag violations |
