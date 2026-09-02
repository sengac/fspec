# Research: SessionSearch content truncation — when an agent reading another agent's session cannot recover the information it needs

**Date:** 2026-09-02
**Status:** Research only (no code changes)
**Intended use:** Attach to a new work unit card to fix the issue.
**Scope:** The `SessionSearch` tool (`recent` / `search` / `show`), the persistence layer it reads, the upstream rig-core tool-result bounding, and the related `AgentManager` cross-session `context` references.

---

## 1. TL;DR

SessionSearch returns content that is **truncated at several independent layers**, and in some of those layers the original content is **permanently destroyed before it ever reaches the tool**. An agent that is trying to read a *different* agent's session (or its own prior session after compaction) frequently hits a wall where:

1. A message comes back cut to **5 000 chars** with a `truncated: true` flag — and **there is no follow-up call to fetch the rest** (no per-turn "full" mode, no content offset).
2. A large tool result (fspec `board`, `validate-tags`, a big `Read`, a big `Bash` output) is already **destroyed to a 2 KB preview + 512 B suffix** by a 64 KiB hard cap in the rig-core patch *before* persistence — so even a "give me the full content" feature would have nothing to give.
3. `search` previews are **300 chars** and `recent` first/last-user-message previews are **200 chars** — enough to triage, never enough to act on.
4. When compaction Layer-0 trimming is active, tool results are replaced with **one-line references** (`[file: path, N lines, T tok — use Read to retrieve]`), so the reading agent gets a pointer, not the data.
5. The sibling path — `AgentManager` `message` with `context: [{session_id, turns: [...]}]` — truncates **each referenced turn to 2 000 chars**, a separate cap that has the same "can't get the full referenced turn" problem.

The core design gap: **truncation is one-directional and there is no "drill back down" primitive.** The `truncated: true` flag tells the agent content was cut, but the tool surface offers no way to retrieve the remainder.

---

## 2. How SessionSearch works (architecture + data flow)

**Tool + schema** (rig `Tool` impl):
- `rust/tools/src/session_search/mod.rs` — `SessionSearchTool`, tool definition, JSON schema (the LLM-facing parameter list).
- `rust/tools/src/session_search/types.rs` — `SessionSearchArgs`, `SessionSearchAction` (`Recent` / `Search` / `Show`), result types, and the truncation constants.
- `rust/tools/src/session_search/reassembly.rs` — reassembles stored streaming chunks (`[Thinking: ...]`, `[Tool: ...]`, raw SSE text fragments) into readable sections.

**Execution** (per-session handler registered by the agent loop):
- `rust/tools/src/session_search/handler.rs` — handler registry (`set_session_search_handler`, `execute_session_search`).
- `rust/agent-loop/src/session_search_handler.rs` **and** the near-identical twin `rust/napi/src/session_search_handler.rs` — `create_handler()`, `handle_recent()`, `handle_search()`, `handle_show()`, `resolve_message_content()`, `extract_match_preview_ripgrep()`, `build_context_turns()`, `truncate_preview()`, `ConditionalTrimmer`. (Both files must be kept in lockstep.)

**Read path:**
1. Handler resolves the session (`load_session`) and loads every message via `get_session_messages_full()` (compaction-ignoring).
2. `resolve_message_content(msg)` returns the text to display:
   - if `msg.content` is a `blob:sha256:<hash>` reference → fetch the blob;
   - else if `msg.blob_refs` is non-empty → join `content` + each blob;
   - else → `msg.content` verbatim.
   - **It never reads `msg.metadata`.** (This matters — see §4, finding F1.)
3. For `show`: reassemble streaming chunks, apply optional Layer-0 trimming, then truncate each message to `MESSAGE_TRUNCATION_LIMIT`.

### 2.1 The truncation constants (single source of truth)

All defined in `rust/tools/src/session_search/types.rs`:

| Constant | Value | Where applied |
|---|---|---|
| `MESSAGE_TRUNCATION_LIMIT` | **5 000** chars | `show` per-message content; `search` context turns |
| `DEFAULT_RECENT_COUNT` | 10 | `recent` (sessions) |
| `DEFAULT_SEARCH_LIMIT` | 20 | `search` (matches) |
| `USER_MESSAGE_PREVIEW_LEN` | **200** chars | `recent` first/last user message |
| `extract_match_preview_ripgrep` `MAX_PREVIEW` | **300** chars | `search` `matched_content` (±100-char context) |

Plus, *outside* the tool:
- rig-core patch: `MAX_TOOL_RESULT_TEXT_BYTES = 64 * 1024`, preview 2 KiB, suffix 512 B (`rust/patches/rig-core/src/agent/prompt_request/streaming.rs:36-114`).
- AgentManager `context` refs: **2 000** chars per turn (`rust/napi/src/agent_manager_handler.rs:683, 764`).
- Persist-time summary: **200** chars for tool results, **50** chars for thinking (`rust/agent-loop/src/persist.rs:89, 179-188`).

---

## 3. Every truncation layer, in order (storage → the agent's eyes)

```
Tool output (e.g. fspec board, Read, Bash)            ~264 KB
   │
   │  (a) rig-core patch  bound_tool_result_text()      ── 64 KiB cap, destroys the rest
   ▼
ToolResult string passed to the stream loop            64 KiB marker (2 KB preview + 512 B suffix)
   │
   │  (b) persist_tool_result_internal()                ── content field = 200-char summary
   ▼
StoredMessage on disk                                  content=summary, metadata envelope=marker
   │
   │  (c) SessionSearch resolve_message_content()       ── reads content field (200-char summary), NOT metadata
   ▼
   │  (d) conditional Layer-0 Trimmer (if compaction)   ── tool results → one-line references
   ▼
   │  (e) SessionSearch display caps                    ── show:5000 / search:300 / context:5000 / recent:200
   ▼
   │  (f) SessionSearch result itself is a tool_result  ── re-bounded by the 64 KiB rig cap if the result is large
   ▼
Agent's context window
```

Each layer is described in §4 with the code locations.

---

## 4. Findings (with code locations)

### F1 — `show` hard-caps each message at 5 000 chars and offers no way to get the rest
`handle_show` (`rust/agent-loop/src/session_search_handler.rs:398-411`):
```rust
let truncated = content.len() > MESSAGE_TRUNCATION_LIMIT;   // 5000
if truncated {
    let boundary = floor_char_boundary(&content, MESSAGE_TRUNCATION_LIMIT);
    content = format!("{}...", &content[..boundary]);
}
result_messages.push(SessionMessage { ..., truncated });
```
`SessionMessage.truncated` (`types.rs:177`) is a boolean that **has no follow-up mechanism**. The `Show` args (`types.rs:56-72`) support `user_only`, `max_turns`, `start_turn`, `end_turn` — but **no per-message full/untruncated mode and no content offset/limit**. So once a message exceeds 5 000 chars, the tail is unreachable through SessionSearch.

### F2 — `search` previews are 300 chars; context turns are 5 000 chars
`extract_match_preview_ripgrep` (`:617-643`) caps `matched_content` at `MAX_PREVIEW = 300`. `build_context_turns` (`:646-679`) caps each context turn at `MESSAGE_TRUNCATION_LIMIT` (5 000) via `truncate_preview`.

### F3 — `recent` first/last user message previews are 200 chars
`get_user_message_previews` (`:434-459`) → `truncate_preview(&content, USER_MESSAGE_PREVIEW_LEN)` (200).

### F4 — `resolve_message_content` ignores the message's metadata envelope, so full tool results already on disk are under-reported
`resolve_message_content` (`:479-501`) reads only `msg.content` (and `msg.blob_refs`). It does **not** read `msg.metadata["message"]["content"][i]["content"]`.

On-disk evidence (from `~/.fspec/messages/messages.jsonl`):
- A sub-64 KiB tool result: the `content` field is a **203-char summary** (`"ed56bf69 feat(tui): add shared animated loading dialog..."`) while the metadata envelope's `tool_result.content` holds the **full 1 260-char** git log. SessionSearch returns only the 203-char field.
- **Conclusion:** the full tool result is stored (in metadata) but SessionSearch never surfaces it.

### F5 — Large tool results are destroyed *before* persistence (unrecoverable data loss)
For any tool output over 64 KiB, `bound_tool_result_text` in the rig-core patch (`rust/patches/rig-core/src/agent/prompt_request/streaming.rs:91-114`) replaces it with a JSON marker:
```json
{ "status":"truncated", "original_bytes":264669, "max_bytes":65536,
  "preview":"<2 KB>", "suffix":"<512 B>", "hint":"..." }
```
That marker string is what flows into `emit_tool_result` → `persist_tool_result_internal`, so **both** the `content` field **and** the metadata envelope hold only the marker (confirmed on disk: `content` = 203 B marker, `metadata.message.content[0].content` = 2 777 B marker, `blob_refs` = `[]`, no `_blobRefs`, no `blob:sha256:` anywhere). The original 264 669 bytes never touch disk.

**This means a "fetch the full content" feature cannot recover over-64-KiB tool results** — the data simply is not there. This is a separate, deeper defect from the display caps in F1–F3.

### F6 — Layer-0 compaction trimming replaces tool results with pointers
When the session's `compaction_in_progress` flag is set, `ConditionalTrimmer` (`:727-753`) runs `codelet_core::compaction::Trimmer` over each message (`rust/core/src/compaction/trimmer.rs`):
- `Read` → `[file: path, N lines, T tok — use Read to retrieve]` (`trim_read_result`, line 190)
- `Bash` → head 10 + tail 5 lines (`trim_bash_output`, line 200)
- `Grep`/`AstGrep`/`Glob`/`Ls` → first 10 matches (`trim_grep_output`, line 225)
- `Write` → `[Write: path, N lines — file persisted to disk]` (line 156); `Edit` → char-count reference (line 171)

So during/after compaction, a reading agent gets *pointers*, not data, for tool results. This is by design for the compacting agent, but it also limits what a *different* agent can read.

### F7 — The SessionSearch result itself can be re-truncated by the 64 KiB cap
A `show`/`search` over a busy session can itself exceed 64 KiB, in which case the rig patch truncates *the tool result of SessionSearch* to a 2 KiB preview. Observed directly in the field (see §5, session `701b9301` turn 646): the agent's own `show` came back as `{"status":"truncated","original_bytes":67031,...}`.

### F8 — Related: `AgentManager` cross-session `context` refs truncate each turn to 2 000 chars
`resolve_turns_context` / `resolve_query_context` (`rust/napi/src/agent_manager_handler.rs:640-815`) resolve a supervisor's `context: [{session_id, turns:[...]}]` by reading each turn via `resolve_message_content` and cutting to `content[..2000] + "... [truncated]"`. This is the "other agent's session" path the task specifically cares about, and it has the same no-drill-down problem, with a *lower* cap (2 000) than SessionSearch's `show` (5 000).

---

## 5. Real examples of agents hitting the wall (from session history)

(Discovered via `SessionSearch` across this project's own sessions.)

- **`701b9301-8477-4a8c-a09f-d5f9ff51811f` (agent re-reading its own prior session)**
  - Turn 646: its `show` returned `{"status":"truncated","original_bytes":67031,"max_bytes":65536,"preview":...}`.
  - Turn 647 (assistant): *"The session is truncated. To understand what I was doing, let me look at the actual code state."* → the agent gave up on SessionSearch and fell back to re-deriving state from the repo. **This is the exact failure mode the task describes.**
- **`ff8ddc47-83ac-43f6-a972-bc16ea57d65f`** — turns 330, 332: fspec `board` / `validate` outputs came back as 64 KiB truncation markers (`original_bytes` 299 577 and 2 210 971); the agent noted the output was "truncated due to size."
- **`89fddaf5-8c04-4d2a-a9df-b210456cfaf8`** — turns 240, 332: same pattern; turn 337 "the full output of validate-tags is truncated (2.2 MB, 513 invalid...)."
- **`9cb124c1-2c6f-47b5-a778-15fd9b448bf4`** — turns 50, 153, 397: repeated 64 KiB markers on `board`/`validate`; the agent kept re-running `validate-tags` trying to see the full list.
- **`cd3a590a-b519-4ce8-b9f3-ca1e4de9a3f7`** — turn 14: a SessionSearch result itself came back as `{"status":"truncated","original_bytes":93966,...}`.

These are almost all the **64 KiB upstream cap (F5/F7)** rather than the 5 000-char display cap — which is an important nuance: the most painful "can't get the info" cases are *upstream data loss*, not SessionSearch display truncation. Both matter, and they need different fixes.

---

## 6. Root-cause analysis

Two distinct defects, easily conflated:

1. **Display truncation (recoverable):** SessionSearch caps content at 5 000 / 300 / 200 chars and exposes no primitive to fetch more (F1–F3, F8). The data is on disk; the tool just won't hand it over.
2. **Persist-time data loss (not recoverable via SessionSearch):** the 64 KiB rig-core cap (F5) destroys large tool results before they reach the store, and `resolve_message_content` (F4) doesn't read the metadata envelope where sub-64-KiB full results *do* live.

A fix that only raises the display caps helps case 1 but is **useless for case 2** — there is nothing larger to raise into.

---

## 7. Proposed solutions (options + tradeoffs)

Design goal (from the task): **give the agent access to everything it needs without bloating context by default** — i.e., cheap to *discover* that more exists, expensive only to *retrieve* it on demand.

### A. Per-turn full-content fetch (primary recommendation)
Add the ability to request one turn's content untruncated, e.g. a new action `read_turn` or a `show` extension:
```
SessionSearch(action="show", session_id=X, turn=N, full=true)
```
- `full=true` bypasses the 5 000-char cap **and** the Layer-0 Trimmer for that single turn.
- Default (`full` absent/false) keeps today's behavior → zero context-bloat change for the common path.
- Cost: one turn's content, agent must name the turn (it already has `turn_index` from `search`).
- Directly answers "unless it asks for the full information."

### B. Make truncation markers self-describing + actionable
When a message/turn is cut, return a compact header the agent can act on:
```json
{ "turn_index": 42, "role": "assistant",
  "content": "<5000 chars>",
  "total_chars": 18432, "returned_chars": 5000,
  "hint": "call show(session_id, turn=42, full=true) or content_offset to continue" }
```
Today `truncated: true` is a dead end; this turns it into a pointer. Cheap (a few ints + a string) and high leverage.

### C. Content-level offset / limit (paging one huge message)
Add `content_offset` + `content_limit` (chars, or tokens) to `show`/`read_turn` for a specific turn so the agent can page through a very long message with repeated calls — the "offset and make repeated calls" model from the task. Pairs naturally with B.

### D. Configurable per-message cap
A `max_content_chars` param (default 5 000) so an agent can request, say, 20 000 for a targeted turn without a full fetch. Simpler than A, still bounded; does **not** solve F5 (over-64-KiB loss).

### E. Fix the upstream data loss (separate, deeper card)
To truly "give the agent access to all of what it needs" for large tool results:
- At persist time, write the **full** tool result to blob storage (keyed by `tool_call_id`), store a `blob:sha256:` ref on the message, and let `resolve_message_content` rehydrate it on demand (the blob machinery + 10 KiB threshold already exist in `rust/core/src/persistence/blob.rs` / `blob_processing.rs`).
- This requires touching the persist boundary (`rust/agent-loop/src/persist.rs`, `rust/cli/src/interactive/stream_handlers.rs`) and/or the rig patch so the full payload is available *before* the 64 KiB bound is applied.
- Tradeoff: higher disk usage; must be opt-in/tunable. It is the only way to recover the over-64-KiB cases in F5.

### F. Triage "digest" mode for cross-agent reads
A `show` mode that returns a per-turn digest (first ~200 chars of each turn + `[Tool: ...]` markers + annotation summary) so a reading agent can cheaply scan *which* turns to then `read_turn(full=true)`. Keeps default context tiny, enables targeted full-fetch.

### G. Raise/align the AgentManager `context` cap
The 2 000-char per-turn cap in `AgentManager` `context` refs (F8) has the same no-drill-down problem. At minimum, make it self-describing (B) and/or raise it; ideally route it through the same per-turn full-fetch primitive (A) so a subordinate can request a referenced turn in full.

---

## 8. Recommended scope for the first card

Split into two cards (matching the two root causes in §6):

**Card 1 — "SessionSearch can't drill back down" (display layer, low-risk, high-value):**
1. **A** per-turn `full=true` fetch (bypass 5 000 cap + Layer-0 trim for that turn).
2. **B** self-describing, actionable truncation markers (`total_chars`, `returned_chars`, `hint`).
3. **C** content `offset`/`limit` paging for a single turn.
4. **F4** make `resolve_message_content` read the metadata envelope so full sub-64-KiB tool results already on disk are actually surfaced.
5. **G** make the AgentManager 2 000-char context cap self-describing / aligned.

**Card 2 — "Large tool results are destroyed before persistence" (data loss, needs care):**
- **E** persist full tool results to blob before the 64 KiB bound; rehydrate on demand. (Separate, larger, touches persist boundary.)

Card 1 gives agents a real way to get what they need for the common case and makes every truncation discoverable; Card 2 closes the hard data-loss hole.

---

## 9. Open questions / risks

- **Context bloat:** `full=true` for a 200-KiB turn will flood the calling agent's window. Mitigate by (a) requiring an explicit turn target, (b) returning a `total_chars` up front so the agent can decide, (c) preferring the blob/offset path (E/C) over inline full for very large turns.
- **Token accounting:** `show` currently has no token estimate in the result; adding `total_tokens`/`returned_tokens` per message would let agents budget. (See `token_estimator`.)
- **Twin-file drift:** the handler exists in both `rust/agent-loop` and `rust/napi` — any change to truncation must land in both (or be de-duplicated).
- **Backwards compatibility:** the `content` field in `messages.jsonl` is currently the 200-char summary for tool results; F4 reads a *different* field (metadata), so it is additive, but the blob-ref approach (E) changes the `content` field's meaning for new writes — existing rows must keep resolving.
- **Scope of "full":** should `full` also reassemble streaming chunks (thinking + tool markers + prose)? Probably yes, for parity with today's `show`, but it should be documented.
- **Is `resolve_message_content`'s ignoring of metadata a bug or intentional?** Worth a quick confirm with the persistence owner before F4 lands — it may be a deliberate simplification that just never got extended.

---

## 10. Evidence appendix (session IDs + turn indices)

| Session | Provider | Turns | What happened |
|---|---|---|---|
| `701b9301-8477-4a8c-a09f-d5f9ff51811f` | openai:runpod qwen | 645–647 | `show` → 67 031 B result truncated to 2 KB preview; agent abandoned SessionSearch, fell back to repo state |
| `ff8ddc47-83ac-43f6-a972-bc16ea57d65f` | openai:runpod qwen | 330, 332 | fspec `board`/`validate` → 299 577 B & 2 210 971 B → 64 KiB markers |
| `89fddaf5-8c04-4d2a-a9df-b210456cfaf8` | openai:runpod qwen | 240, 332, 337 | repeated 64 KiB markers; "validate-tags output is truncated (2.2 MB)" |
| `9cb124c1-2c6f-47b5-a778-15fd9b448bf4` | openai:sglang qwen | 50, 153, 397 | `board`/`validate` 64 KiB markers; repeated re-runs to see full list |
| `cd3a590a-b519-4ce8-b9f3-ca1e4de9a3f7` | openai:spark qwen | 2, 14 | `board` 269 145 B marker; SessionSearch result 93 966 B marker |
| (on-disk) `~/.fspec/messages/messages.jsonl` | — | line 2 / 4 | F4/F5 proof: `content` field = 203 B summary or 64 KiB marker; metadata envelope = full sub-64-KiB or marker; `blob_refs` empty |

### Key code locations (for the fix cards)
- `rust/tools/src/session_search/types.rs` — constants (`:205-214`), `Show` args (`:56-72`), `SessionMessage.truncated` (`:177`)
- `rust/agent-loop/src/session_search_handler.rs` (twin: `rust/napi/src/session_search_handler.rs`) — `handle_show` (`:311-427`), `resolve_message_content` (`:479-501`), `extract_match_preview_ripgrep` (`:617-643`), `build_context_turns` (`:646-679`), `ConditionalTrimmer` (`:727-753`)
- `rust/core/src/compaction/trimmer.rs` — Layer-0 tool-result trimming
- `rust/patches/rig-core/src/agent/prompt_request/streaming.rs` — `bound_tool_result_text` / `MAX_TOOL_RESULT_TEXT_BYTES` (`:30-114`)
- `rust/agent-loop/src/persist.rs` — `persist_tool_result_internal` (200-char summary, `:140-195`), `persist_assistant_message_internal` (50-char thinking, `:89`)
- `rust/napi/src/persistence/napi_bindings.rs` — `persistence_store_message_envelope` + `extract_content_summary` (`:628-687`, `:908-942`)
- `rust/core/src/persistence/blob.rs` / `blob_processing.rs` — existing blob store + 10 KiB threshold (for E)
- `rust/napi/src/agent_manager_handler.rs` — `resolve_turns_context` / `resolve_query_context` (2 000-char cap, `:640-815`)
