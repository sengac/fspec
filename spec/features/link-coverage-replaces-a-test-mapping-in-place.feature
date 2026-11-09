@done
@BUG-502
@cli
@coverage-tracking
@rust
Feature: link-coverage replaces a test mapping in place
  """
  link-coverage only ever added a test mapping, so relinking a drifted range
  left the stale mapping in place (and BUG-452 validated the union of the old
  and new ranges), while unlink-coverage --test-file also removed the
  mapping's implementation mappings. COV-068 therefore edited
  testMappings[].lines in the .feature.coverage JSON by hand.
  Now link-coverage --replace (with --test-file and --test-lines) replaces
  every mapping of that test file in the scenario with one mapping holding the
  new lines, keeping their implementation mappings; the step check reads only
  the new lines. Without --replace link-coverage still adds a mapping (steps
  may be spread over several ranges). An implementation-only link of an
  implementation file already on the test mapping replaces its lines in place.
  unlink-coverage --test-file --impl-file removes that implementation mapping
  from whichever mapping of the test file carries it, instead of failing when
  the first mapping does not.
  Code: rust/fspec-core/src/commands/link_coverage.rs (+ mapping.rs),
  rust/fspec-core/src/commands/unlink_coverage.rs (+ mapping.rs),
  rust/fspec/src/link_coverage.rs and cli_registration (the --replace flag).
  Tests: rust/fspec-core/tests/bug502_link_coverage_replaces_a_test_mapping.rs.
  """

  Background: User Story
    As a maintainer fixing coverage mappings
    I want to replace a scenario's test mapping lines with fspec link-coverage while keeping its implementation mappings
    So that stale ranges can be fixed without editing .feature.coverage JSON by hand

  Scenario: Replacing a test mapping keeps its implementation mappings
    Given scenario "Log in" maps test lines 1-3 carrying an implementation mapping
    When link-coverage is run with --replace and test lines 4-6
    Then the scenario has one test mapping for lines 4-6
    And that mapping still carries the implementation mapping

  Scenario: A replacement range that lacks the steps is refused
    Given scenario "Log in" maps test lines 4-6 holding all its @step comments
    When link-coverage is run with --replace and test lines 1-2
    Then link-coverage fails naming the missing steps
    And the mapping for lines 4-6 is unchanged

  Scenario: Replacing a test file the scenario does not map is refused
    Given scenario "Log in" has no test mapping
    When link-coverage is run with --replace and test lines 4-6
    Then link-coverage fails saying there is no mapping of that test file to replace

  Scenario: Linking without replace still adds a second range
    Given scenario "Log in" maps test lines 4-4 holding its Given step
    When link-coverage is run without --replace for test lines 5-6
    Then the scenario has two test mappings for that test file

  Scenario: Relinking an implementation file replaces its lines
    Given scenario "Log in" maps test lines 4-6 carrying an implementation mapping of lines 5 and 6
    When the same implementation file is linked with lines 10-11
    Then the test mapping carries that implementation file once with lines 10 and 11

  Scenario: Unlinking an implementation mapping from the second mapping of a test file
    Given scenario "Log in" has two mappings of one test file and only the second carries an implementation mapping
    When unlink-coverage is run with that test file and implementation file
    Then the implementation mapping is removed
    And both test mappings remain
