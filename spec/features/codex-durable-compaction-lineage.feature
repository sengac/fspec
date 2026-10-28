@done
@validation
@BUG-278
@cli
@context-management
@compaction
@critical
Feature: Codex durable compaction lineage
  """
  Provider generations advance from verified authoritative replay, independently of the exactly-once durable counter. Offset boundaries carry a versioned explicit durable/provider generation binding and source checkpoint digest; unbound mismatched generations remain invalid. Current-history validation uses the selected serializer without applying the replacement token budget to the overfull source. Under the manifest transaction, originals, model, DAG preservation and exact source checkpoint identity are rechecked after admission. Publication and cold restoration consume that exact committed binding. No reset, truncation, fallback, invented outcome or live repair is used.
  """

  Background: User Story
    As an AI agent I want valid selected Codex compaction to preserve authoritative lineage So that I can continue completed work safely

  Scenario: First and successive Codex compactions preserve independent authoritative generations
    Given a selected Codex runtime with genuine persisted checkpoint and tool outcomes
    When first and successive admitted compactions publish and restore cold
    Then durable and provider generations advance independently and continuation retains genuine outcomes exactly once

  Scenario: Unproven or incompatible Codex checkpoint authority is refused
    Given selected Codex checkpoint evidence that is stale absent divergent for another model or lacks a committed generation binding
    When the runtime attempts an admitted compaction
    Then original history stays unchanged without promotion publication or inference

  Scenario: Missing originals or genuine outcomes cannot authorize compaction
    Given a selected Codex source with missing original or tool outcome evidence
    When the runtime attempts an admitted compaction
    Then the missing evidence blocks promotion publication and inference without fabricated success

  Scenario: Mutation during admission cannot promote a different candidate
    Given a selected Codex candidate with authoritative lineage
    When the provider checkpoint changes during selected request admission
    Then the changed candidate is rejected without promotion publication or inference

  Scenario: Failed Codex publication reconciles only the promoted identity
    Given a selected Codex candidate with authoritative lineage
    When publication fails after durable promotion
    Then inference stays blocked until the exact promoted identity reconciles without another generation

  Scenario: Lost Codex durability acknowledgement retains exact ownership
    Given a selected Codex candidate with authoritative lineage
    When manifest promotion succeeds but directory synchronization fails
    Then inference stays blocked until the exact promoted identity reconciles without another generation

  Scenario: Stale durable Codex sources cannot advance lineage
    Given a selected Codex source superseded by a successful compaction
    When the runtime attempts to promote the old source again
    Then original history stays unchanged without another promotion publication or inference
