@BUG-582
@done
@cli
@coverage-tracking
@validation
@BUG-563
Feature: Rule blocks are rejected
  """
  A line starting with 'Rule:' makes Gherkin open a Rule block, and every scenario after it becomes the Rule's child. The coverage commands only tracked top-level scenarios (generate-coverage --feature-name included Rule children, the all-features run did not), so websearch-tool-pause-integration's scenarios silently had no coverage. Decision (coordinator, 2026-11-07, the conservative option): fspec does not support Rule blocks; business rules live on the work unit. One check in fspec-core io/gherkin.rs (rule_block_errors) names each Rule line; validate (and check) report it, and generate-coverage, link-coverage, fspec audit and show-coverage refuse the feature with the same message. Tests: rust/fspec-core/tests/bug563_rule_blocks_are_rejected.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Decision (coordinator, 2026-11-07; conservative option, recorded here): fspec does not support Gherkin Rule blocks. Business rules live on the work unit (fspec add-rule), not in feature files
  #   2. fspec validate (and so fspec audit) reports every Rule block as an error naming its line and the Rule line text, and exits 1
  #   3. generate-coverage (all features and --feature-name), link-coverage, fspec audit and show-coverage refuse a feature that contains a Rule block with the same message instead of silently skipping its scenarios
  #   4. create-feature, generate-scenarios and add-scenario never write a Rule block
  #
  # EXAMPLES:
  #   1. A feature whose description has a line 'Rule: pause implies a visible browser' followed by three scenarios makes fspec validate report 'line 9: Rule blocks are not supported' and exit 1
  #   2. fspec audit on that feature reports the Rule block message instead of no problems for 0 scenarios
  #   3. fspec generate-coverage --feature-name on that feature fails with the Rule block message and writes no coverage file
  #
  # ========================================
  Background: User Story
    As a maintainer tracking scenario coverage
    I want to be told when a feature file contains a Gherkin Rule block
    So that no scenario silently drops out of generate-coverage, link-coverage and fspec audit

  Scenario: validate reports a Rule block naming its line
    Given a feature whose line 5 is "  Rule: pause implies a visible browser" followed by scenarios
    When fspec validate runs on it
    Then it reports "line 5: Rule blocks are not supported" with the Rule line text
    And the feature is invalid

  Scenario: generate-coverage refuses a feature with a Rule block
    Given a feature with a Rule block and no coverage file
    When fspec generate-coverage runs for all features and for that feature by name
    Then both runs fail naming the Rule block line
    And no coverage file is written

  Scenario: link-coverage, fspec audit and show-coverage reject a feature with a Rule block
    Given a feature with a Rule block and a coverage file
    When fspec link-coverage, fspec audit and fspec show-coverage run on it
    Then each fails naming the Rule block line
