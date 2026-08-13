@done
@agent-core
@context-management
@rpc
@compaction
@CMPCT-041
Feature: Pre-compaction snapshot basis unification — auto and manual writers agree across both twins
  """
  AUTO pre_compaction_tokens writers cannot read token_tracker mid-stream: BackgroundSession.inner is a tokio::sync::Mutex held by the streaming agent loop for the whole turn (see pending_dag_content comment in background_session.rs). Per CMPCT-041 dossier 5.2's documented alternative, AUTO keeps cached_input_tokens as its source; after the root seed fix (token-accounting-cache-integrity.feature) this equals the tracker basis in the seed window. Equivalence is pinned by the auto/manual parity test (dossier 6.3).
  All pre_compaction_tokens writes are routed through shared BackgroundSession accessors in codelet-sessions (snapshot_pre_compaction_tokens for the AUTO twins, store_pre_compaction_tokens for the manual twins), so basis drift is structurally impossible across the NAPI/agent-loop twins.
  Test gating: the behavioral parity tests in rust/agent-loop/tests/cmpct041_pre_compaction_basis_test.rs require `cargo test -p codelet-agent-loop --features test-support` (hermetic stub provider, mirroring rpc086_token_tracking.rs). A default `cargo test -p codelet-agent-loop` run executes ONLY the supplementary structural wiring guard (all_four_pre_compaction_writers_route_through_shared_accessors), not the behavioral scenarios.
  """

  Background: User Story
    As a codelet session user
    I want to have turn-start token seeding, compaction snapshots, and billing all report the true cache-inclusive context exactly once
    So that compaction metrics and billing analytics never show a context that never existed

  Scenario: Overflow-recovery snapshot in the seed window records the true context total
    Given a background session whose display pipeline emitted the turn-start seed for a 180000-token context with 150000 cache-read tokens
    When overflow recovery starts compaction and snapshots the pre-compaction token count
    Then the pre-compaction snapshot records 180000 tokens
    And the snapshot never records the double-counted 330000 tokens

  Scenario: Auto and manual compaction snapshots agree on the same basis across both twins
    Given a background session whose token tracker reads 180000 tokens and whose cached display tokens were fed by the seed emit
    When the auto-compaction path snapshots the pre-compaction token count
    And the manual compaction path reads its tracker-based original token count
    Then both paths report the same 180000-token basis
    And both the agent-loop twin and the NAPI twin route the snapshot through the same shared session accessor
