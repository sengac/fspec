@BUG-491
@BUG-315
@coverage
@validator
@BUG-458
Feature: Coverage implementation mappings reference existing source
  """
  Implementation mappings in spec/features/*.feature.coverage name a source file and line
  numbers. Mappings whose file no longer exists, or whose lines run past the end of the
  file, are lineage drift: they no longer describe implementation. They are removed rather
  than remapped. rust/fspec/tests/coverage_impl_mapping_integrity.rs scans every sidecar.
  """

  Background: User Story
    As a developer reading coverage reports
    I want every implementation mapping to point at real source lines
    So that coverage describes the code that exists today

  Scenario: Every implementation mapping points at existing source lines
    Given the coverage sidecars under spec/features
    When every implementation mapping is resolved against the repository
    Then each mapped file exists
    And each mapped line is within that file
