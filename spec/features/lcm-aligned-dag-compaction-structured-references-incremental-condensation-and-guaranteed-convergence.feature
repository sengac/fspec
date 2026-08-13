@done
@CMPCT-016
Feature: LCM-Aligned DAG Compaction — Structured References, Incremental Condensation, and Guaranteed Convergence
  """
  Reuse resolve_turns_context() and resolve_query_context() from rust/napi/src/agent_manager/agent_manager_handler.rs — these already resolve turn ranges/queries against persisted messages using the same persistence layer as SessionSearch. Extract into shared utility.
  Watchdog retry logic in rust/napi/src/session_manager.rs agent_loop — after each run_with_provider call during compaction, checks compaction_in_progress. Attempt 1: normal stream. Attempt 2: inject COMPACTION_ESCALATION_MESSAGE, run another stream. Attempt 3: call force_inject_fallback_dag() which extracts partial dag-nodes or creates minimal fallback, resets session to reminders, and clears compaction_in_progress.
  The COMPACTION_SYSTEM_INSTRUCTION constant in interactive_helpers.rs must be split into two variants: COMPACTION_INSTRUCTION_FRESH (no existing DAG — build from scratch) and COMPACTION_INSTRUCTION_INCREMENTAL (existing DAG in context — extend, promote D0→D1, summarize only fresh turns). execute_compaction() detects which to use by checking if a compaction-dag system-reminder exists in current messages.
  inject_summary_handler::apply_pending_dag should parse <dag-node> blocks from the content, extract turn ranges, validate them against persisted message count, and store as structured metadata on the session (Vec<DagNodeMeta> with depth, turn range, label). Also scan FileModification annotations from the compacted turns and append a <dag-files> block if the agent didn't include one.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. DAG nodes must use structured <dag-node depth="Dx" turns="N-M"> XML blocks instead of free-text [SessionSearch: turns X-Y] breadcrumbs
  #   2. Turn references in dag-node blocks must use the same compact format as AgentManager context references ("1-3" for ranges, "1,3,5" for non-contiguous)
  #   3. The engine must be able to parse <dag-node> blocks and extract turn ranges for validation and scoped queries
  #   4. DAG references must be resolvable via the same resolve_turns_context() infrastructure used by AgentManager context references
  #   5. When a previous compaction-dag exists in context, the compaction instruction must tell the agent to EXTEND the existing DAG (preserve D2/D1, add new D0) — not rebuild from scratch
  #   6. Previous D0 nodes from an older compaction should be promoted to D1 during subsequent compactions when the agent determines the detailed content is no longer current
  #   7. Compaction must guarantee convergence via three-level escalation: Level 1 (normal DAG), Level 2 (aggressive bullet-point compression), Level 3 (deterministic truncation, no LLM)
  #   8. An attempt-count watchdog must detect when the agent fails to call inject_summary after each stream attempt and escalate automatically — Attempt 1 (normal), Attempt 2 (escalation message + second stream), Attempt 3 (deterministic force-inject, no LLM)
  #   9. After inject_summary, the engine must extract file paths from FileModification structural annotations and append a <dag-files> block if the agent omitted one
  #   10. File references must propagate through DAG across multiple compaction cycles — files encountered in early turns must remain discoverable even after several rounds of DAG rebuilding
  #   11. Level 3 deterministic fallback must work without any LLM call — guaranteed to produce a valid (if minimal) DAG and call inject_summary
  #   12. The compaction instruction includes a token budget for the DAG itself — agent should aim for DAG content under N tokens (matching LCM's target token parameter for summarization)
  #   13. Extend SessionSearch with optional start_turn/end_turn scope parameters. No dedicated expand tool — keeps the tool surface small and the agent already knows SessionSearch.
  #   14. Three-attempt escalation: Attempt 1 (normal DAG construction in a single stream), Attempt 2 (escalation message injected, second stream initiated), Attempt 3 (engine deterministically force-injects fallback DAG, no LLM call)
  #
  # EXAMPLES:
  #   1. Agent builds DAG with <dag-node depth="D2" turns="1-120" label="Architecture">JWT + Redis + bcrypt</dag-node>. Later, agent wants to drill down. Calls SessionSearch(show, start_turn: 1, end_turn: 120) and gets the original messages for those turns — exactly like AgentManager's resolve_turns_context()
  #   2. Second compaction triggers on a session that already has a DAG. Agent receives instruction telling it to keep existing D2/D1 nodes and only summarize fresh turns 121-200. Agent writes new D0 nodes for turns 121-200 and promotes old D0 (turns 100-120) to D1. Calls inject_summary with the updated DAG containing old D2, old+promoted D1, and new D0 nodes.
  #   3. Agent's first stream attempt during compaction fails to call inject_summary. Watchdog detects compaction_in_progress is still true, injects escalation message: 'Stop making SessionSearch calls. Write a summary and call inject_summary immediately.' Second stream attempt also fails. Engine extracts any partial dag-node blocks from recent messages, assembles them into a fallback DAG, force-injects it, clears compaction_in_progress. Agent continues working with minimal but valid context.
  #   4. Agent modified src/auth.rs, src/middleware.rs, and read src/config.ts during turns 50-120. After compaction, the DAG includes <dag-files>src/auth.rs (modified)\nsrc/middleware.rs (modified)\nsrc/config.ts (read)</dag-files>. Third compaction fires — the engine carries forward file references from the previous DAG's <dag-files> block merged with newly modified files, so none are lost.
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should we create a dedicated expand_dag_node tool (like LCM's lcm_expand) that parses <dag-node turns="..."> and auto-resolves via resolve_turns_context(), or is extending SessionSearch with an optional start_turn/end_turn scope parameter sufficient?
  #   A: Extend SessionSearch with optional start_turn/end_turn scope parameters. No dedicated expand tool — keeps the tool surface small and the agent already knows SessionSearch.
  #
  #   Q: For the watchdog turn count, 5 turns before Level 2 escalation and 7 turns before Level 3 deterministic fallback — does that feel right, or should it be more/less aggressive?
  #   A: Revised to attempt-based: 3 attempts (normal → escalation → force-inject). Counting stream attempts is more precise than counting turns — it only escalates when the system actually tried to compact and failed.
  #
  #   Q: Should the soft threshold (τ_soft early warning at ~70%) be in scope for this card, or is it a nice-to-have we defer?
  #   A: Deferred. Soft threshold early warning is out of scope for this card — focus on Gaps 1 (structured refs), 2 (incremental condensation), 3 (convergence), 4 (scoped search via SessionSearch), and 5 (file tracking).
  #
  # ========================================
  Background: User Story
    As a AI coding agent
    I want to have my compaction DAG summaries contain structured, machine-readable references to source turns that can be automatically resolved
    So that I can losslessly drill down from any DAG node to the original conversation context without manually crafting SessionSearch queries

  # ========================================
  # Gap 1: Structured DAG References
  # ========================================
  @structured-references
  Scenario: Compaction instruction guides agent to write structured dag-node blocks
    Given a session has exceeded the compaction threshold
    When execute_compaction injects the compaction system instruction
    Then the instruction tells the agent to write <dag-node depth="Dx" turns="N-M" label="..."> blocks
    And the instruction specifies the compact turn format matching AgentManager references
    And the instruction includes a target token budget for the DAG

  @structured-references
  Scenario: Engine parses dag-node blocks and extracts turn ranges from injected DAG
    Given the agent calls inject_summary with content containing <dag-node depth="D2" turns="1-120" label="Architecture">
    When the inject_summary handler processes the content
    Then it parses all <dag-node> blocks from the content
    And it extracts turn ranges using the AgentManager compact format parser
    And it validates turn ranges against the persisted message count
    And it stores Vec<DagNodeMeta> with depth, turn range, and label on the session

  @structured-references
  Scenario: Agent drills down into a DAG node via scoped SessionSearch
    Given a compaction DAG exists with <dag-node depth="D1" turns="80-120">
    When the agent calls SessionSearch with start_turn 80 and end_turn 120
    Then SessionSearch returns only messages within turns 80 through 120
    And the resolution uses the same persistence layer as AgentManager resolve_turns_context

  @structured-references
  Scenario: SessionSearch show action supports optional start_turn and end_turn parameters
    Given a session has 200 persisted messages
    When SessionSearch show is called with start_turn 50 and end_turn 75
    Then only messages at turn indices 50 through 75 are returned
    And messages outside that range are excluded

  @structured-references
  Scenario: SessionSearch search action supports optional start_turn and end_turn scope
    Given a session has 200 persisted messages
    When SessionSearch search is called with query "error" and start_turn 80 and end_turn 120
    Then only messages within turns 80-120 that match "error" are returned
    And matches outside the turn range are excluded

  # ========================================
  # Gap 2: Incremental Condensation
  # ========================================
  @incremental-condensation
  Scenario: First compaction uses fresh instruction to build DAG from scratch
    Given a session has no existing compaction-dag system reminder
    When compaction triggers
    Then execute_compaction injects the COMPACTION_INSTRUCTION_FRESH variant
    And the instruction tells the agent to build the complete DAG from scratch via SessionSearch

  @incremental-condensation
  Scenario: Subsequent compaction uses incremental instruction to extend existing DAG
    Given a session has an existing compaction-dag system reminder from a previous compaction
    When compaction triggers again
    Then execute_compaction detects the existing compaction-dag in session messages
    And injects the COMPACTION_INSTRUCTION_INCREMENTAL variant
    And the instruction tells the agent to preserve existing D2 and D1 nodes
    And the instruction tells the agent to summarize only fresh turns since the last compaction
    And the instruction tells the agent to promote stale D0 nodes to D1

  @incremental-condensation
  Scenario: Agent extends DAG with new D0 nodes and promotes old D0 to D1
    Given a previous DAG has D2 nodes for turns 1-80, D1 nodes for turns 40-80, and D0 nodes for turns 60-80
    And fresh conversation has occurred in turns 81-160
    When the agent builds an incremental DAG
    Then the D2 nodes from the previous DAG are preserved unchanged
    And the previous D0 nodes for turns 60-80 are promoted to D1
    And new D0 nodes are written for turns 140-160
    And the complete updated DAG is passed to inject_summary

  # ========================================
  # Gap 3: Guaranteed Convergence
  # ========================================
  @convergence
  Scenario: Level 2 escalation fires after first failed stream attempt
    Given compaction has been triggered and the agent is building a DAG
    And the first stream attempt completes without the agent calling inject_summary
    When the watchdog detects compaction_in_progress is still true
    Then an escalation message is injected into session messages
    And a second stream attempt is initiated automatically

  @convergence
  Scenario: Level 3 deterministic fallback fires after two failed stream attempts
    Given compaction has been triggered and the agent is building a DAG
    And both the first and second stream attempts complete without inject_summary
    When the watchdog detects two consecutive failures
    Then the engine deterministically extracts any dag-node blocks from the agent's recent messages
    And if no dag-node blocks exist it creates a minimal fallback DAG
    And the engine force-injects the assembled DAG directly bypassing the agent
    And the compaction_in_progress flag is cleared
    And the agent can continue working with minimal but valid context

  @convergence
  Scenario: Level 3 deterministic fallback requires no LLM call
    Given two stream attempts have failed during compaction
    When the engine performs deterministic force-injection
    Then zero LLM API calls are made during the force-injection
    And the injected DAG contains at minimum a session identifier and a message instructing the agent to use SessionSearch for history

  @convergence
  Scenario: Normal DAG construction completes before watchdog triggers
    Given compaction has been triggered and the agent is building a DAG
    And the agent calls inject_summary during the first stream attempt
    When the watchdog checks compaction_in_progress
    Then no escalation occurs because inject_summary was called before any retry
    And the watchdog counter is reset

  # ========================================
  # Gap 5: File ID Propagation Through DAG
  # ========================================
  @file-tracking
  Scenario: Engine appends dag-files block when agent omits file references
    Given the compacted turns contain FileModification annotations for src/auth.rs and src/middleware.rs
    And the agent's DAG content does not contain a <dag-files> block
    When inject_summary handler processes the content
    Then it extracts file paths from FileModification annotations in the compacted turns
    And appends a <dag-files> block listing all modified files with their operations

  @file-tracking
  Scenario: Agent-provided dag-files block is preserved without engine override
    Given the compacted turns contain FileModification annotations for src/auth.rs
    And the agent's DAG content already contains a <dag-files> block
    When inject_summary handler processes the content
    Then the agent's dag-files block is preserved as-is
    And no duplicate dag-files block is appended

  @file-tracking
  Scenario: File references propagate through multiple compaction cycles
    Given a first compaction DAG contains <dag-files> with src/auth.rs and src/config.ts
    And a second compaction occurs where the agent modified src/middleware.rs
    When the agent builds the incremental DAG
    Then the compaction instruction includes the previous dag-files content
    And the final DAG's dag-files block contains src/auth.rs, src/config.ts, and src/middleware.rs
