@CMPCT-071
@compaction
@agent-core
@phase-1
@validation
Feature: Whole request admission before DAG commit
  """
  All pin routes require one admission veto before durable promotion. Admission
  uses the complete selected provider request, not DAG size or a builder bound:
  mandatory instructions, unchanged validated DAG, verified suffix, schemas,
  media and output reservation. Limits must be explicit and usable. Measurement
  provenance remains measured or estimated; a trigger threshold is not capacity.
  Rejection preserves the committed generation and originals, blocks inference
  and emits no completion or alternate request. Runtime builder bounds do not
  certify request fit. CMPCT-065, CMPCT-070 and CMPCT-082 are prerequisites.
  """

  Background: User Story
    As an agent preserving committed hierarchical memory
    I want the entire next request admitted before a DAG replaces memory
    So that compaction cannot claim success while mandatory context still overflows

  Scenario: Mandatory fixed context cannot be shrunk into a successful commit
    Given mandatory instructions larger than an explicit provider capacity
    And a valid small DAG returned by the selected builder
    When the compaction round attempts to commit the candidate
    Then the round terminates boundedly with a blocked admission failure
    And the previous memory and token state remain unchanged without completion

  Scenario: Output reservation participates in candidate admission
    Given mandatory instructions below capacity but above the usable input budget
    And a valid small DAG returned by the selected builder
    When the compaction round attempts to commit the candidate
    Then the round terminates boundedly with a blocked admission failure
    And the previous memory and token state remain unchanged without completion

  Scenario: Missing capacity cannot authorize a DAG replacement
    Given an explicitly unavailable provider capacity
    And a valid small DAG returned by the selected builder
    When the compaction round attempts to commit the candidate
    Then the round terminates boundedly with a blocked admission failure
    And the previous memory and token state remain unchanged without completion

  Scenario: A fitting complete request remains eligible for commit
    Given explicit usable limits large enough for all retained context
    And a valid small DAG returned by the selected builder
    When the compaction round attempts to commit the candidate
    Then the selected unchanged DAG replaces the old view with one completion
