@BUG-582
@done
@REFAC-034
@codelet
@refactor
@code-quality
Feature: The GenerateCompaction feature is split into small files
  """
  spec/features/generate-compaction-tool.feature (300 lines) keeps every scenario except the three
  wiring scenarios, which move unchanged, with their coverage, to
  spec/features/generate-compaction-tool-wiring.feature. BUG-345 tags the wiring feature next.
  """

  Background: User Story
    As an fspec developer
    I want the GenerateCompaction feature split into small files
    So that fixing a bug in its wiring scenarios touches a feature under 300 lines

  Scenario: The GenerateCompaction feature files stay under 300 lines
    Given the GenerateCompaction tool feature and its wiring feature
    When the line count of each feature file is measured
    Then no feature file has 300 or more lines
    And the three wiring scenarios are in the wiring feature and not in the tool feature
