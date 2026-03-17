# RLM-002: Making DeepSearch Truly Recursive — Gap Analysis & Plan

**Date:** 2026-03-17
**Work Unit:** RLM-002
**Based on:** RLM Paper (MIT CSAIL, arXiv:2512.24601v2), RLM source code (github.com/alexzhang13/rlm)
**Depends on:** RLM-001 (DeepSearch MVP, done)

---

## 1. The Problem

DeepSearch (RLM-001) was inspired by the RLM paper but is not actually recursive.
The sub-agent's tool set explicitly excludes DeepSearch — it cannot call itself.
This means it cannot do divide-and-conquer over large corpora, which is the
paper's core contribution.

The system prompt is also a generic "research assistant" prompt rather than the
RLM paper's decomposition-oriented prompt that teaches chunking, delegation, and
aggregation via sub-calls.

---

## 2. What the Paper Does (Algorithm 1)

The RLM paper defines three layers:

| Layer | Function | Code equivalent |
|-------|----------|-----------------|
| Root LM (depth=0) | Interacts with REPL, writes code to explore context | The DeepSearch sub-agent |
| `llm_query()` (depth=1) | Plain single-shot LLM call from within code — no tools, just text in/out | A DeepSearch call with a trivial query and no scope |
| `rlm_query()` (depth=1+) | Full recursive RLM — child gets its own REPL and can iterate | A DeepSearch call with scope and tools |

**Key insight: `llm_query()` and `rlm_query()` are both just DeepSearch at different
complexity levels.** A DeepSearch call with a simple question and no scope degenerates
to a single LLM round-trip (the sub-agent answers immediately without tool calls).
A DeepSearch call with scope spawns a full iterative sub-agent. One tool covers both.

The paper's REPL environment provides:
- `context` variable (the loaded prompt)
- `llm_query(prompt)` — single LLM call
- `llm_query_batched(prompts)` — concurrent LLM calls
- `rlm_query(prompt)` — recursive RLM sub-call
- `print()` — observe truncated output
- `FINAL(answer)` / `FINAL_VAR(var)` — return final answer

In our architecture:
- `context` = files on disk (the filesystem IS the environment)
- `llm_query()` = DeepSearch with trivial query, no scope
- `rlm_query()` = DeepSearch with scope
- `print()` = Bash tool (`python3 -c "..."`, shell pipelines, etc.)
- `FINAL()` = rig's natural termination (LLM stops calling tools → returns text)
- Batching = LLM can emit multiple tool calls per turn (rig handles them)

---

## 3. Two Gaps to Close

### Gap 1: Self-Recursion (P0, LOW effort)

**Current state:** The sub-agent gets 7 tools: Read, Grep, AstGrep, Glob, Ls,
Bash, SessionSearch. DeepSearch is NOT in the list. The compile-time assertion
`assert!(SUB_AGENT_TOOL_COUNT == 7)` enforces this.

**Fix:** Add DeepSearchTool to the sub-agent's tool set, making it 8 tools.
The child DeepSearch must be constructed with `current_depth + 1`, and when
`depth >= max_depth`, the sub-agent is built WITHOUT DeepSearch in its tools
(base case — it can still use Read/Grep/Bash but can't recurse further).

**Depth semantics:**
- `depth = 0` (default for parent agent call): Full recursive DeepSearch
- `depth = 1`: Child sub-agent, can still recurse if max_depth > 2
- `depth >= max_depth`: Base case — tools only, no further DeepSearch
- Default max_depth stays at 50 (tool-call rounds), but we need a separate
  `max_recursion_depth` (default: 2-3) to prevent runaway recursive spawning

**Important distinction:** `max_depth` in rig controls tool-call rounds within
a single agent (how many Read/Grep/etc. calls before giving up). We need a
SEPARATE concept for recursion depth (how many nested DeepSearch levels).

**Implementation changes:**
1. Add `depth: usize` field to `DeepSearchTool` struct
2. Add `max_recursion_depth: Option<usize>` to `DeepSearchArgs` (default: 2)
3. In the sub-agent builder (`build_and_run!` macro), conditionally add
   `DeepSearchTool::new(session_id).with_depth(current_depth + 1)` when
   `current_depth < max_recursion_depth`
4. Update `SUB_AGENT_TOOL_COUNT` to be dynamic (or use two constants)
5. Update the compile-time assertion or make it a runtime check
6. Pass `max_recursion_depth` through the handler chain

### Gap 2: RLM-Aligned System Prompt (P1, LOW effort)

**Current state:** Generic prompt:
```
You are a research assistant tasked with answering a query by exploring a scoped
corpus of files and session history...

STRATEGY:
1. Start by understanding the scope — use Grep or Glob to find relevant files
2. Read targeted sections, not entire files
3. For code: use AstGrep to find structural patterns
4. Use SessionSearch to find relevant past conversations
5. Build up your answer incrementally
6. When you have enough information, provide your final answer
```

**Paper's prompt teaches a fundamentally different strategy:**
1. First explore the scope to understand its size and structure
2. Use Bash (python3) to programmatically chunk/transform data
3. **Delegate sub-questions to recursive DeepSearch calls**
4. Use DeepSearch with simple queries for summarization/extraction (= llm_query)
5. Use DeepSearch with scope for deeper sub-problems (= rlm_query)
6. Aggregate results from sub-calls before answering
7. Don't try to answer until you've explored sufficiently

**New system prompt should teach:**

```
You are a research sub-agent tasked with answering a query by exploring a scoped
corpus. You have tools for reading files, searching, running code, exploring
session history, AND spawning recursive DeepSearch sub-agents for delegation.

{scope_section}

AVAILABLE TOOLS:
- Read: Read file contents (use offset/limit for large files)
- Grep: Search file contents by regex pattern
- AstGrep: AST-based structural code search
- Glob: Find files matching patterns
- Ls: List directory contents
- Bash: Execute shell commands — use python3 -c "..." for data processing,
  chunking, filtering, and transformation
- SessionSearch: Search and view session conversation history
- DeepSearch: Spawn a recursive sub-agent for complex sub-questions. Use this
  to delegate analysis of specific files, sections, or sub-problems.
  - Simple query, no scope → acts like a plain LLM call (fast, one-shot)
  - Query with scope → spawns a full sub-agent that can explore and iterate

STRATEGY — DECOMPOSE, DELEGATE, AGGREGATE:
1. EXPLORE: Use Grep/Glob/Ls to understand what's in scope — file count, sizes,
   structure. Do NOT try to read everything directly.
2. CHUNK: Use Bash (python3) to programmatically identify logical chunks —
   files by directory, sections by delimiter, code by module.
3. DELEGATE: Use DeepSearch to analyze chunks you can't fit in your own context.
   For simple extraction/summarization: DeepSearch(query="summarize: ...", no scope).
   For deep analysis of a sub-tree: DeepSearch(query="...", scope=["specific/path/"]).
4. AGGREGATE: Combine results from sub-calls. Use Bash for data processing if needed.
5. ANSWER: Only provide your final answer when you have sufficient evidence.

IMPORTANT:
- Break large problems into smaller sub-problems and delegate via DeepSearch
- Use Bash + python3 for any data transformation, counting, or filtering
- DeepSearch sub-calls are powerful — they get their own tools and can explore
  independently. Don't try to do everything yourself.
- Your answer should directly address the original query
- If the answer is not in scope, say so explicitly
```

---

## 4. What Doesn't Need to Change

| Paper feature | Why it's already covered |
|--------------|------------------------|
| REPL / code execution | Bash tool with `python3 -c "..."` — already in sub-agent |
| `context` variable | Files on disk ARE the environment — Read/Grep access them |
| `FINAL()` protocol | Rig terminates when LLM stops calling tools — same effect |
| Batched sub-calls | LLM can emit multiple DeepSearch calls per turn — rig runs them |
| Cost controls | `max_depth` (tool rounds) + new `max_recursion_depth` |
| Credentials | Inherited from parent session — already works |

---

## 5. Implementation Plan

### Phase 1: Self-Recursion (~2-3 hours)

**Files to change:**

1. **`codelet/tools/src/deep_search/mod.rs`**
   - Add `depth: usize` field to `DeepSearchTool`
   - Add `max_recursion_depth: Option<usize>` to `DeepSearchArgs` (default: 2)
   - Pass depth through `execute_deep_search()` call
   - Update `build_system_prompt()` — conditionally include DeepSearch tool
     description when depth < max_recursion_depth

2. **`codelet/napi/src/deep_search_handler.rs`**
   - Accept `depth` and `max_recursion_depth` in `execute_deep_search()`
   - In `build_and_run_agent()`: conditionally add `DeepSearchTool` to the
     sub-agent when `depth < max_recursion_depth`
   - The child's DeepSearchTool gets `depth + 1`
   - Need to register a DeepSearch handler for the CHILD sub-agent too
     (so the child can invoke DeepSearch). This handler wraps another call
     to `execute_deep_search()` with `depth + 1`.
   - Update or remove the compile-time tool count assertion (now dynamic)

3. **`codelet/napi/src/session_manager.rs`**
   - Update the parent's DeepSearch handler registration to pass `depth=0`
     and `max_recursion_depth` (from args or default)

4. **`codelet/napi/src/deep_search_provider_config.rs`**
   - No changes needed — provider config is depth-agnostic

### Phase 2: System Prompt Rewrite (~1 hour)

1. **`codelet/tools/src/deep_search/mod.rs`** — `build_system_prompt()`
   - Rewrite to the RLM-aligned prompt (see §3 Gap 2 above)
   - Conditionally include DeepSearch in AVAILABLE TOOLS section based on
     whether the sub-agent has recursion capability at this depth
   - Include strategy section teaching decompose-delegate-aggregate

### Phase 3: Depth Display & Logging (~30 min)

1. Add depth info to the sub-agent's system prompt: "You are at recursion
   depth {depth} of max {max_recursion_depth}"
2. Log recursion depth in handler for debugging

---

## 6. Recursion Safety

**Risk:** Unbounded recursive DeepSearch spawning.

**Mitigations (already partially in place):**
- `max_recursion_depth` (new, default: 2) — hard limit on nesting
- `max_depth` (existing, default: 50) — tool-call rounds per agent
- Each sub-agent is ephemeral — no state accumulation
- Each level creates its own ProviderManager — no shared mutable state
- SessionSearch handlers are per-UUID — no conflicts between levels

**Example recursion tree (max_recursion_depth=2):**
```
Parent agent (interactive session)
 └─ DeepSearch(depth=0, query="How does auth work?", scope=["src/"])
     ├─ Grep → finds 47 files
     ├─ DeepSearch(depth=1, query="Analyze src/auth/login.rs", scope=["src/auth/login.rs"])
     │   ├─ Read(src/auth/login.rs)
     │   ├─ AstGrep(...)
     │   └─ [returns synthesized answer — cannot recurse further at depth=1 if max=2]
     ├─ DeepSearch(depth=1, query="Analyze src/auth/session.rs", scope=["src/auth/session.rs"])
     │   └─ [returns synthesized answer]
     └─ [aggregates sub-answers, returns final answer to parent]
```

---

## 7. References

- RLM Paper: https://arxiv.org/abs/2512.24601
- RLM Source Code: https://github.com/alexzhang13/rlm
- RLM Minimal: https://github.com/alexzhang13/rlm-minimal
- RLM System Prompt: https://github.com/alexzhang13/rlm/blob/main/rlm/utils/prompts.py
- Key codelet files:
  - `codelet/tools/src/deep_search/mod.rs` — DeepSearchTool struct + system prompt
  - `codelet/napi/src/deep_search_handler.rs` — Sub-agent construction + execution
  - `codelet/napi/src/deep_search_provider_config.rs` — Provider-specific configs
  - `codelet/napi/src/session_manager.rs:~4297` — Handler registration
