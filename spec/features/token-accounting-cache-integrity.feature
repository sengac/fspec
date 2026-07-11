@done
@agent-core
@context-management
@rpc
@compaction
@CMPCT-041
Feature: Token accounting cache integrity — turn-start seeding, compaction snapshots, and billing report the true context exactly once
  """
  CMPCT-041 root fix: the token tracker stores the CACHE-INCLUSIVE total (PROV-001) alongside non-zero cache fields, so seeding StreamingTokenDisplay with both re-adds cache via TokenDisplayUpdate::total_input() at the emit. All seed/re-seed sites in stream_loop.rs route through a single audited StreamingTokenDisplay constructor (from_cache_inclusive_total) that de-overlaps raw = total - cache, preserving the split so the pre-compaction flush bills only raw input (dossier 6.2) while the emitted total stays exact. A consistency guard falls back to (total, 0, 0) when stale cache exceeds the total. Snapshot basis unification for the four pre_compaction_tokens writers is specified in pre-compaction-snapshot-basis-unification.feature.
  """

  Background: User Story
    As a codelet session user
    I want to have turn-start token seeding, compaction snapshots, and billing all report the true cache-inclusive context exactly once
    So that compaction metrics and billing analytics never show a context that never existed

  Scenario: Turn-start seed emit reports the true cache-inclusive total exactly once
    Given a completed turn left the token tracker at a cache-inclusive total of 180000 tokens with 150000 cache-read and 0 cache-creation tokens
    When the next turn seeds the streaming token display from the tracker and emits the initial token state
    Then the emitted token update reports 180000 input tokens
    And the emitted token update never reports the double-counted 330000 tokens

  Scenario: Pre-compaction flush in the no-usage-event window preserves tracker and billing integrity
    Given the streaming token display was seeded from a tracker total of 180000 tokens including 150000 cache-read tokens
    And no Usage event has arrived this turn because the prompt was rejected as too long
    When partial state is flushed before compaction recovery
    Then the token tracker still reads 180000 input tokens
    And cumulative billed input grows by only the 30000 raw input tokens

  Scenario: Inconsistent stale cache split falls back to the trusted total
    Given a freshly recalculated post-compaction tracker total of 5000 tokens alongside stale cache-read values of 150000 tokens
    When the streaming token display is seeded from those tracker values
    Then the emitted token update reports 5000 input tokens
    And the stale cache split is dropped instead of inflating the total

  Scenario: Mid-stream Usage self-heal behavior is unchanged
    Given a streaming token display seeded from a cache-inclusive tracker total of 180000 tokens
    When an authoritative mid-stream Usage event arrives with 35000 raw input, 160000 cache-read, and 0 cache-creation tokens
    Then the display reports the authoritative 195000-token total
    And subsequent emits use the authoritative raw and cache values from the Usage event
