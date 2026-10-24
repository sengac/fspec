@BUG-229
@phase-1
@agent-core
@context-management
@compaction
Feature: Compactor occupancy runway
  """
  The shared compactor execution checks complete provider-serialized exploration
  and tool-free synthesis requests before each logical round. It reserves runway
  for growing evidence without changing admission estimates or output limits.
  Capacity exhaustion can synthesize from the entire gathered history, including
  the final tool result. Unfit evidence fails explicitly rather than being cut.
  Physical Codex continuations retain their independent admission gate. Only
  typed capacity failures permit synthesis; network and provider errors do not.
  Existing source-boundary DAG validation, cancellation, durable commit and
  publication remain authoritative for both registered recovery and generation.
  """

  Scenario: Both compaction routes reserve synthesis runway as originals grow
    Given both compaction routes gather growing persisted original evidence with explicit limits
    When another exploration round would consume the synthesis runway
    Then tool-free synthesis starts before depth exhaustion with every gathered original intact
    And the complete source-covering DAG passes durable commit and publication gates

  Scenario: Evidence that cannot fit synthesis preserves the prior generation
    Given both compaction routes have a prior durable generation and an unfit original evidence page
    When gathering that page exhausts synthesis capacity
    Then compaction fails explicitly without dispatching oversized bytes or truncating evidence
    And the prior durable generation and all persisted originals remain unchanged

  Scenario: Invalid runway synthesis cannot publish a candidate
    Given both compaction routes reach occupancy synthesis with persisted original evidence
    When synthesis returns a DAG that omits the captured source boundary
    Then bounded correction fails without durable promotion or publication

  Scenario: Cancellation during runway synthesis cannot publish a candidate
    Given both compaction routes are waiting for occupancy synthesis
    When the compaction operation is cancelled
    Then no candidate is committed or published and persisted originals remain available

  Scenario: Provider errors do not masquerade as capacity exhaustion
    Given a compactor has gathered persisted original evidence
    When the provider rejects exploration with an ordinary error mentioning capacity
    Then the provider error is returned without tool-free synthesis
