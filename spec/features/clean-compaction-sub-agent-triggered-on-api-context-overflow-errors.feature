@agent-core
@context-management
@compaction
@CMPCT-044
Feature: Clean compaction sub-agent triggered on API context-overflow errors

  """
  Frontend topology (verified 2026-09-16): the legacy REPL was deleted (commit d2ff9b16) — codelet-cli (rust/cli) is a lib-only shared streaming engine, NOT a binary. The live agent-session frontend is the Rust TUI inside the fspec binary (rust/fspec, combined/daemon/client modes) via codelet-rpc, driven by codelet-agent-loop (FspecAgentHooks, fspec/src/common.rs:128). codelet-napi (rust/napi) is a DORMANT legacy twin with a frozen copy of the agent loop — nothing in the live dependency graph references it. CMPCT-044 wiring: classifier + pin primitive + stream-loop trigger in codelet-cli; compactor sub-agent spawner + per-session handler registration in codelet-agent-loop (mirroring register_deep_search_handler in bridges.rs). No codelet-napi changes. Handler-registry pattern (codelet-tools CompactorSubAgentHandler) keeps codelet-cli decoupled from the spawner and avoids changing run_agent_stream_with_images' canonical 9-arg signature (pinned by rpc084_streaming tests).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A new is_context_overflow_error classifier must be a strict superset of is_prompt_too_long_error: it must match provider-variant wording (OpenAI 'maximum context length', OpenAI-compatible 'context length exceeded' / 'input is too long', Bedrock/Vertex variants) and must keep the PROV-010 exclusion — thinking-budget errors containing 'budget_tokens' must NEVER trigger compaction.
  #   2. In the stream-loop error cascade the overflow check must run BEFORE the transient-network check (NET-001), because some providers' context-overflow 400s abort the SSE stream and would otherwise be misclassified as network errors and retried against the same oversized payload.
  #   3. The compactor sub-agent runs in an EPHEMERAL session (Uuid::new_v4, DeepSearch pattern) with a clean context, inheriting the parent session's provider and model (BUG-102 pattern). Its only context source for the dying session is SessionSearch over the parent's session id — it must NOT read the parent's in-memory message list.
  #   4. The pin is handler-side, not a tool call (v1): after the sub-agent returns the DAG as its final response, the trigger site (production code, not the LLM) performs the apply_pending_dag mutation under the parent session lock — reset_session_to_reminders, push wrap_dag_content(dag), recalculate_token_tracker. The sub-agent's tool surface stays the 7 read-only DeepSearch base set (no inject_summary tool), so it cannot mis-target or loop.
  #   5. The compactor sub-agent is bounded by a wall-clock timeout (reusing deep_search_wall_clock_timeout, AMGR-016 pattern) and a fixed max tool-turn depth. On timeout, failure, or unparseable DAG output, the trigger falls back to a FREE force-inject fallback DAG (CMPCT-020 Level-3 pattern — extract_partial_dag_nodes then force_inject_fallback_dag) so the session is ALWAYS reduced, never left oversized.
  #   6. The compaction_in_progress flag must be set for the whole sub-agent run so SessionSearch Layer-0 trimming applies to the sub-agent's reads of the parent (keeping the sub-agent's own context small), and cleared after the pin completes — consistent with the in-view flow's use of the flag.
  #   7. The compactor sub-agent must use the INCREMENTAL compaction instruction when the parent session already contains a DAG (detected via detect_existing_dag BEFORE any clear), and the FRESH instruction otherwise — preserving D2 durable nodes across sub-agent compactions exactly like the in-view flow does.
  #   8. The sub-agent is the SECOND STAGE of overflow recovery: in-view compaction (Paths A/B/C/D) remains the primary in-place mechanism. The sub-agent fires when the API returns an overflow error the in-loop recovery cannot resolve — including the retry streams issued by in_loop_compaction_restart (they are governed by the same error cascade, so a second overflow routes to the sub-agent without new wiring).
  #   9. All production wiring lives in the LIVE frontend only: codelet-cli (classifier, pin primitive, stream-loop trigger) + codelet-agent-loop (compactor sub-agent spawner, per-session handler registration mirroring register_deep_search_handler) + codelet-tools (handler registry + types). The dormant codelet-napi legacy twin gets NO changes; the legacy REPL no longer exists (deleted in d2ff9b16) and no new entry points are added to codelet-cli.
  #
  # EXAMPLES:
  #   1. OpenAI-compatible provider returns 400 'Input is too long: 201,000 tokens > 200,000 maximum' mid-stream. is_context_overflow_error matches (no existing substring matches). The trigger runs begin_compaction_recovery, spawns the compactor sub-agent (ephemeral session, parent's provider/model), the sub-agent surveys the parent via SessionSearch(session_id=<parent>), returns the DAG, the trigger clears the parent to reminders + pins the wrapped DAG, and re-issues the stream with the CMPCT-028 policy prompt. The turn continues on the reduced context.
  #   2. The compactor sub-agent hits its wall-clock timeout while surveying a very large session. The trigger extracts any partial <dag-node> blocks from the sub-agent's output, and if none exist creates a generic 'Auto-recovered: compaction timeout' D1 node; it pins that fallback DAG to the parent session (zero LLM cost) and resumes the stream. The session survives with reduced-but-coarse context instead of dying.
  #   3. The parent session already contains a DAG from a previous compaction. The sub-agent receives the INCREMENTAL instruction embedding the existing DAG and the last-compact end turn: it preserves D2 nodes, promotes D0→D1, and only searches turns since the last compaction (start_turn=<last_compacted_turn>) before returning the updated DAG. Durable architecture decisions survive sub-agent compactions unchanged.
  #   4. A provider's overflow 400 aborts the SSE stream mid-response and the error surfaces as a stream-level error ('stream closed before completion' wrapping a 400 body). Because the overflow check runs BEFORE the NET-001 transient-network arm, the session compacts via the sub-agent exactly once instead of being silently network-retried MAX_NETWORK_RETRIES times against the same oversized payload.
  #   5. A background (TUI/subordinate) session's turn dies with an unmatched overflow error. The agent-loop terminal-error arm routes it to the same compactor entry point: it reads the parent's persisted turns, compacts, and the session returns to Idle with reminders + DAG. The next user message starts a fresh stream on the reduced context — the oversized payload is never replayed.
  #
  # ========================================

  Background: User Story
    As a long-running agent session
    I want to survive provider context-overflow API errors via a clean compaction sub-agent
    So that sessions compact and resume instead of dying when the error wording isn't caught by the existing string classifiers

  # ============================================================================
  # Classifier: robust context-overflow detection
  # ============================================================================

  @compaction
  @context-management
  @regression
  Scenario: Overflow classifier matches every error is_prompt_too_long_error matches
    Given a provider context-overflow error string that the existing is_prompt_too_long_error classifier matches
    When is_context_overflow_error is called with that error string
    Then is_context_overflow_error returns true
    And the new classifier matches it for the same reason as the old one

  @compaction
  @context-management
  Scenario: Overflow classifier matches provider-variant wording the old classifier misses
    Given an OpenAI-compatible provider error "Input is too long: 201,000 tokens > 200,000 maximum"
    When is_context_overflow_error is called with that error string
    Then is_context_overflow_error returns true
    And is_prompt_too_long_error returns false for that same string

  @compaction
  @context-management
  Scenario: Overflow classifier matches Bedrock and Vertex context-limit wording
    Given a provider error in the style "The input (approximately 200,500 tokens) exceeds the maximum number of input tokens (200,000)"
    When is_context_overflow_error is called with that error string
    Then is_context_overflow_error returns true

  @compaction
  @context-management
  @regression
  Scenario: Overflow classifier rejects thinking-budget configuration errors
    Given an API error containing "budget_tokens" from a thinking-budget configuration failure
    When is_context_overflow_error is called with that error string
    Then is_context_overflow_error returns false

  @compaction
  @context-management
  @regression
  Scenario: Overflow classifier rejects truncation and unrelated errors
    Given an API error that is NOT a context-overflow error (a truncation, rate-limit, or auth error)
    When is_context_overflow_error is called with that error string
    Then is_context_overflow_error returns false

  @compaction
  @context-management
  Scenario: Overflow classifier sees through anyhow context wrapping
    Given a context-overflow API error that is wrapped with anyhow context layers
    When is_context_overflow_error is called with the wrapped error
    Then is_context_overflow_error returns true based on the full error chain

  # ============================================================================
  # Trigger: error cascade ordering in the stream loop
  # ============================================================================

  @compaction
  @context-management
  @integration
  Scenario: Unmatched overflow error triggers compaction instead of terminating the session
    Given a streaming session with compactable turns whose context exceeds the provider limit
    And the provider returns a 400 whose body matches no existing is_prompt_too_long_error substring
    When the stream loop processes the error
    Then is_context_overflow_error classifies it as a context overflow
    And the compaction-recovery path runs for the session
    And the session does NOT terminate with a terminal "Agent error"

  @compaction
  @context-management
  @integration
  @regression
  Scenario: Overflow error is classified before the transient-network retry arm
    Given a provider context-overflow 400 that aborts the SSE stream mid-response
    And the error surface also matches a transient-network pattern such as "stream closed before completion"
    When the stream loop classifies the error
    Then the overflow branch is taken
    And the error is NOT retried as a transient network error
    And no "Reconnecting..." network-retry status is emitted for that error

  @compaction
  @context-management
  @integration
  Scenario: Overflow classifier defers to the typed PromptCancelled branch
    Given a stream error that carries a typed PromptError::PromptCancelled in its chain
    When the stream loop classifies the error
    Then classify_compaction_branch routes it to the compaction-cancel recovery path first
    And the context-overflow string classifier is not the deciding signal for that error

  @compaction
  @context-management
  @integration
  Scenario: Overflow without compactable turns does not trigger compaction
    Given a streaming session with only system-reminder messages and no user/assistant turns
    And the provider returns a context-overflow error
    When the stream loop processes the error
    Then no compaction is triggered because there are no compactable turns
    And the error follows the existing terminal error handling

  # ============================================================================
  # Compactor sub-agent: spawner contract
  # ============================================================================

  @compaction
  @context-management
  @tools
  @integration
  Scenario: Compactor sub-agent runs in an ephemeral clean session
    Given a parent session whose provider and model are configured
    When the compactor sub-agent is spawned for the parent session
    Then the sub-agent runs under a fresh ephemeral session id that is not the parent's id
    And the sub-agent inherits the parent session's provider and model

  @compaction
  @context-management
  @tools
  Scenario: Compactor sub-agent has only the read-only tool surface
    Given a compactor sub-agent is constructed
    When its tool list is built
    Then the sub-agent has exactly the seven read-only tools: Read, Grep, AstGrep, Glob, Ls, Bash, and SessionSearch
    And the sub-agent does NOT have the inject_summary tool
    And the sub-agent does NOT have any Write or Edit tool

  @compaction
  @context-management
  @persistence
  @integration
  Scenario: Compactor sub-agent surveys the parent via SessionSearch on the parent session id
    Given a parent session with persisted turns
    When the compactor sub-agent is asked to build the compaction DAG
    Then the sub-agent's SessionSearch calls target the parent session id, not the sub-agent's own ephemeral session
    And the sub-agent never reads the parent's in-memory message list

  @compaction
  @context-management
  Scenario: Compaction instruction is FRESH when the parent has no DAG
    Given a parent session that contains no existing compaction DAG
    When the compactor sub-agent's task prompt is built
    Then the FRESH compaction instruction is used
    And the task prompt names the parent session id for every SessionSearch call
    And the task prompt instructs the sub-agent to output the complete DAG as its final response instead of calling inject_summary

  @compaction
  @context-management
  @regression
  Scenario: Compaction instruction is INCREMENTAL when the parent already has a DAG
    Given a parent session whose context already contains a compaction DAG ending at turn N
    When the compactor sub-agent's task prompt is built
    Then the INCREMENTAL compaction instruction is used with the existing DAG embedded
    And the task prompt tells the sub-agent to preserve D2 nodes, promote D0 to D1, and only survey turns from N+1 onward

  # ============================================================================
  # Pin: handler-side clear-and-pin of the parent session
  # ============================================================================

  @compaction
  @context-management
  @session
  @integration
  Scenario: Successful sub-agent DAG is pinned to the parent session
    Given a parent session with system reminders and many compactable conversation messages
    And the compactor sub-agent returns a parseable DAG with at least one dag-node block
    When the trigger site pins the sub-agent's DAG to the parent
    Then the parent's in-memory messages are replaced by the system reminders plus the wrapped DAG
    And the parent's turn list is cleared
    And the parent's token tracker is recalculated from the reduced message list

  @compaction
  @context-management
  @session
  Scenario: Pinning a sub-agent DAG keeps the persisted history intact
    Given a parent session whose turns are persisted in the session manifest
    When the trigger site pins the sub-agent's DAG to the parent
    Then the in-memory context is reduced to reminders plus the DAG
    And the persisted session manifest history is NOT truncated
    And the DAG content is wrapped in the compaction-dag system-reminder wrapper

  # ============================================================================
  # Fallback: convergence guarantee when the sub-agent fails
  # ============================================================================

  @compaction
  @context-management
  @session
  @regression
  Scenario: Sub-agent timeout pins a free fallback DAG
    Given a parent session that is over its context limit
    And the compactor sub-agent exceeds its wall-clock timeout without returning a DAG
    When the trigger site completes compaction
    Then a fallback DAG is pinned to the parent session without any further LLM call
    And the fallback DAG is a generic auto-recovered node covering the session's turns
    And the parent's in-memory context is reduced to reminders plus the fallback DAG

  @compaction
  @context-management
  @session
  Scenario: Unparseable sub-agent output pins a fallback DAG
    Given the compactor sub-agent returns final text containing no parseable dag-node blocks
    When the trigger site completes compaction
    Then a fallback DAG is pinned to the parent session
    And the parent's in-memory context is reduced to reminders plus the fallback DAG

  @compaction
  @context-management
  @session
  Scenario: Partial dag-node blocks from a failed sub-agent are recovered
    Given the compactor sub-agent times out after emitting some complete dag-node blocks but no final DAG
    When the trigger site completes compaction
    Then the fallback DAG is assembled from the partial dag-node blocks
    And the parent's in-memory context is reduced to reminders plus the recovered DAG

  # ============================================================================
  # Integration: flag discipline, lifecycle, and the background session path
  # ============================================================================

  @compaction
  @context-management
  @integration
  Scenario: compaction_in_progress gates sub-agent reads and is cleared after the pin
    Given the compactor sub-agent run begins for an over-limit parent session
    When the sub-agent runs and reads the parent via SessionSearch
    Then the compaction_in_progress flag is true during the sub-agent run so Layer-0 trimming applies to its reads
    And the compaction_in_progress flag is false after the pin completes

  @compaction
  @context-management
  @integration
  Scenario: Compaction lifecycle events are emitted for the sub-agent path
    Given an overflow error triggers the compactor sub-agent for a session
    When the sub-agent run and pin complete
    Then a compaction-started event was emitted before the sub-agent was spawned
    And a compaction-complete event is emitted after the pin with the recalculated post-pin token basis
    And the CompactionComplete event reflects the reduced context, not just the DAG summary size

  @compaction
  @context-management
  @session
  @integration
  Scenario: Background session terminal overflow error compacts via the same entry point
    Given a background agent-loop session whose turn dies with a context-overflow error that no classifier matched
    When the agent-loop terminal-error arm processes the error
    Then the compactor recovery entry point runs for that session
    And the session's context is cleared to reminders plus a DAG
    And the session returns to Idle
    And the next user message starts a fresh stream on the reduced context

  @compaction
  @context-management
  @integration
  Scenario: Handler registration lifecycle mirrors the DeepSearch pattern
    Given a background session is created in the agent loop
    When the session's handlers are registered
    Then a compactor-sub-agent handler is registered for the session id capturing the provider, model, and project path
    And the handler is removed for the session id during end-of-turn cleanup

  @compaction
  @context-management
  @session
  @regression
  Scenario: Missing compactor handler still guarantees a reduced session
    Given a session for which no compactor-sub-agent handler is registered
    And the session is over its context limit with a terminal overflow error
    When the trigger site attempts compactor recovery
    Then a fallback DAG is force-injected into the session without any LLM call
    And the session's in-memory context is reduced to reminders plus the fallback DAG
    And the failure is surfaced with a structured compaction-failed lifecycle event

  @compaction
  @context-management
  @regression
  Scenario: Sub-agent escalation is bounded by the shared retry budget
    Given the in-loop in-view compaction retry budget has been exhausted for a turn
    And another context-overflow error occurs
    When the error cascade processes it
    Then the turn terminates with the structured compaction-budget-exhausted error
    And the compactor sub-agent is not spawned for a turn past the retry budget
