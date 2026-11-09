@done
@cli
@validation
@gherkin
@BUG-479
Feature: Feature files match the formatter
  """
  fspec audit's format area (fspec-core commands/audit.rs) parses every feature file, re-serialises it with format_feature (fspec-core io/gherkin_format.rs) and reports the file when the bytes differ. Files failed for two reasons: the generators (create-feature, generate-scenarios) wrote a layout the formatter rewrites, and three formatter defects (a '#' line inside a description taken for a comment and moved, comments after the last scenario dropped, 'Background: ' written with a trailing space) made some files impossible to satisfy without losing content. The generators now write the canonical layout, the formatter keeps content, and the committed files that failed were corrected by hand (fspec format is not used on the repository, BUG-152). Tests: rust/fspec-core/tests/bug479_feature_files_match_the_formatter.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. fspec audit reports a feature file as needing formatting when the formatter (fspec-core io/gherkin_format.rs) would change its bytes. Canonical layout: one tag per line; no blank line between the Feature line and its description; one blank line before each Background, Scenario and Rule; a comment block sits directly above the construct it precedes; the file ends with exactly one newline
  #   2. Why the files failed (found 2026-11-06): create-feature and generate-scenarios write a blank line after the Feature line, generate-scenarios a blank line between the Example Mapping comment block and Background, and create-feature all tags on one line; hand edits left trailing blank lines, a missing final newline, deeper-indented description lines and one-line tags. Three formatter defects made some files impossible to satisfy without losing content
  #   3. The formatter never changes content: a description or docstring line starting with # is text, not a comment, and stays where it is; comment lines after the last scenario stay at the end of the file; a Background without a name is written 'Background:' with no trailing space
  #   4. create-feature and generate-scenarios write feature files that already pass the formatting check
  #   5. The committed feature files that failed are corrected to the canonical layout by hand (never with fspec format, BUG-152) without losing content; feature files of cards still in progress (untracked) are left to those cards
  #
  # EXAMPLES:
  #   1. client-mcp-stale-approval-replies.feature has a description line '#[path] fixtures.' inside its docstring; formatting it keeps the line inside the docstring, so check passes on the file as written
  #   2. delete-scenario-rust-port.feature ends with a '# BUG-444 superseded ...' comment block after its last scenario; formatting it keeps the comment block, so check passes
  #   3. rlcd-blocklist-security.feature has 'Background:' with no name; the formatter keeps 'Background:' without a trailing space
  #   4. fspec create-feature 'Sample Thing' and fspec generate-scenarios for a work unit with a user story, rules and examples each write a feature file that fspec audit reports as formatted
  #   5. After the fix, fspec audit lists no committed feature file under 'needs formatting'
  #
  # ========================================
  Background: User Story
    As a maintainer validating a card
    I want to have fspec audit's formatting pass on feature files that fspec wrote or a card corrected
    So that a failing check always points at a real problem in my card, not at old files or generator output

  Scenario: A description line starting with a hash stays in the description
    Given a feature whose description docstring contains the line "#[path] fixtures."
    When fspec audit runs and the file is formatted in a scratch project
    Then the audit reports the file as formatted
    And formatting leaves the file unchanged

  Scenario: Comments after the last scenario are kept
    Given a feature that ends with a comment block after its last scenario
    When fspec audit runs and the file is formatted in a scratch project
    Then the audit reports the file as formatted
    And formatting leaves the file unchanged

  Scenario: A Background without a name has no trailing space
    Given a feature with a line "  Background:" and no Background name
    When fspec audit runs and the file is formatted in a scratch project
    Then the audit reports the file as formatted
    And formatting leaves the file unchanged

  Scenario: create-feature writes a formatted feature file
    Given an empty project
    When fspec create-feature "Sample Thing" runs
    Then fspec audit reports spec/features/sample-thing.feature as formatted

  Scenario: generate-scenarios writes a formatted feature file
    Given a work unit with a user story, rules, examples and an answered question
    When fspec generate-scenarios writes its feature file
    Then fspec audit reports that feature file as formatted

  Scenario: Every committed feature file is formatted
    Given the repository's committed feature files
    When fspec audit runs on them
    Then none of them is reported as needing formatting
