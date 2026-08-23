@done
@formatting
@parser
@formatter
@BUG-158
Feature: Formatter duplicates scenarios when a scenario has no steps (prose-only or unrecognized lowercase step keywords)

  """
  The AST-based Gherkin formatter (rust/fspec-core/src/io/gherkin_format.rs) must emit every scenario exactly once when a scenario has no steps. The Rust gherkin-0.16.0 parser only recognizes capitalized step keywords (Given/When/Then/And/But/*), so a scenario whose step lines use lowercase keywords (given/when/then) — or a genuinely prose-only scenario — has zero parsed steps and its prose is stored in Scenario.description. format_scenario must bound the verbatim description extraction by the next sibling construct (next scenario or rule at the same nesting level) when the scenario has neither steps nor Examples, mirroring the BUG-157 Background fix; an unbounded extraction swallows the trailing sibling scenarios into the description and re-emits them nested under the scenario, duplicating every scenario block. Formatting must be idempotent for such files. The TypeScript @cucumber/gherkin reference has the same case-sensitive keyword behavior, so this is a formatter-side fix, not a parser-side fix.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When a Scenario has no steps and no Examples, the formatter must bound the verbatim description extraction by the next sibling construct (the next Scenario or Rule at the same nesting level), mirroring the BUG-157 Background fix
  #   2. A scenario's description is the prose between the Scenario header and its first step (or first Examples table); it never includes subsequent sibling scenarios or rules
  #   3. Every scenario in the parsed AST is emitted exactly once at its original indentation level; formatting is idempotent for files containing step-less scenarios
  #
  # EXAMPLES:
  #   1. A file whose first scenario uses lowercase step keywords (given/when/then) — which the Rust gherkin parser stores as description — formats to exactly the original scenario count
  #   2. Formatting a step-less-scenario file twice produces byte-identical output (idempotent)
  #   3. A 2-scenario file where the first scenario is prose-only (no steps) formats to exactly 2 Scenario lines, each at 2-space top-level indentation
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to format feature files whose scenarios have no recognized steps
    So that my feature files are formatted correctly without scenario duplication

  Scenario: Formatting a file with a step-less scenario does not duplicate scenarios
    Given a feature file where the first scenario is prose-only (no steps) followed by a second scenario with steps
    When the formatter formats the feature file
    Then the formatted output contains exactly two Scenario lines
    And every scenario is indented at the top level (two spaces)

  Scenario: Formatting a step-less-scenario file is idempotent
    Given a feature file where the first scenario is prose-only (no steps) followed by a second scenario
    When the formatter formats the file twice
    Then the output of the second run is byte-identical to the output of the first run

  Scenario: A scenario with lowercase step keywords is formatted without duplication
    Given a feature file whose scenarios use lowercase step keywords (given, when, then)
    When the formatter formats the feature file
    Then the formatted output contains exactly the original number of Scenario lines
    And the lowercase step lines are preserved verbatim under their scenario

  Scenario: A Rule containing a step-less scenario is formatted without duplication
    Given a feature file with a Rule containing a prose-only scenario followed by a second scenario
    When the formatter formats the feature file
    Then both scenarios are emitted exactly once at the Rule nesting level
