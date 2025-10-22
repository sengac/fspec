@done
@validator
@validation
@BUG-029
Feature: Fix failing tests after phase tag removal

  """
  This bug fix updates test files after removing phase tags and usage metadata from the tag system. Test features must now have component tags and feature-group tags to pass validation. Test expectations checking for Usage column in TAGS.md must be removed.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All test features must have both component and feature-group tags to pass validation
  #   2. Test expectations for TAGS.md must not check for Usage column (removed)
  #   3. Remove or update scenarios in feature files that test phase tag validation or usage metadata
  #
  # EXAMPLES:
  #   1. feature-level-tag-management.test.ts expects result.success=true but gets false because validation fails
  #   2. generate-tags-md.test.ts and tags-md.test.ts check for Usage column in output but it no longer exists
  #   3. get-scenarios.test.ts expects 1 scenario but gets 3 because test features now have more tags after adding required component/feature-group
  #   4. retag.test.ts expects '@critical' in output but feature now has @cli @validation instead of @critical
  #
  # ========================================

  Background: User Story
    As a developer maintaining test suite
    I want to fix failing tests after removing phase tags and usage metadata
    So that all 1360 tests pass and the codebase is clean

  Scenario: Fix feature-level-tag-management tests with missing required tags
    Given test features in feature-level-tag-management.test.ts are missing component or feature-group tags
    When tests run and call addTagToFeature or removeTagFromFeature
    Then validation fails because required tags are missing
    When I add @cli and @validation tags to all test features
    Then all feature-level-tag-management tests pass

  Scenario: Remove Usage column expectations from TAGS.md generator tests
    Given generate-tags-md.test.ts and tags-md.test.ts check for Usage column in output
    And the Usage column was removed from tags-md generator
    When tests run
    Then assertions fail because Usage column no longer exists in output
    When I remove Usage column assertions from both test files
    Then both test files pass

  Scenario: Fix tag count expectations after adding required tags
    Given get-scenarios.test.ts expects specific scenario counts
    And test features now have additional component and feature-group tags
    When test runs with updated tag filter logic
    Then more scenarios match because features have more tags
    When I update expected counts to match new tag totals
    Then get-scenarios test passes

  Scenario: Fix retag test expectations after replacing phase tags
    Given retag.test.ts expects @critical tag in test features
    And test features now use @cli @validation instead of @critical
    When test runs and checks for @critical
    Then assertion fails because @critical no longer exists
    When I update test to use @cli @validation tags
    Then retag test passes