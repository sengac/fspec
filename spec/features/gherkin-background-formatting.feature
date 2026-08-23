@done
@formatter
@parser
@formatting
@BUG-157
Feature: Gherkin Background formatting
  """
  The AST-based Gherkin formatter (rust/fspec-core/src/io/gherkin_format.rs) must emit every scenario exactly once when a feature file contains a Background section. The Rust gherkin-0.16.0 parser stores Background prose (the text between the Background header and its first step) in Background.description, so the formatter must NOT re-extract the Background description verbatim from the raw source without bounding the extraction by the Background's own content — an unbounded verbatim extraction swallows the trailing top-level scenarios and re-emits them nested under the Background, duplicating every scenario block. Formatting must be idempotent for Background-containing files.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A Background section's description is the prose between the Background header and the Background's first step (or the end of the Background block when it has no steps); it never includes subsequent top-level scenarios
  #   2. Every scenario in the parsed AST is emitted exactly once at its original top-level (2-space) indentation
  #   3. Formatting is idempotent: formatting an already-formatted Background-containing file produces byte-identical output
  #
  # EXAMPLES:
  #   1. A 2-scenario file with a `Background: User Story` prose block formats to exactly 2 `Scenario:` lines
  #   2. Re-running format on the formatted output is byte-identical
  #   3. A Background with actual steps (Given/When/Then) formats with its steps and does not duplicate scenarios
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to format feature files that contain a Background section
    So that my feature files are formatted correctly without scenario duplication

  Scenario: Formatting a Background-containing file does not duplicate scenarios
    Given a feature file with a Background section followed by two scenarios
    When the formatter formats the feature file
    Then the formatted output contains exactly two Scenario lines
    And every scenario is indented at the top level (two spaces)

  Scenario: Formatting a Background-containing file is idempotent
    Given a feature file with a Background section and two scenarios
    When the formatter formats the file twice
    Then the output of the second run is byte-identical to the output of the first run

  Scenario: A Background with steps is formatted with its steps
    Given a feature file with a Background section containing a Given step and one scenario
    When the formatter formats the feature file
    Then the Background step is emitted under the Background
    And the scenario is emitted exactly once at the top level

  Scenario: A Rule with a Background is formatted without duplication
    Given a feature file with a Rule containing a Background and one scenario
    When the formatter formats the feature file
    Then the scenario is emitted exactly once
    And the Background step is emitted under the Rule's Background
