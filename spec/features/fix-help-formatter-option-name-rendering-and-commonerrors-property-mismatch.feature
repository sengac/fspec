@bug
@help-system
@cli
@high
@HELP-006
Feature: Fix help formatter option name rendering and commonErrors property mismatch

  """
  Fix requires two changes: (1) Update all 7 Event Storm help files to use 'flag' property instead of 'name' in options array, matching CommandOption interface. (2) Update commonErrors to use 'fix' property instead of 'solution'. Interface is in src/utils/help-formatter.ts. No formatter changes needed - help files are using wrong property names.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Option names in help output must show the actual flag name (e.g., '--timestamp <ms>'), not 'undefined'
  #   2. CommandHelpConfig commonErrors array should use 'fix' property consistently (not 'solution')
  #   3. Help formatter must parse option.name field correctly to extract flag names
  #
  # EXAMPLES:
  #   1. Running 'fspec add-domain-event --help' shows OPTIONS section with '--timestamp <ms>' and '--bounded-context <context>' (not 'undefined')
  #   2. All Event Storm help files use 'fix' property in commonErrors, not 'solution'
  #   3. Help formatter extracts flag name from option.name using regex or string parsing
  #
  # ========================================

  Background: User Story
    As a developer viewing help for Event Storm commands
    I want to see properly formatted option names in help output
    So that I can understand what flags are available without seeing 'undefined'

  Scenario: Display option flags correctly in help output
    Given Event Storm help files use 'flag' property in options array
    When I run "fspec add-domain-event --help"
    Then the OPTIONS section should display "--timestamp <ms>"
    And the OPTIONS section should display "--bounded-context <context>"
    And the OPTIONS section should NOT display "undefined"

  Scenario: Use 'fix' property consistently in commonErrors
    Given all Event Storm help files exist
    When I check the commonErrors array in each help file
    Then all commonErrors should use 'fix' property
    And no commonErrors should use 'solution' property

  Scenario: Verify all 7 Event Storm help files are fixed
    Given the Event Storm help files have been updated
    When I run help for each Event Storm command
    Then "fspec add-domain-event --help" should show proper option flags
    And "fspec add-command --help" should show proper option flags
    And "fspec add-policy --help" should show proper option flags
    And "fspec add-hotspot --help" should show proper option flags
    And "fspec show-event-storm --help" should show proper option flags
    And "fspec show-foundation-event-storm --help" should show proper option flags
    And "fspec generate-example-mapping-from-event-storm --help" should show proper option flags
