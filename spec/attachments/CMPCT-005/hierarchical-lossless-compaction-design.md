# Hierarchical Lossless Context Compaction with In-View DAG Construction

## Research Foundation

This design synthesizes insights from seven papers published 2025–2026:

| Paper | Citation | Key Contribution |
|---|---|---|
| **CMV** | Santoni, Imperial College London, arXiv:2602.22402, Feb 2026 | DAG-based state management + structurally lossless three-pass trimming (20–86% reduction) |
| **LCM** | Ehrlich & Blackman, Voltropy PBC, Feb 2026 | Hierarchical summary DAG with depth-aware prompts, bounded sub-agent expansion, tools for DAG navigation |
| **Focus** | Verma, arXiv:2601.07190, Jan 2026 | Agent-autonomous compression via start_focus/complete_focus; 22.7% token reduction, zero accuracy loss |
| **ACON** | Kang et al., KAIST/Microsoft, arXiv:2510.00615, Oct 2025 | Compression guideline optimization via failure analysis; 26–54% peak token reduction, 95%+ accuracy preserved after distillation |
| **HiAgent** | Hu et al., ACL 2025 (aclanthology.org/2025.acl-long.1575) | Subgoal-based hierarchical working memory management; 2× success rate, 3.8 fewer steps |
| **SimpleMem** | Liu et al., ICLR 2026 Workshop LLA, arXiv:2601.02553 | Three-stage semantic compression + online synthesis + intent-aware retrieval; 26.4% F1 improvement, 30× token reduction |
| **H-MEM** | Sun & Zeng, arXiv:2507.22925, Jul 2025 | Multi-level semantic abstraction with positional index routing for efficient layer-by-layer retrieval |

Additionally, the survey paper "Memory in the Age of AI Agents" (Hu et al., arXiv:2512.13564, Dec 2025) provides a comprehensive taxonomy that positions this work within the broader agent memory landscape.

---

## Problem Statement

The current compaction system (`codelet/core/src/compaction/`) has three critical weaknesses:

1. **Expensive batch LLM calls.** `AnchorDetector::detect_batch()` sends ALL conversation turns to the LLM in a single prompt. For a 50-turn session: potentially 50k+ tokens in the analysis prompt alone, with a timeout of `15s + 2s × num_turns`. This happens on every compaction trigger.

2. **Lossy flat summarization.** After detection, `ContextCompactor::compact()` generates a single narrative LLM summary (`generate_llm_summary()`) that replaces all compacted turns. The summary is prose — not searchable, not incrementally updatable, not expandable back to source material. Key decisions, error context, and subtle reasoning are permanently lost.

3. **Flaky heuristics in PreservationContext.** `detect_build_status()` matches substrings "pass"/"fail" against arbitrary tool output (false positives on "password", "bypass", `DB_PASS`). `extract_goal_from_message()` matches action verbs like "build", "remove", "please" in user messages. These produce silent misclassifications.

### What CMPCT-004 Proposed (and Why It's Insufficient)

CMPCT-004 proposed a three-tier hot/warm/cold architecture with zero LLM cost. Its warm tier is a flat Markdown document (~825 tokens) containing only structural facts extracted from tool call metadata. The research unanimously contradicts this approach:

- **Flat structure is insufficient.** H-MEM and LCM both demonstrate hierarchical memory organization is a *necessary condition* for robust long-term reasoning, not an optional refinement.
- **Excluding semantic content degrades quality.** ACON shows that preserving WHY decisions were made (not just WHAT tool was called) is critical for continuation quality. SimpleMem achieves 26.4% F1 improvement specifically because it preserves semantic density.
- **Agent passivity reduces effectiveness.** Focus and HiAgent show that agents that control their own compression outperform agents that are compressed externally.

---

## Key Design Insight: In-View DAG Construction via /clear + SessionSearch

Instead of running compaction in a background process with separate LLM calls, **the agent itself builds the DAG summary in the same conversation view**, using existing tools.

### Why This Is Better

1. **Zero marginal LLM cost.** The agent is already running. It can summarize its own history as part of the conversation — no separate summarization API calls. The cost of the DAG construction is embedded in the normal turn cost.

2. **The agent is the best possible summarizer of its own work.** An external summarization call gets a canned prompt like "summarize these 5 turns." The agent doing it in-context knows *why* those turns mattered, what's still relevant, what was a dead end. It has full judgment.

3. **SessionSearch already exists.** No new retrieval tools needed. The agent can do targeted searches (regex, time-filtered), retrieve specific turns, and build its summary selectively — far more powerful than a rigid cohort-based pipeline.

4. **No background infrastructure.** No async workers, no background LLM call management, no race conditions between the background summarizer and the active conversation. Everything happens in one linear flow.

### Confirmed: /clear Preserves Messages for SessionSearch

**Verified in code.** `clear_history()` (line 1574 of `session_manager.rs`) only clears in-memory state:
- `inner.messages.clear()` — clears the in-memory message array
- `inner.turns.clear()` — clears the in-memory turn tracker
- `inner.token_tracker = TokenTracker::default()` — resets token counts
- Then reinjects system reminders (CLAUDE.md, environment info)

It **never** touches the on-disk persistence layer. Messages are written to disk as they stream via `append_message_with_metadata()` into a JSONL store (`messages/messages.jsonl`). The persistence layer has no message deletion path at all.

`SessionSearch`'s `handle_show()` (line 268 of `session_search_handler.rs`) explicitly loads from persistence:
```rust
// Load all messages (ignoring compaction — we want full history for show)
let messages = match get_session_messages_full(&session) { ... }
```

And `handle_search()` (line 120) also reads from the on-disk store, not in-memory state. **All pre-clear messages remain fully searchable and retrievable.**

---

## Architecture: Three-Layer Compaction

```
┌────────────────────────────────────────────────────────┐
│                 ACTIVE CONTEXT WINDOW                    │
│                                                          │
│  ┌────────────────────────────────────────────────────┐ │
│  │ System Reminders (stable prefix, cacheable)         │ │
│  │ CLAUDE.md, environment, fspec workflow              │ │
│  └────────────────────────────────────────────────────┘ │
│                                                          │
│  ┌────────────────────────────────────────────────────┐ │
│  │ Injected DAG Summary (structured, hierarchical)     │ │
│  │  D2: durable narrative (decisions still in effect)  │ │
│  │  D1: arc summaries (what evolved, outcomes)         │ │
│  │  D0: detailed summaries (decisions, file changes)   │ │
│  │  Each level: turn range references for SessionSearch│ │
│  └────────────────────────────────────────────────────┘ │
│                                                          │
│  ┌────────────────────────────────────────────────────┐ │
│  │ Fresh conversation (post-compaction turns)           │ │
│  │  Fully verbatim, no trimming needed                 │ │
│  └────────────────────────────────────────────────────┘ │
│                                                          │
└────────────────────────────────────────────────────────┘
                         │
                         │ SessionSearch (search/show actions)
                         ▼
┌────────────────────────────────────────────────────────┐
│ Persistence Layer (all original messages on disk)       │
│  - JSONL message store (append-only, never deleted)     │
│  - Session manifests with message references            │
│  - Blob store for large content                         │
│  - Full tool outputs preserved for re-read              │
└────────────────────────────────────────────────────────┘
```

### Layer 0: Structurally Lossless Trimming (Pre-Clear Optimization)

**Inspired by:** CMV (Santoni 2026), Deep Agents SDK (LangChain)

**What:** Before the agent begins DAG construction, automatically strip mechanical bloat from the conversation history that SessionSearch will return. This is pure data transformation — no LLM call, zero cost, fully reversible from persisted originals.

**Trimming rules:**

| Content Type | Trim Action | Replacement |
|---|---|---|
| Read tool results (file content) | Replace with reference | `[file: {path}, {lines} lines, {tokens} tok — use Read to retrieve]` |
| Write tool parameters (full file content) | Replace with reference | `[Write: {path}, {lines} lines — file persisted to disk]` |
| Edit tool parameters (old_string/new_string) | Condense | `[Edit: {path} — replaced {old_len} chars with {new_len} chars]` |
| Bash tool output (stdout/stderr) | Truncate to head+tail | First 10 lines + `... ({N} lines omitted)` + last 5 lines + exit code |
| Base64 image data | Strip entirely | `[image: {W}x{H}, {bytes} bytes, from {path}]` |
| Search/Grep tool output | Truncate matches | First 10 matches + `... ({N} more matches)` |

**Token impact:** CMV demonstrates 20% mean reduction (up to 86% for sessions with significant tool output overhead) from trimming alone. For tool-heavy coding sessions with Read/Write/Bash output, the reduction should be in the 40-70% range.

**Key constraint:** Trimming NEVER touches user messages or assistant reasoning text. Only structured tool parameters and tool result outputs are trimmed.

**Where trimming happens:** Applied to the results that SessionSearch returns to the agent during DAG construction. The agent sees trimmed content, which reduces the context consumed during the rebuild phase. The on-disk originals remain untouched.

**Integration point — SessionSearch handler:**

The trimming is integrated into `session_search_handler.rs` by adding a `trimmed`
mode that transforms `StoredMessage` content before returning it as `SessionMessage`.
Two implementation options, with Option A recommended:

**Option A — Implicit trimming via session state flag (recommended):**

When the compaction flow starts (after `/clear`, before DAG construction), the
`BackgroundSession` sets a `compaction_in_progress: AtomicBool` flag. The
`SessionSearchHandler` closure (created in `create_handler()` at
`session_search_handler.rs:35`) already captures the session's project path —
it can be extended to also check this flag. When the flag is set, the handler
applies trimming to all `resolve_message_content()` results before returning
them via `handle_show()` and `handle_search()`.

This is transparent to the agent — it calls SessionSearch normally and gets
trimmed content automatically during DAG construction. After `inject_summary`
completes, the flag is cleared, and subsequent SessionSearch calls return
full content.

The flag lives on `BackgroundSession` (alongside existing fields like
`compaction_progress: RwLock<Option<CompactionProgress>>`). The handler
closure needs a reference to read it — the simplest approach is to pass
an `Arc<AtomicBool>` into `create_handler()` alongside the existing
`project_path: PathBuf`.

**Option B — Explicit parameter on SessionSearch actions:**

Add an optional `trimmed: Option<bool>` parameter to `SessionSearchAction::Show`
and `SessionSearchAction::Search` in `types.rs`. The agent (or the compaction
system instruction) would pass `trimmed: true` during DAG construction.

This is more explicit but requires the agent to remember to pass the flag,
and adds a parameter that's only useful during compaction.

**Trimming implementation:**

The trimming function itself takes a `StoredMessage` (which has `role: String`,
`content: String`, and `metadata: HashMap<String, Value>`) and returns a
transformed content string. The metadata field already contains tool information
for assistant messages (tool name, input parameters) via `AssistantContent::ToolUse`
serialization. For tool result messages, the content field contains the raw output.

The trimmer inspects `metadata` to identify the tool type and applies the
appropriate rule from the trimming table above. Messages without tool metadata
(plain user/assistant text) pass through unchanged.

**Implementation location:** New module `codelet/core/src/compaction/trimmer.rs`

### Layer 1: In-View DAG Construction (Agent-Driven)

**Inspired by:** LCM (Ehrlich & Blackman 2026), Focus (Verma 2026), HiAgent (Hu et al. 2025), H-MEM (Sun & Zeng 2025)

**What:** When compaction is triggered (either by threshold or manually via `/compact`), the agent builds a hierarchical summary DAG in-view using `/clear` + `SessionSearch` + a new `inject_summary` tool. The agent IS the summarizer — no separate LLM calls.

**The Compaction Flow:**

```
Step 1: /clear
  → In-memory context cleared (system reminders reinjected)
  → All messages remain on disk, fully searchable via SessionSearch
  → Full context budget available for DAG construction

Step 2: Agent retrieves previous history via SessionSearch
  → Targeted searches, not linear retrieval
  → SessionSearch returns trimmed content (Layer 0 applied)
  → Agent reads strategically across multiple calls

Step 3: Agent writes hierarchical DAG summary
  → D0 nodes: detailed (exact files, decisions, errors, rationale)
  → D1 nodes: arc-level (what was attempted, outcomes, current state)
  → D2 nodes: durable narrative (architecture decisions, milestones)
  → Turn range references on each node for future SessionSearch drilldown

Step 4: Agent calls inject_summary(content)
  → DAG summary injected as pinned system-level content
  → All builder turns (Steps 2-3) are dropped from context
  → Agent continues with: system reminders + DAG + clean slate
```

**Why the agent builds a better DAG than an automated pipeline:**

- The agent knows which decisions are still in effect vs. reversed
- The agent knows what's a dead-end exploration vs. important context
- The agent can prioritize current work state over historical details
- The agent adapts its summary depth to what it actually needs going forward

**DAG Structure (Written as structured text by the agent):**

```markdown
## Session Summary (DAG)

### Durable Decisions (D2) — turns 1-120
- Architecture: JWT + Redis + bcrypt for auth
- Switched from node-redis to ioredis (connection reliability)
- Using Vitest, not Jest (project standard)
[SessionSearch: turns 1-120 for full history]

### Recent Arc (D1) — turns 80-120
- Completed auth implementation (turns 80-100)
- Started rate limiting work (turns 101-120)
- Key blocker resolved: Redis WRONGTYPE error was wrong data structure
[SessionSearch: turns 80-120 for arc details]

### Current State (D0) — turns 115-120
- Working on rate-limiting middleware in src/middleware/rateLimit.ts
- Tests written but 2 failing: counter increment race condition
- Last error: Redis WATCH/MULTI pipeline not atomic in cluster mode
- Files in progress: rateLimit.ts, rateLimit.test.ts
[SessionSearch: turns 115-120 for exact content]

### Active Files
- src/middleware/rateLimit.ts (in progress)
- src/__tests__/rateLimit.test.ts (2 failing)
- src/auth/login.ts (complete, don't modify)
```

**Agent search strategy during Step 2:**

The agent doesn't retrieve everything linearly. It searches strategically:

```
Call 1: SessionSearch(action: "show", session_id: "current", max_turns: 10)
  → Gets the last 10 turns (most recent, highest value)

Call 2: SessionSearch(action: "search", query: "error|failed|fix")
  → Gets all error resolutions across the session

Call 3: SessionSearch(action: "search", query: "decision|chose|switched|architecture")
  → Gets all decision points

Call 4: SessionSearch(action: "search", query: "TODO|blocker|question")
  → Gets open items and blockers

Agent synthesizes results into DAG, calls inject_summary.
```

**Context budget during rebuild:** After `/clear`, the agent has a full context window. But the retrieval is selective — typically 3-5 SessionSearch calls retrieving perhaps 20-30 turns of content (with trimming applied). This easily fits within budget even for the smallest context windows.

**New tool required: `inject_summary`**

```
Tool: inject_summary

Parameters:
  content: string  — The DAG summary content to inject

Behavior:
  1. Stores the content as a pinned system-level message at the top of context
     (after system reminders, before any conversation turns)
  2. Deletes ALL conversation turns that occurred after the last /clear
     (the SessionSearch calls and agent reasoning during DAG construction)
  3. Agent's next turn starts fresh with: system reminders + injected DAG
  4. Returns: injected token count, available context budget remaining
```

**Precise mechanism for "drop builder turns":**

After `/clear`, the in-memory `session.messages` Vec contains only system reminders
(injected by `inject_context_reminders_with_isolation()` inside `clear_history()`).
As the agent makes SessionSearch calls and reasons about the DAG, new messages
accumulate in `session.messages` (user prompt, assistant response with tool calls,
tool results, etc.). These are the "builder turns."

When the agent calls `inject_summary(content)`, the tool implementation:

1. **Partitions** `session.messages` using the existing `partition_for_compaction()`
   function from `codelet_cli::session::system_reminders` — this cleanly separates
   system reminder messages (identified by `<system-reminder>` XML tags in
   `UserContent::Text`) from all other messages.

2. **Clears** `session.messages` entirely.

3. **Restores** only the system reminder messages (from step 1).

4. **Appends** the DAG content as a new `Message::User` with a distinguishing
   `<system-reminder>` wrapper (type `compaction-dag`) so it's treated as part
   of the stable prefix for prompt caching and preserved through future
   compactions:
   ```
   <system-reminder>
   <!-- type:compaction-dag -->
   {content}
   </system-reminder>
   ```

5. **Resets** `session.turns` to empty and `session.token_tracker` display
   values to reflect the new context size.

6. **Returns** `{ injected_tokens: u64, remaining_budget: u64 }` to the agent.

This is essentially the same pattern as `execute_compaction()` in
`interactive_helpers.rs` (lines 242-278) which already does:
clear messages → restore system reminders → append summary → append continuation.
The difference is that `inject_summary` replaces the LLM-generated summary with
the agent-written DAG, and skips the kept-turns reconstruction (there are no
kept turns — the DAG IS the kept context).

**Persistence note:** The builder turns (SessionSearch calls, agent reasoning)
ARE persisted to disk as they stream — `persist_user_message()` and
`persist_assistant_message()` in `session_manager.rs` fire during normal
agent execution regardless of what happens to in-memory state afterwards.
This means the DAG construction process itself is fully recoverable via
SessionSearch if needed, even though it's dropped from the active context.

**Implementation location:** New tool in `codelet/tools/src/inject_summary.rs` with
NAPI handler in `codelet/napi/src/inject_summary_handler.rs`. The tool follows
the same handler pattern as `SessionSearchTool` and `FspecTool` — the tool
definition and schema live in `codelet-tools`, the actual session manipulation
is registered via a handler closure from `codelet-napi` that has access to the
`BackgroundSession`'s inner `Session`.

### Layer 2: Emergency Threshold Compaction

**What:** The existing `CompactionHook` threshold trigger remains as a safety net. When context hits 85-90% capacity and the agent hasn't proactively managed its context, the system forces the in-view DAG construction flow:

1. Auto-trigger `/clear` (same as manual)
2. Inject a system prompt instructing the agent to build its DAG summary NOW
3. Agent uses SessionSearch to retrieve what it needs
4. Agent calls `inject_summary` with its DAG
5. Agent continues working

This is the only time compaction happens without explicit agent initiation. It's a garbage collector — necessary but not the primary mechanism.

**Minimum preserved context:** System reminders + injected DAG summary + last user message (so the agent knows what to do next). Never drops below this floor.

**Implementation:** Modifies the existing `execute_compaction()` in `codelet/cli/src/interactive_helpers.rs` to trigger the in-view flow instead of the batch LLM approach.

---

## Structural Annotations (What We Keep From CMPCT-004)

CMPCT-004's per-turn structural signal detection is correct in principle. We keep it, but use it as metadata that the agent can reference during DAG construction rather than as the sole content of a flat warm tier.

```rust
enum StructuralAnnotation {
    /// fspec milestone reached (from Fspec tool call with specific command)
    FspecMilestone {
        command: String,     // e.g., "update-work-unit-status"
        args: Vec<String>,   // e.g., ["AUTH-001", "implementing"]
    },
    /// Error was resolved (previous failure + current file modification + all-success)
    ErrorResolution {
        failed_tool: String,
        resolved_file: String,
    },
    /// File modification (from Edit/Write tool calls)
    FileModification {
        path: String,
        operation: FileOp, // Created, Modified, Deleted
    },
}
```

These annotations are:
- Detected per-turn inline (not batch), at zero cost
- Stored on the persisted messages as metadata
- Available via SessionSearch when the agent is building its DAG
- The agent can use them to identify important turns without reading full content

---

## Token Budget Analysis

### Post-Compaction Context Window Layout

| Layer | Tokens | Notes |
|---|---|---|
| System reminders | ~3,000 | Unchanged from current |
| Injected DAG summary | ~1,500–3,000 | Agent-written, naturally concise, structured |
| Fresh conversation | unlimited | Agent starts fresh, uses full remaining budget |
| **Total overhead** | **~4,500–6,000** | Minimal fixed cost, rest is usable context |

### Cost Per Compaction Cycle

| System | LLM Calls | Token Input | Latency |
|---|---|---|---|
| **Current** | 2 (batch anchor + summary) | ~50k+ (all turns) + ~10k (summary prompt) | 10–30 seconds |
| **CMPCT-004** | 0 | 0 | <100ms |
| **This design** | 0 separate calls | ~5k-10k (SessionSearch results consumed in-view) | 1-2 agent turns |

The cost is zero marginal LLM cost — the DAG construction happens within the agent's normal turn budget. The only "cost" is 1-2 agent turns spent on retrieval and summarization, which replaces the 10-30 second background compaction pause.

---

## What Changes In Existing Code

### Files To Create (New)

| File | Purpose |
|---|---|
| `codelet/core/src/compaction/trimmer.rs` | Layer 0: Structurally lossless trimming of tool outputs in SessionSearch results. Takes `StoredMessage` content + metadata, returns trimmed content string. |
| `codelet/tools/src/inject_summary.rs` | New `inject_summary` tool: Rig Tool definition, schema, and handler dispatch (follows SessionSearchTool/FspecTool handler pattern). |
| `codelet/napi/src/inject_summary_handler.rs` | NAPI handler for inject_summary tool: closure that captures `BackgroundSession` inner `Session`, performs partition → clear → restore → inject DAG. |

### Files To Modify (Existing)

#### 1. `codelet/core/src/compaction/mod.rs`
**Current:** Re-exports `AnchorDetector`, `ContextCompactor`, `TurnSelector`, `PreservationContext`
**Change:** Add re-export for `Trimmer`. Keep old exports behind `#[deprecated]` during migration.

#### 2. `codelet/core/src/compaction/model.rs`
**Current:** Defines `ConversationTurn`, `ToolCall`, `ToolResult`, `PreservationContext`, `BuildStatus`, `TokenTracker`
**Change:**
- Add `StructuralAnnotation` enum
- `PreservationContext` and `BuildStatus` become `#[deprecated]` — subsumed by agent judgment
- `TokenTracker` stays unchanged (tracks API-level token usage, orthogonal to compaction strategy)

#### 3. `codelet/core/src/compaction/anchor.rs`
**Current:** 486 lines. `AnchorDetector` with LLM-based batch detection.
**Change:**
- Delete all LLM-based detection: `detect_batch()`, `build_anchor_analysis_prompt()`, `build_batch_anchor_analysis_prompt()`, `parse_llm_anchor_response()`, `parse_batch_llm_anchor_response()`, `extract_json_from_response()`, `LlmAnchorResponse`, `LLM_TIMEOUT_SECS`
- Keep structural detection for annotations (per-turn, inline, zero-cost)
- `AnchorType`: Remove `TaskCompletion`, `UserCheckpoint`. Keep `ErrorResolution`, `FeatureMilestone`.

#### 4. `codelet/core/src/compaction/compactor.rs`
**Current:** 442 lines. `ContextCompactor` orchestrates LLM calls for anchor detection + summary generation.
**Change:** This file becomes a thin coordinator for the in-view flow:
- **Delete:** `generate_llm_summary()`, `build_summary_prompt()`, `build_state_checkpoint_prompt()`, `RETRY_DELAYS_MS`, `FALLBACK_SUMMARY`
- **New:** `compact()` triggers `/clear`, sets up the agent instruction to build DAG via SessionSearch, awaits `inject_summary` call

#### 5. `codelet/core/src/compaction/selector.rs`
**Current:** 123 lines. `TurnSelector` splits turns into kept/summarized based on anchor position.
**Change:** **Delete entirely.** The selector's binary keep/summarize split is replaced by the agent's strategic retrieval via SessionSearch.

#### 6. `codelet/cli/src/interactive_helpers.rs`
**Current:** `execute_compaction()` orchestrates the full pipeline.
**Change:** `execute_compaction()` is rewritten to:
1. Trigger `/clear` (clears in-memory context, preserves persistence)
2. Inject a system instruction telling the agent to build its DAG summary using SessionSearch
3. Wait for the agent to call `inject_summary`
4. Return compaction metrics

The function signature stays the same but the internals change completely.

#### 7. `codelet/napi/src/session_manager.rs`
**Current:** `BackgroundSession` has `anchor_points`, `compaction_progress`. `session_compact()` calls `execute_compaction()`.
**Change:**
- `session_compact()` triggers the in-view flow instead of background LLM calls
- Anchor points derived from structural annotations on persisted messages
- Add NAPI binding for `inject_summary` tool (register handler in agent loop setup, alongside existing `set_session_search_handler()` registration at line 5365)
- `compaction_progress` simplified (no multi-phase LLM progress to track)
- Add `compaction_in_progress: Arc<AtomicBool>` field to `BackgroundSession` for Layer 0 trimming integration (set after `/clear`, cleared after `inject_summary`)

#### 8. `codelet/napi/src/session_search_handler.rs`
**Current:** Handles recent/search/show actions against persistence. `create_handler()` captures `project_path: PathBuf` and returns a `SessionSearchHandler` closure.
**Change:** Extend `create_handler()` to also accept an `Arc<AtomicBool>` for the compaction-in-progress flag. When the flag is set, apply Layer 0 trimming to message content returned by `resolve_message_content()` before building `SessionMessage` results in `handle_show()` and `SearchMatch` results in `handle_search()`. The trimming function is imported from `codelet_core::compaction::trimmer`.

#### 9. `codelet/cli/src/interactive/stream_loop.rs`
**Current:** Post-stream-loop logic checks `compaction_needed` flag and calls `execute_compaction()`.
**Change:**
- After each turn, run structural annotation detection (inline, zero cost)
- When `compaction_needed`, trigger the in-view DAG construction flow
- Add handling for `inject_summary` tool call

### Files That Stay Unchanged

| File | Why |
|---|---|
| `codelet/core/src/compaction/metrics.rs` | `CompactionMetrics` and `CompactionResult` are generic enough for new pipeline |
| `codelet/core/src/compaction_hook.rs` | Threshold detection is orthogonal to compaction strategy |
| `src/tui/types/anchor.ts` | Anchor display types stay the same |
| `src/tui/utils/anchorUtils.ts` | Anchor formatting utilities stay the same |

### Files To Delete (After Migration)

| File | Why |
|---|---|
| `codelet/core/src/compaction/selector.rs` | Turn selection replaced by agent's strategic SessionSearch |
| `codelet/core/src/compaction/__tests__/llm_anchor_detection.test.rs` | LLM-based anchor detection tests replaced by structural detection tests |

---

## Migration Strategy

### Phase 1: Trimmer + Inject Summary Tool

**Scope:** Create `trimmer.rs` with the trimming rules. Create `inject_summary` tool with NAPI bindings. These are standalone additions — nothing existing changes.

**Validation:** Unit tests for trimming rules. Integration test for inject_summary (inject content, verify builder turns removed, verify DAG pinned). Measure token reduction from trimming across real SessionSearch output.

**Risk:** Zero. Pure additions, no existing behavior changes.

### Phase 2: In-View Compaction Flow

**Scope:** Rewrite `execute_compaction()` to trigger the in-view flow: `/clear` → agent retrieves via SessionSearch → agent calls `inject_summary`. Wire up the system instruction that tells the agent what to do. Apply trimming to SessionSearch results during DAG construction.

**Validation:** Run on 5+ real sessions. Compare: (a) post-compaction context size, (b) continuation quality (does the agent maintain context?), (c) wall-clock time, (d) cost (should be zero marginal).

**Risk:** Medium. This is the core replacement. The agent's ability to produce good DAG summaries depends on the system instruction quality. Run in shadow mode initially (old pipeline runs, new pipeline's DAG is logged but not used).

### Phase 3: Cleanup

**Scope:** Remove `selector.rs`, `PreservationContext`, `BuildStatus`, all LLM anchor detection functions, `synthetic_checkpoint()`, `RETRY_DELAYS_MS`, `FALLBACK_SUMMARY`. Remove `#[deprecated]` attributes.

**Validation:** Full test suite passes. All existing compaction tests replaced with new tests.

---

## Comparison: This Design vs. Flat Anchored Summarization

The anchored summarization approach (as used by Factory.AI Droid and similar systems) uses a single-layer architecture: detect anchor → summarize everything before it → discard originals. This design is fundamentally different:

| Dimension | Flat Anchored | This Design |
|---|---|---|
| Summarizer | Separate LLM call with canned prompt | The agent itself, with full context and judgment |
| Summary structure | Flat prose paragraph | Hierarchical DAG (D0/D1/D2) with turn references |
| LLM cost | Dedicated summarization calls (~30k-50k input) | Zero marginal (embedded in agent's normal turn) |
| Retrievability | Gone forever once summarized | SessionSearch to drill into any past turn |
| State preservation | Hardcoded struct (todo, agent_md, etc.) | Agent decides what matters, extensible annotations |
| Agent autonomy | None — compression happens TO the agent | Full — agent IS the compressor |
| Background infra | Async workers, race conditions | None — everything in one linear flow |
| Tool outputs | Summarized by LLM (expensive) | Trimmed deterministically (free), originals on disk |
| Failure mode | Bad summary → context permanently lost | Bad DAG → SessionSearch still has everything |

---

## Open Questions

1. **What should the system instruction say during DAG construction?** The agent needs clear guidance on: (a) use SessionSearch strategically, not linearly, (b) write structured DAG with depth levels, (c) include turn range references for future drilldown, (d) call inject_summary when done. This instruction needs to be concise (it consumes context during the rebuild).

2. **Should inject_summary support multiple calls?** The agent might want to build the DAG incrementally — inject D2 first, then D1, then D0. Or it might be simpler to require a single call with the complete DAG. Recommendation: single call initially, iterate based on experience.

3. **What's the right trigger for compaction?** Options: (a) existing 85-90% threshold, (b) agent proactively runs `/compact` when it notices context pressure, (c) both. Recommendation: both — agent can compact proactively, threshold is safety net.

4. **How does this interact with PROV-009 (Server-Side Compaction)?** Anthropic's server-side compaction API could be used as an alternative to the in-view flow — the server generates the summary. The DAG structure would be similar but the construction mechanism differs. These could coexist: server-side for speed, in-view for quality.

### Resolved Questions

5. **~~Should trimming be applied to all SessionSearch results or only during DAG construction?~~** **RESOLVED:** Trimming is applied automatically during DAG construction only, via an implicit `compaction_in_progress` flag on `BackgroundSession`. The flag is set after `/clear` and cleared after `inject_summary`. Normal SessionSearch calls outside compaction return full content. See "Integration point — SessionSearch handler" section above for details.

6. **~~How does inject_summary "drop builder turns" precisely?~~** **RESOLVED:** `inject_summary` uses the same `partition_for_compaction()` → clear → restore pattern as `execute_compaction()`. Builder turns are dropped from in-memory `session.messages` but remain persisted to disk (written during streaming by `persist_user_message()` and `persist_assistant_message()`). See "Precise mechanism for drop builder turns" section above for details.

7. **~~What is SimpleMem's venue?~~** **RESOLVED:** SimpleMem was published at ICLR 2026 Workshop LLA (Lifelong Learning Agents) as a Poster, and also at ICLR 2026 Workshop MemAgents and Workshop RSI Spotlight (all March 2026). NOT ICML 2026 as previously stated. Corrected in research-papers.md.
