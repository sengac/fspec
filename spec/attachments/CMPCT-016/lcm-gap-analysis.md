# LCM Paper vs. Current Implementation — Detailed Gap Analysis

## Source Materials

- **LCM Paper:** Ehrlich & Blackman, "LCM: Lossless Context Management", Voltropy PBC, Feb 14 2026, https://papers.voltropy.com/LCM
- **CMV Paper:** Santoni, "Contextual Memory Virtualisation", Imperial College London, arXiv:2602.22402, Feb 2026
- **Current implementation:** CMPCT-005 through CMPCT-013 (all done), plus bugs CMPCT-003, CMPCT-014, CMPCT-015, BUG-101
- **AgentManager context references:** `codelet/tools/src/agent_manager/types.rs`, `codelet/napi/src/agent_manager/agent_manager_handler.rs`

---

## Architecture Comparison

### LCM Architecture (from paper)

```
┌─────────────────────────────────────────────────────────────┐
│  Active Context (sent to LLM each turn)                      │
│   ├─ Recent raw messages (fresh tail)                        │
│   └─ Summary nodes (materialized views over older messages)  │
│       ├─ Leaf summaries: direct summary of message spans     │
│       └─ Condensed summaries: summary of summaries (DAG)     │
│           Each has: provenance links → parent messages/nodes  │
│                     file IDs propagated through compaction    │
│                     engine-inserted IDs for lcm_expand        │
└─────────────────┬───────────────────────────────────────────┘
                  │ lcm_grep, lcm_describe, lcm_expand
                  ▼
┌─────────────────────────────────────────────────────────────┐
│  Immutable Store (PostgreSQL-backed, transactional)          │
│   ├─ Messages table: full-fidelity user/assistant/tool       │
│   │   (indexed full-text search for lcm_grep)                │
│   ├─ Summaries table: leaf + condensed, with provenance      │
│   └─ Large Files: path-based, Exploration Summaries          │
└─────────────────────────────────────────────────────────────┘
```

### Current Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  Active Context (session.messages sent to LLM)               │
│   ├─ System reminders (CLAUDE.md, env, fspec workflow)       │
│   ├─ Compaction DAG (free-text markdown in system-reminder)  │
│   │   └─ [SessionSearch: turns X-Y] text breadcrumbs         │
│   └─ Fresh conversation (post-compaction turns)              │
└─────────────────┬───────────────────────────────────────────┘
                  │ SessionSearch (search/show/recent)
                  ▼
┌─────────────────────────────────────────────────────────────┐
│  Persistence Layer (JSONL append-only message store)         │
│   ├─ messages.jsonl: all messages with metadata              │
│   │   (structural annotations in metadata.annotations)       │
│   ├─ BlobStore: large content (>threshold)                   │
│   └─ Session manifests with message references               │
└─────────────────────────────────────────────────────────────┘
```

---

## Gap 1: No Structured References — CRITICAL

### What LCM does (Section 2, Section 2.4)

> "When LCM compacts older messages into summary nodes, the engine deterministically
> inserts the IDs of the summarized content into the active context alongside each
> summary. The engine enforces this *programmatically as a post-processing step*,
> independent of model output."

> "Any message from earlier in the session can always be retrieved losslessly via
> the lcm_expand tool, regardless of how many rounds of compaction have occurred.
> The model never needs to 'know' that compaction happened; it simply sees summary
> text annotated with stable identifiers it can expand on demand."

LCM tools for DAG navigation:
- `lcm_grep(pattern, summary_id?)` — regex search across immutable history, grouped by covering summary node
- `lcm_describe(id)` — metadata for any file or summary (kind, tokens, parent pointers, text)
- `lcm_expand(summary_id)` — expand summary to constituent messages (restricted to sub-agents)

### What we have

The agent writes free-text markdown:
```markdown
### Durable Decisions (D2) — turns 1-120
- Architecture: JWT + Redis + bcrypt for auth
[SessionSearch: turns 1-120 for full history]
```

These `[SessionSearch: turns X-Y]` hints are:
- Not machine-parseable (no structured format)
- Not validated (agent might write wrong turn numbers)
- Not engine-enforced (agent might omit them entirely)
- Not expandable (no tool that takes a reference and resolves it)

### What AgentManager already has

`ContextReference` enum (codelet/tools/src/agent_manager/types.rs):
```rust
pub enum ContextReference {
    Turns { session_id: String, turns: Vec<usize> },
    TurnRange { session_id: String, start_turn: usize, end_turn: usize },
    Query { session_id: String, query: String },
}
```

Resolution at send time (agent_manager_handler.rs):
- `resolve_turns_context()` — loads session from persistence, resolves message content
- `resolve_query_context()` — reuses ripgrep matcher from SessionSearch
- Output format: `<from session="uuid" turns="1-3">content</from>`

This is the exact pattern needed for DAG references.

### Proposed Fix

#### A. Structured DAG node format

Instead of free-text markdown, the compaction instruction tells the agent to write DAG content using structured XML-like blocks:

```xml
<dag-node depth="D2" turns="1-120" label="Architecture Decisions">
- JWT + Redis + bcrypt for auth
- Switched from node-redis to ioredis (connection reliability)
- Using Vitest, not Jest (project standard)
</dag-node>

<dag-node depth="D1" turns="80-120" label="Auth Implementation Arc">
- Completed auth handler (turns 80-100)
- Started rate limiting (turns 101-120)
- Resolved: Redis WRONGTYPE error was wrong data structure
</dag-node>

<dag-node depth="D0" turns="115-120" label="Current State">
- Working on rate-limiting middleware in src/middleware/rateLimit.ts
- Tests written, 2 failing: counter increment race condition
- Last error: Redis WATCH/MULTI pipeline not atomic in cluster mode
</dag-node>

<dag-files>
src/middleware/rateLimit.ts (in progress)
src/__tests__/rateLimit.test.ts (2 failing)
src/auth/login.ts (complete, don't modify)
</dag-files>
```

The `turns` attribute uses the same compact format as AgentManager: `"1-3"` for contiguous ranges, `"1,3,5"` for specific turns.

#### B. Engine-side post-processing

After inject_summary receives the DAG content, the engine can:
1. Parse `<dag-node turns="...">` blocks
2. Validate turn ranges against persisted message count
3. Store the DAG metadata (node depths, turn ranges) as structured data alongside the text
4. Make turn ranges available for future scoped queries

#### C. Expansion via SessionSearch

Add an optional `scope` or `turns` parameter to SessionSearch that restricts results to specific turn ranges. The agent can "expand" a DAG node by calling:

```
SessionSearch(action: "show", start_turn: 80, end_turn: 120)
```

This is simpler than LCM's `lcm_expand` (which requires sub-agent delegation to prevent context flooding) because our SessionSearch already supports `max_turns` limiting. The agent self-regulates how much detail to retrieve.

Alternatively, a dedicated `expand_dag_node` tool could parse the `<dag-node>` block and auto-resolve via the existing `resolve_turns_context()` from AgentManager — reusing that infrastructure directly.

---

## Gap 2: No Incremental Condensation

### What LCM does (Section 2.1, Figure 2)

LCM's context control loop (Figure 2 in paper):
```
1: Persist new item h to Store
2: Append h to Active Context C (as pointer)
3: if Tok(C) > τ_soft then
4:   Trigger asynchronous compaction (non-blocking)
5: if Tok(C) > τ_hard then
6:   while Tok(C) > τ_hard do
7:     Identify oldest block in C
8:     S ← EscalatedSummary(block)
9:     Replace block in C with pointer to S
```

Key: compaction is **incremental**. Only the oldest block is compacted at a time. When enough leaf summaries accumulate, they're condensed into higher-order summaries — building the DAG depth over time.

### What we have

Every compaction is a **full rebuild**. The agent:
1. Receives `/clear` + system instruction
2. Re-reads everything via SessionSearch
3. Writes an entirely new DAG from scratch
4. Calls inject_summary to pin it

The previous DAG (if any) survives as a `<system-reminder type=compaction-dag>` and is visible to the agent during rebuild, but there's no instruction to extend rather than replace it.

### Proposed Fix

Update the compaction system instruction to be DAG-aware:

**First compaction (no existing DAG):** Current behavior — build from scratch.

**Subsequent compactions (existing DAG in context):**
```
Your previous session summary DAG is already in context. DO NOT rebuild it
from scratch. Instead:

1. Your existing D2 (Durable) and D1 (Arc) nodes are still valid — keep them.
2. Promote any D0 (Detailed) nodes from the previous DAG to D1 if they
   describe work that's no longer current.
3. Use SessionSearch to retrieve only the FRESH conversation since the last
   compaction (turns after your most recent D0 node's turn range).
4. Write new D0 nodes summarizing only the fresh conversation.
5. Call inject_summary with the COMPLETE updated DAG (old nodes + new nodes).
```

This means the agent only reads the delta, not the entire history. Compaction cost scales with conversation length since last compaction, not total session length.

---

## Gap 3: No Guaranteed Convergence

### What LCM does (Section 2.3, Figure 3)

Three-Level Summarization Escalation:
```
Level 1 (Normal): LLM-Summarize(X, mode="preserve_details", T)
Level 2 (Aggressive): LLM-Summarize(X, mode="bullet_points", T/2)
Level 3 (Deterministic): DeterministicTruncate(X, 512)  // No LLM
```

If Level 1 output is bigger than input, escalate to Level 2. If Level 2 fails, Level 3 guarantees convergence via deterministic truncation — no LLM involved.

### What we have

No escalation. If the agent writes a DAG that's too large, or gets confused and never calls inject_summary, there's no safety net. CMPCT-011 rule [13]: "No watchdog needed... If the agent fails to call inject_summary, context keeps growing until the emergency threshold fires again."

But the emergency threshold (CMPCT-012) just retriggers the same flow — same system instruction, same agent. This can loop.

### Proposed Fix

**Watchdog timer:** After `execute_compaction()` injects the system instruction, start a turn counter. If the agent hasn't called `inject_summary` within 5 turns:

1. **Escalation Level 2:** Inject a follow-up instruction: "You're taking too long to build the DAG. Write a bullet-point summary ONLY — no detail, no SessionSearch lookups — and call inject_summary immediately."

2. **Escalation Level 3 (deterministic):** If still no inject_summary after 2 more turns, the engine force-injects a deterministic fallback:
   - Take the current in-memory messages (whatever the agent has written during DAG construction)
   - Extract any `<dag-node>` blocks found
   - If none, create a minimal summary: "Session compacted. Use SessionSearch to retrieve history."
   - Force-call the inject_summary handler directly (bypassing the agent)
   - Clear the compaction_in_progress flag

This mirrors LCM's Level 3: guaranteed convergence via deterministic truncation, no LLM needed.

---

## Gap 4: No Scoped Search

### What LCM does (Appendix C.1)

> `lcm_grep(pattern, summary_id?)` — "An optional summary_id parameter restricts
> the search to messages within the scope of a particular summary."

### Proposed Fix

Two approaches (non-exclusive):

**Instruction-level (immediate):** Update the compaction system instruction to tell the agent to use SessionSearch with targeted turn ranges rather than broad searches. The agent already *can* do `SessionSearch(show, start_turn: 80, end_turn: 120)` — it just isn't guided to.

**Engine-level (future):** Add an optional `scope_turns` parameter to SessionSearch search action that restricts regex matching to messages within the specified turn range. Low implementation cost since the persistence layer already loads messages with indices.

---

## Gap 5: No File ID Propagation Through DAG

### What LCM does (Section 2.2)

> "File IDs are propagated through the summary DAG: when messages referencing a
> file are compacted, the resulting summary node retains the file IDs. This ensures
> that even after multiple rounds of compaction, the model retains awareness of,
> and can re-read, any file encountered earlier in the session."

### What we have

FileModification structural annotations track `{path, operation}` per-turn. These are visible in SessionSearch results during DAG construction as `[annotations: FileModification(src/auth.rs → Created)]`. But after inject_summary, they only survive if the agent chose to mention them in its DAG text.

### Proposed Fix

**Engine-side:** After inject_summary, the engine scans all structural annotations from the compacted turns, extracts the unique file paths from FileModification annotations, and appends a `<dag-files>` block to the DAG content if the agent didn't include one. This is the same "engine post-processing" principle as LCM's ID insertion.

**Instruction-side:** The compaction system instruction already suggests an "Active Files" section. Make it mandatory and explicitly reference the annotations.

---

## Gap 6: Soft/Hard Threshold

### What LCM does (Section 2.4)

Two thresholds with different behaviors:
- **τ_soft:** Triggers async compaction. Non-blocking — user continues interacting while compaction runs in background. Zero user-facing latency.
- **τ_hard:** Blocks until compaction completes. Only fires if context grows during the ~25s async compaction window.

### What we have

Single threshold. When hit, `execute_compaction()` clears context and injects compaction instruction. The agent must build the DAG before it can continue the user's work. Always blocking.

### Proposed Fix

**τ_soft (early warning):** When context hits ~70% capacity, inject a subtle system annotation (not a full compaction trigger) telling the agent: "Context is getting full. Consider wrapping up your current logical unit of work. Compaction will trigger automatically at 85%."

This gives the agent the opportunity to finish what it's doing, produce a natural checkpoint (like completing a test or committing), before the hard threshold interrupts.

**τ_hard:** Current behavior — force compaction at 85-90%.

---

## Summary: Implementation Priority

| Priority | Gap | Impact | Effort | Description |
|----------|-----|--------|--------|-------------|
| **P0** | Gap 1 | Critical | 8 pts | Structured DAG references with `<dag-node turns="...">` blocks, reusing AgentManager resolution infrastructure |
| **P0** | Gap 3 | Critical | 5 pts | Convergence guarantee: watchdog + escalation + deterministic fallback |
| **P1** | Gap 2 | High | 5 pts | Incremental condensation: extend existing DAG rather than rebuild |
| **P1** | Gap 5 | Medium | 3 pts | File tracking: engine-side extraction from structural annotations |
| **P2** | Gap 4 | Low | 2 pts | Scoped search: instruction-level initially, engine support later |
| **P2** | Gap 6 | Low | 3 pts | Soft threshold early warning |
