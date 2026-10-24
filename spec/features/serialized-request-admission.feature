@done
@CMPCT-071
@providers
@compaction
@context-management
@phase-1
Feature: Admit complete serialized requests against explicit usable limits
  """
  Admission measures the complete selected request without rewriting its bytes:
  fixed instructions, unchanged DAG, verified suffix, tools, media and output
  reservation. Missing or invalid limits and unknown media fail explicitly.
  Estimated occupancy remains estimated, never authoritative measured usage.
  """

  Scenario: Serialized schemas and retained suffix participate in the same budget
    Given a selected request containing instructions and a small unchanged DAG
    And large tool schemas and a retained conversation suffix
    When the complete serialized request is admitted against explicit limits
    Then the schemas and suffix can veto admission without changing the request

  Scenario: Unknown media occupancy cannot certify request fit
    Given a selected request whose media occupancy is unavailable
    When the complete serialized request is admitted against explicit limits
    Then admission returns a typed unavailable-media failure

  Scenario: Successful estimated admission preserves its request and provenance
    Given a fitting selected request with explicit media occupancy and output reservation
    When the complete serialized request is admitted against explicit limits
    Then admission retains the exact selected bytes and reports estimated occupancy

  Scenario: Unusable limits and unrepresentable occupancy cannot authorize admission
    Given complete selected request bytes and unavailable or unusable admission limits
    When admission checks the limits and explicit media occupancy
    Then every invalid budget returns its typed failure without changing request bytes
