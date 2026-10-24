@done
@CMPCT-083
@phase-1
@integration
@compaction
@context-management
@high
Feature: Gate compaction failures on typed rejection without alternate success
  """
  Implementation:
  - The negative integration target for the CMPCT-083 gate is specified separately to retain the one-test-target-per-feature transition invariant. The selected-provider repeated cold-resume matrix and recovery cases remain in cold-resume-compaction-acceptance-gate.feature. No negative row or no-fallback requirement is removed by this traceability split.
  - Real selected adapters and production runtime owners handle each failure. Local HTTP servers redirect physical transports without substituting application logic. Compaction state, immutable originals, typed errors, blocked inference, alternate request count and completion notices are checked.
  - Replay-gap failure compares the corrupted fixture's manifest bytes, authoritative identity, live state and immutable original payloads before repairing the fixture. Cleanup cannot conceal partial mutation by the rejected operation.
  """

  Background: User Story
    As an agent operator
    I want invalid compaction and replay to fail without alternate requests
    So that authoritative committed memory cannot be silently replaced

  Scenario Outline: Compaction failures terminate without an alternate successful request
    Given a session with an authoritative committed DAG generation
    When <failure> interrupts the selected compaction or replay path
    Then the operation terminates boundedly with a typed failure
    And the committed generation and immutable originals remain unchanged
    And ordinary inference is blocked with zero alternate requests and no completion notice

    Examples:
      | failure                         |
      | missing handler                 |
      | bad candidate                   |
      | timeout                         |
      | stale generation                |
      | durable write failure           |
      | structured replay gap           |
      | websocket upgrade rejection     |
      | incompatible checkpoint lineage |
