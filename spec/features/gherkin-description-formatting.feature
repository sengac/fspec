@formatting
@formatter
@done
@RPC-330
Feature: Gherkin Description Blank-Line Preservation in Formatter
  """
  Fix lives in rust/fspec-core/src/io/gherkin_format.rs: feature/scenario/Background/Rule descriptions must be re-extracted from RAW source (between the header line and the first child construct), mirroring extract_description_verbatim in show_acceptance_criteria.rs, because gherkin-0.16 parser.rs:381 `description = (description_line ** _)` consumes inter-paragraph blank lines. Doc-string bodies (format_docstring/dedent) must NOT be touched by this change.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A feature-level description with multiple paragraphs separated by a blank line must retain that blank line after formatting
  #   2. A scenario-level (and Background/Rule) description with multiple paragraphs separated by a blank line must retain that blank line after formatting
  #   3. A single-paragraph description (no internal blank lines) must format identically to before — no regression
  #   4. Runs of 2 or more consecutive blank lines between paragraphs are collapsed to at most 2 blank lines (existing consecutive-blank cap is preserved)
  #   5. Step doc strings and scenario/step media-type doc strings must NOT regress: blank-line preservation applies only to free-form descriptions, not doc-string bodies
  #   6. Formatting a description that already contains preserved blank lines is idempotent (format twice yields identical bytes)
  #
  # EXAMPLES:
  #   1. A feature with a two-paragraph description (paragraph A, blank line, paragraph B) under the Feature header is formatted and still shows both paragraphs separated by exactly one blank line
  #   2. A scenario whose description has two paragraphs separated by a blank line is formatted and the blank line between the two paragraphs is preserved
  #   3. A feature description that is a single paragraph (one line, no blanks) formats with no leading/internal blank line — byte-identical to today's output
  #   4. A feature description with 4 consecutive blank lines between paragraphs is formatted down to exactly 2 blank lines between them
  #   5. A scenario with a step doc string spanning multiple lines (including a blank line in the body) formats with the existing doc-string behavior unchanged — no spurious blanks added or removed by the description fix
  #   6. A feature with a two-paragraph description is formatted twice; the output of the second run equals the output of the first run (idempotent)
  #
  # ========================================
  Background: User Story
    As a developer using the fspec Rust formatter
    I want to have `fspec format` preserve blank lines between paragraphs inside feature and scenario descriptions
    So that multi-paragraph specification prose survives a format round-trip instead of being collapsed into a single block

  Scenario: Preserve blank line between two feature-level description paragraphs
    Given a feature file whose Feature header is followed by this description:
      """
      Feature: Multi paragraph

        First paragraph of the feature description.

        Second paragraph of the feature description.

        Scenario: A
          Given x
      """
    When the formatter formats the feature file
    Then the feature description retains exactly one blank line between the two paragraphs:
      """
      Feature: Multi paragraph
        First paragraph of the feature description.

        Second paragraph of the feature description.

        Scenario: A
          Given x
      """

  Scenario: Preserve blank line between two scenario-level description paragraphs
    Given a feature file with a scenario whose header is followed by this description:
      """
      Feature: Scenario desc

        Scenario: Has prose
          First paragraph of the scenario description.

          Second paragraph of the scenario description.

          Given x
      """
    When the formatter formats the feature file
    Then the scenario description retains exactly one blank line between the two paragraphs:
      """
      Feature: Scenario desc

        Scenario: Has prose
          First paragraph of the scenario description.

          Second paragraph of the scenario description.

          Given x
      """

  Scenario: Single-paragraph description is unchanged
    Given a feature file with a single-line feature description and no internal blank lines:
      """
      Feature: One line desc

        Only one paragraph here.

        Scenario: A
          Given x
      """
    When the formatter formats the feature file
    Then the description is emitted with no leading or internal blank line and is byte-identical to the input layout:
      """
      Feature: One line desc
        Only one paragraph here.

        Scenario: A
          Given x
      """

  Scenario: Collapse runs of more than two blank lines between paragraphs to two
    Given a feature file whose feature description separates "Paragraph one." and "Paragraph two." by four consecutive blank lines
    When the formatter formats the feature file
    Then the two paragraphs are separated by exactly two blank lines:
      """
      Feature: Excessive blanks
        Paragraph one.


        Paragraph two.

        Scenario: A
          Given x
      """

  Scenario: Step doc string with an internal blank line is not regressed
    Given a feature file with a step doc string whose body is "line one", a blank line, then "line three"
    When the formatter formats the feature file
    Then the step doc string body is emitted unchanged with no spurious blanks added or removed

  Scenario: Formatting a multi-paragraph description is idempotent
    Given a feature file with a two-paragraph feature description that has already been formatted once
    When the formatter formats the feature file a second time
    Then the output of the second run is byte-identical to the output of the first run
