@done
@agent-core
@context-management
@compaction
@CMPCT-042
Feature: Gemini continuation TokenState double-counts cache — cache-inclusive tracker total combined with non-zero cache fields can trigger compaction early
  """
  Fix site: rust/cli/src/interactive/gemini_continuation.rs:126-132. tracker.input_tokens is cache-INCLUSIVE after update_display_only (model.rs:189 uses usage.total_input()). Fix: zero cache_read_input_tokens/cache_creation_input_tokens in the continuation TokenState, mirroring the guarded pattern at stream_loop.rs:398-401 ('Don't double count'). Do NOT touch the nested seed at :310-316 (display-basis, correct) or the StreamingTokenDisplay::new seeds at :158/:343 (pinned by cmpct041_seed_cache_double_count_test.rs).
  """

  Background: User Story
    As a codelet CLI user on a Gemini model
    I want to have the compaction threshold check during Gemini continuations use the true context total
    So that compaction is not triggered prematurely by cache tokens being counted twice

  Scenario: Continuation TokenState reports the true total when the tracker total is cache-inclusive
    Given the token tracker was updated via update_display_only with 30k fresh input, 150k cache read and 2k cumulative output
    When the Gemini continuation TokenState is seeded from the tracker
    Then the TokenState total is 182k
    And the TokenState total is not 332k

  Scenario: CompactionHook does not trigger early during a Gemini continuation
    Given a continuation TokenState seeded from a 180k cache-inclusive tracker total with 2k output
    And a compaction threshold of 200k that lies between the true total and the cache-inflated total
    When the CompactionHook evaluates the threshold before the continuation call
    Then compaction_needed remains false

  Scenario: CompactionHook still triggers when the true total genuinely exceeds the threshold
    Given a continuation TokenState seeded from a 180k cache-inclusive tracker total with 2k output
    And a compaction threshold of 150k that is below the true total
    When the CompactionHook evaluates the threshold before the continuation call
    Then compaction_needed is set to true

  Scenario: Continuation TokenState construction site routes through the audited constructor
    Given the gemini_continuation.rs source file
    When the continuation TokenState construction at the tracker-basis seed site is inspected
    Then the seed routes through TokenState from_cache_inclusive_total
    And the cache-inflating seed shape with tracker total plus display cache fields is absent

  Scenario: Nested continuation TokenState keeps its display-basis seeding unchanged
    Given a nested continuation TokenState seeded from a display snapshot with 30k raw input, 150k cache read and 2k output
    When the TokenState total is computed
    Then the total is 182k with cache fields carried separately from raw input
