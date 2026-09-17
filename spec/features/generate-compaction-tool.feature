@done
@agent-core
@tools
@context-management
@CMPCT-045
Feature: GenerateCompaction tool — DeepSearch-clone tool that builds a compaction DAG for a session and pins it
  """
  New/changed code inventory: (1) rust/tools/src/generate_compaction/mod.rs — GenerateCompactionTool + GenerateCompactionArgs{session_id: Option<String>} + GenerateCompactionHandler registry (set/has/clear, mirroring deep_search/mod.rs), re-exported from tools lib.rs; (2) rust/agent-loop/src/generate_compaction_handler.rs — execute_generate_compaction spawner (clone of deep_search_handler.rs, compaction prompt instead of DeepSearch system prompt, 7 read-only tools, AMGR-016 wall-clock timeout, BUG-102 provider inheritance) + register_generate_compaction_handler in bridges.rs (captures facade_override/current_provider, model, context_window, max_output, project path, owning SessionManager); (3) call sites in agent-loop/src/agent_loop.rs: registration beside line ~740 (session creation) + cleanup beside set_deep_search_handler(session.id, None) at ~1507; (4) provider tool chains: .tool(GenerateCompactionTool::new(session_id)) beside DeepSearchTool in claude/openai/gemini/codex/zai/copilot/custom create_rig_agent(); (5) rust/cli/src/compaction_dag.rs: build_generate_compaction_prompt(target_uuid, existing_dag: Option<(String, usize)>); (6) pin primitive callable with a DAG string on the agent-loop side (reuse force_inject_fallback_dag shape: reset_session_to_reminders + push wrap_dag_content(dag) + recalculate_token_tracker + reset_after_compaction).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A provided `session_id` must parse as a UUID. On parse failure the tool returns `ToolError::Validation` with a usage hint (mirroring DeepSearch's empty-query validation, TOOL-024 recovery surface) — it does NOT attempt dispatch and does NOT fall back to the calling session.
  #   2. The tool takes an optional target session id (argument `session_id`). When omitted or `null`, the target is the CALLING session — the session whose tool registry entry invoked the tool (the `session_id` the tool was constructed with).
  #   3. The compactor sub-agent is ephemeral (DeepSearch pattern): a fresh Uuid::new_v4() never shared with the target or caller, no persisted session record, no worktree, no NAPI boundary. Its SessionSearch handler is registered for the ephemeral id before the run and removed by a drop guard after (guaranteed even on panic). The target session's compaction_in_progress flag is passed as the handler's compaction_trimming gate so Layer-0 trimming applies to the sub-agent's reads of the target (CMPCT-044 Rule [5]).
  #   4. Handler lookup is per CALLING session (the tool's construction session_id), mirroring DeepSearch. When no GenerateCompactionHandler is registered for the calling session, the tool returns ToolError::Execution with the 'handler not configured for session <uuid> — GenerateCompactionTool requires session context' message pattern. Nothing is spawned or pinned.
  #   5. FRESH vs INCREMENTAL selection: detect_existing_dag() scans the TARGET session's messages for the `<!-- type:compaction-dag -->` marker and must be captured BEFORE any clear (Some((content, max_turn_end)) ⇒ INCREMENTAL with the existing DAG embedded and `last_compacted_turn = max_turn_end + 1`; None ⇒ FRESH) — exactly mirroring execute_compaction's CMPCT-019 logic.
  #   6. The compactor sub-agent's context source for the target session is EXCLUSIVELY SessionSearch over the target UUID — every SessionSearch call the sub-agent makes must target the target session (the sub-agent's own session is its ephemeral one). It must NOT read the target's in-memory message list. The sub-agent's tool surface stays the 7 read-only DeepSearch base set (Read, Grep, AstGrep, Glob, Ls, Bash, SessionSearch) — no inject_summary, no Write/Edit (CMPCT-044 Rule [3]).
  #   7. The compactor sub-agent inherits the CALLING session's provider, model, context_window, and max_output_tokens via ProviderManager::with_provider_and_model (BUG-102 pattern), captured by the registered handler closure at registration time so `/model` / `/provider` changes are reflected (re-registration mirrors register_deep_search_handler).
  #   8. The compactor sub-agent is bounded by the shared AMGR-016 wall-clock timeout (deep_search_wall_clock_timeout, default 600s) and the fixed sub-agent tool-depth. On timeout, spawn/build failure, LLM error, or unparseable DAG output, the handler falls back to a FREE force-inject: extract any partial <dag-node> blocks from the sub-agent's final text; if none, emit the generic Level-3 `Auto-recovered: compaction timeout` D1 node; then pin that fallback DAG with zero further LLM cost (CMPCT-044 Rule [4], CMPCT-020 Level-3 shape).
  #   9. Handler-side pin (Option A / CMPCT-044 Rule [3]): when the sub-agent returns a DAG whose text contains at least one parseable <dag-node> block (validated with codelet_core::compaction::parse_dag_nodes), the handler performs the apply_pending_dag mutation on the target — reset_session_to_reminders → push wrap_dag_content(dag) as a User message → recalculate_token_tracker → reset_after_compaction on the token tracker. The DAG source is the sub-agent's final response, not pending_dag_content.
  #   10. The target's compaction_in_progress flag is set true for the whole sub-agent run (gating Layer-0 trimming on the sub-agent's SessionSearch reads of the target) and cleared false after the pin completes — consistent with the in-view flow (CMPCT-044 Rule [5]). The target's pre-compaction token basis is snapshotted before the sub-agent starts (CMPCT-038/CMPCT-041 basis rule).
  #   11. Target resolution and pinning strategy: the tool resolves target = session_id or the calling session id. If target == calling session, the tool executes inside the caller's turn (the caller's inner lock is held by the agent loop for the whole turn), so the handler CANNOT lock the target's inner itself (deadlock). Instead: the handler sets target.compaction_in_progress, stashes the (wrapped) DAG in the calling session's pending_dag_content, and returns the DAG text as the tool result immediately. The EXISTING end-of-turn apply_pending_dag_and_emit path then performs the pin + CompactionComplete emit + flag clear (agent_loop.rs cleanup block). If target != calling session, the handler locks the target's inner (safe — a different session), pins immediately, emits CompactionComplete on the target, and clears the target's flag before returning.
  #   12. A target session that is not a live in-memory session (unknown UUID, or not registered in the owning SessionManager) is rejected with ToolError::Execution 'target session <uuid> is not a live session' — the DAG is read-only and pinning requires a live session's in-memory message list; no partial pinning of persisted history happens.
  #   13. GenerateCompactionTool::new(session_id) is wired into every provider's create_rig_agent() tool chain (claude, openai, gemini, codex, zai, github-copilot, custom provider) exactly like DeepSearchTool — the agent can invoke it in any session. The ephemeral compactor sub-agent's 7-tool surface does NOT include GenerateCompaction (no recursion into compaction). The per-session handler is registered at session creation (agent_loop.rs registration site) and after /model / /provider changes, and removed in the end-of-turn cleanup block beside set_deep_search_handler(session.id, None).
  #   14. Lifecycle events are emitted on the TARGET session's output: before spawning the sub-agent the target gets SessionStateChange(Compacting) + a compaction progress update ("Compaction sub-agent working") + snapshot_pre_compaction_tokens; after the pin the target gets SessionStateChange(Running) followed by CompactionComplete with the recalculated post-pin token basis (CMPCT-038 measurement rule — compacted_tokens is the recalculated tracker total, not the DAG summary size; ratio via compression_ratio). The target's status flips to Compacting for the run and back after the pin. A compaction-failed user notification is emitted only if the fallback pin itself fails (the session must ALWAYS be reduced — never left oversized).
  #   15. On success (parseable DAG), the tool result string is the DAG text itself (so the caller sees exactly what was pinned). On fallback, the tool result string is the fallback DAG text with a structured note that it is a free force-inject fallback (sub-agent failed / timed out) and the reason. Both outcomes return Ok(String) — the tool never returns Err for a pinned-but-coarse session; Err is reserved for validation failures, missing handler, and unknown-target rejections.
  #   16. The sub-agent's task prompt is built by codelet_cli::compaction_dag::build_generate_compaction_prompt(target_uuid, mode): FRESH mode adapts COMPACTION_INSTRUCTION_FRESH ("Your conversation history" → "Session <target-uuid>") and INCREMENTAL mode adapts COMPACTION_INSTRUCTION_INCREMENTAL with the existing DAG embedded and last_compacted_turn filled; every SessionSearch reference in the prompt tells the sub-agent to pass session_id=<target-uuid> explicitly, and the final step changes from "Call inject_summary(content)" to "Output the complete DAG (all <dag-node> blocks + the <dag-files> section) as your final response; do not call any other tool after." The prompt must NOT contain the string 'inject_summary' (the sub-agent has no such tool).
  #
  # EXAMPLES:
  #   1. A supervisor session asks the agent to compact a subordinate's context. The agent calls GenerateCompaction with the subordinate's session UUID. The tool returns a DAG summary as its result, and the subordinate session's context is now reduced to system reminders plus that DAG — visible to the subordinate's next turn — while the supervisor's own context is untouched. The tool result string is the pinned DAG itself, so the supervisor can see exactly what the subordinate now carries.
  #   2. Mid-turn in session A (no arguments): the agent calls GenerateCompaction to shrink its own context. It gets back a DAG summary as the tool result. From that point on, session A's context is reduced to system reminders plus that DAG — the agent's next turn starts on the reduced context, and the CompactionComplete event reports the real post-compaction context size.
  #   3. The compactor sub-agent stalls past the 600s wall-clock timeout while surveying a very large session. GenerateCompaction still returns a result: a fallback DAG (the partial dag-node blocks recovered from the sub-agent's output, or a generic auto-recovered node if none were emitted), pinned to the target session with zero further LLM cost. The tool result carries a structured note that this is a fallback and why. The target session survives with reduced-but-coarse context instead of staying oversized.
  #   4. The agent calls GenerateCompaction with session_id "not-a-uuid". The tool fails fast with a validation error naming the offending parameter and showing the accepted shape (a UUID string or omitted) — nothing is spawned and no session is touched.
  #   5. Session A was compacted earlier this run (its context carries a compaction DAG ending at turn N). A later GenerateCompaction call on A returns an updated DAG: the durable D2 nodes survive unchanged, the arc D0 nodes are promoted to D1, and only turns after N were surveyed. The pinned result keeps the settled architecture decisions intact.
  #
  # ========================================

  Background: User Story
    As a AI agent (or any session running in codelet)
    I want to invoke the GenerateCompaction tool to build a hierarchical DAG summary of a target session's history in a clean ephemeral sub-agent and have the DAG pinned to that session by the handler side
    So that compaction becomes an explicit on-demand tool call available any time — including on a DIFFERENT session — instead of only the passive error-cascade recovery of CMPCT-044

  # ============================================================================
  # Tool definition and argument validation
  # ============================================================================

  @tools
  @tool-definition
  Scenario: GenerateCompaction implements the rig tool trait
    Given the GenerateCompaction tool struct exists in the rust/tools/src/generate_compaction module
    When the rig agent builder includes GenerateCompactionTool::new(session_id)
    Then GenerateCompaction has NAME = "GenerateCompaction"
    And GenerateCompaction has Args type = GenerateCompactionArgs with session_id (optional string, UUID)
    And GenerateCompaction has Output type = String
    And GenerateCompaction has Error type = ToolError
    And the definition() returns a JSON schema describing the optional session_id parameter with no required fields

  @tools
  @error-handling
  Scenario: Invalid session_id returns a validation error
    Given a GenerateCompaction handler is registered for the calling session
    When the agent calls GenerateCompaction with session_id "not-a-uuid"
    Then the tool returns a ToolError::Validation naming the offending parameter
    And the error includes a usage hint showing the accepted shape (a UUID string or omitted)
    And no sub-agent is spawned and no session is modified

  @tools
  @default
  Scenario: Omitted session_id targets the calling session
    Given the agent is running in session A with a registered GenerateCompaction handler
    When the agent calls GenerateCompaction with no arguments
    Then the target session resolves to session A (the calling session)

  # ============================================================================
  # Handler lookup and target resolution
  # ============================================================================

  @agent-core
  @error-handling
  Scenario: Missing GenerateCompaction handler returns an execution error
    Given no GenerateCompactionHandler is registered for the calling session
    When the agent calls GenerateCompaction
    Then the tool returns a ToolError::Execution with the message pattern "handler not configured for session <uuid>"
    And nothing is spawned and no session is pinned

  @agent-core
  @error-handling
  Scenario: Unknown target session is rejected
    Given the calling session has a registered GenerateCompaction handler
    And the target UUID does not correspond to a live in-memory session
    When the agent calls GenerateCompaction with that target UUID
    Then the tool returns a ToolError::Execution stating the target session is not a live session
    And no sub-agent is spawned and no pinning occurs

  # ============================================================================
  # Compactor sub-agent: spawner contract (DeepSearch clone)
  # ============================================================================

  @agent-core
  @context-management
  @tools
  @integration
  Scenario: Compactor sub-agent runs in an ephemeral clean session
    Given the calling session has a configured provider and model
    When GenerateCompaction is invoked for a live target session
    Then the compactor sub-agent runs under a fresh ephemeral session id that is not the target's or caller's id
    And the sub-agent inherits the calling session's provider, model, context window, and max output tokens
    And no session record is persisted for the sub-agent and no worktree is created

  @context-management
  @tools
  Scenario: Compactor sub-agent has only the read-only tool surface
    Given a compactor sub-agent is constructed
    When its tool list is built
    Then the sub-agent has exactly the seven read-only tools: Read, Grep, AstGrep, Glob, Ls, Bash, and SessionSearch
    And the sub-agent does NOT have the inject_summary tool
    And the sub-agent does NOT have the GenerateCompaction tool
    And the sub-agent does NOT have any Write or Edit tool

  @context-management
  @persistence
  @integration
  Scenario: Compactor sub-agent surveys the target exclusively via SessionSearch on the target session id
    Given a live target session with persisted turns
    When the compactor sub-agent is asked to build the compaction DAG
    Then every SessionSearch call the sub-agent issues targets the target session id, not the sub-agent's own ephemeral session
    And the sub-agent never reads the target's in-memory message list
    And the sub-agent's SessionSearch handler is created before the run and removed by a drop guard after it, even if the run panics

  # ============================================================================
  # Prompt builder: FRESH / INCREMENTAL
  Scenario: Successful sub-agent DAG is pinned to the target session
    Given a live target session with system reminders and many compactable conversation messages
    And the compactor sub-agent returns a parseable DAG with at least one dag-node block
    When the handler side pins the sub-agent's DAG to the target
    Then the target's in-memory messages are replaced by the system reminders plus the wrapped DAG
    And the target's turn list is cleared
    And the target's token tracker is recalculated from the reduced message list
    And the persisted session manifest history is NOT truncated
    And the DAG content is wrapped in the compaction-dag system-reminder wrapper

  @context-management
  @session
  @integration
  Scenario: Calling-session target is pinned by the existing end-of-turn path
    Given the agent calls GenerateCompaction with no arguments on its own session A mid-turn
    And the sub-agent returns a parseable DAG
    When the tool call completes within session A's turn
    Then the tool result is the DAG text
    And the DAG is stashed in session A's pending_dag_content rather than pinned in place
    And the end-of-turn apply_pending_dag path performs the pin, emits CompactionComplete, and clears the flag
    And the handler does not lock session A's inner while the tool call is in flight

  @context-management
  @session
  @integration
  Scenario: Other-session target is pinned immediately under the target's lock
    Given session S calls GenerateCompaction with a live subordinate session T's UUID
    And the sub-agent returns a parseable DAG
    When the handler side completes the run
    Then the pin is applied to T's in-memory messages under T's inner lock
    And T's messages are reduced to system reminders plus the wrapped DAG
    And session S's context is untouched
    And the target's compaction_in_progress flag is false again after the pin

  # ============================================================================
  # Fallback: convergence guarantee when the sub-agent fails
  # ============================================================================

  @context-management
  @session
  @regression
  Scenario: Sub-agent timeout pins a free fallback DAG
    Given a live target session
    And the compactor sub-agent exceeds the 600s wall-clock timeout without returning a DAG
    When the handler side completes compaction
    Then a fallback DAG is pinned to the target session without any further LLM call
    And the fallback DAG is a generic auto-recovered node covering the session's turns
    And the tool result is the fallback DAG text with a structured note that it is a free force-inject fallback and the reason

  @context-management
  @session
  Scenario: Unparseable sub-agent output pins a fallback DAG
    Given the compactor sub-agent returns final text containing no parseable dag-node blocks
    When the handler side completes compaction
    Then a fallback DAG is pinned to the target session
    And the tool result is the fallback DAG text with a structured fallback note

  @context-management
  @session
  Scenario: Partial dag-node blocks from a failed sub-agent are recovered
    Given the compactor sub-agent times out after emitting some complete dag-node blocks but no final DAG
    When the handler side completes compaction
    Then the fallback DAG is assembled from the partial dag-node blocks
    And the target's in-memory context is reduced to reminders plus the recovered DAG

  # ============================================================================
  # Flag discipline and lifecycle events
  # ============================================================================

  @context-management
  @integration
  Scenario: compaction_in_progress gates sub-agent reads and is cleared after the pin
    Given the compactor sub-agent run begins for a live target session
    When the sub-agent runs and reads the target via SessionSearch
    Then the target's compaction_in_progress flag is true during the sub-agent run so Layer-0 trimming applies to its reads
    And the target's compaction_in_progress flag is false after the pin completes

  @context-management
  @session
  @integration
  Scenario: Compaction lifecycle events are emitted on the target session
    Given GenerateCompaction is invoked for a live target session
    When the sub-agent run and pin complete
    Then the target emitted a Compacting state change and a compaction progress update before the sub-agent was spawned
    And the target emitted a Running state change followed by CompactionComplete after the pin
    And the CompactionComplete event reflects the recalculated post-pin token basis, not just the DAG summary size

  # ============================================================================
  # Tool surface and registration lifecycle
  # ============================================================================

  @tools
  @providers
  @integration
  Scenario: GenerateCompaction tool wired into provider agent builders
    Given all provider implementations exist (Claude, OpenAI, Gemini, Codex, ZAI, GitHub Copilot, custom provider)
    When each provider's create_rig_agent() method builds an agent
    Then GenerateCompactionTool::new(session_id) is included in the tool chain beside DeepSearchTool
    And the parent agent can invoke GenerateCompaction like any other tool

  @agent-core
  @context-management
  @integration
  Scenario: Handler registration lifecycle mirrors the DeepSearch pattern
    Given a background session is created in the agent loop
    When the session's handlers are registered
    Then a GenerateCompaction handler is registered for the session id capturing the provider, model, project path, and owning SessionManager
    And the handler is re-registered after model or provider changes
    And the handler is removed for the session id during end-of-turn cleanup

  @tools
  @session
  @integration
  # ============================================================================

  @context-management
  Scenario: Compaction prompt is FRESH when the target has no DAG
    Given a target session that contains no existing compaction DAG
    When the compactor sub-agent's task prompt is built
    Then the FRESH compaction instruction is used
    And the task prompt names the target session id for every SessionSearch call
    And the task prompt instructs the sub-agent to output the complete DAG as its final response
    And the task prompt does not mention inject_summary

  @context-management
  @regression
  Scenario: Compaction prompt is INCREMENTAL when the target already has a DAG
    Given a target session whose context already contains a compaction DAG ending at turn N
    When the compactor sub-agent's task prompt is built
    Then the INCREMENTAL compaction instruction is used with the existing DAG embedded
    And the task prompt tells the sub-agent to preserve D2 nodes, promote D0 to D1, and only survey turns from N+1 onward
    And the existing DAG is captured from the target BEFORE any clear of its messages

  # ============================================================================
  # Pin: handler-side clear-and-pin of the target session
  # ============================================================================

  @context-management
  @session
  @integration
  Scenario: Tool result is the pinned DAG text
    Given the compactor sub-agent returns a parseable DAG
    When GenerateCompaction completes successfully
    Then the tool result string is the DAG text itself so the caller sees exactly what was pinned
    And the tool returns Ok for both success and fallback outcomes, reserving Err for validation failures, missing handler, and unknown-target rejections
