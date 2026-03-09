# Tiered Memory Compaction with Structural Anchors and SessionSearch Retrieval

## Executive Summary

This document proposes replacing the current LLM-based anchor detection and narrative summary compaction system with a **three-tier memory architecture**. The current system makes an expensive batch LLM call to detect anchors, discards old turns, and generates a single narrative summary. The proposed system eliminates LLM calls from the core compaction pipeline, uses cheap per-turn **structural signal detection** for anchor detection, maintains an incrementally-built structured memory document, and leverages SessionSearch for on-demand retrieval of full historical detail.

### Design Principle: Only Detect What You Can Prove

This design draws a hard line between **structural signals** (tool names, exit codes, parameter values, success/failure booleans) and **semantic inference** (interpreting natural language intent, detecting "key decisions," classifying direction changes from prose). The system detects ONLY structural signals. It does not attempt to infer meaning from natural language content — that path leads to substring-matching heuristics that silently produce wrong results with no way to know.

**What we CAN detect deterministically:**
- Which tools were called (exact string match on tool name)
- Whether they succeeded (boolean `success` field)
- Which files were touched (structured `file_path` parameter)
- Which fspec commands were run (structured `command` parameter)
- Whether the previous turn had errors (boolean check on prior turn)

**What we DO NOT attempt to detect:**
- Whether the user changed direction (requires understanding intent)
- Whether a "key decision" was made (requires understanding reasoning)
- Whether test output means tests passed (substring matching "pass" in arbitrary output is unreliable)

**Key benefits:**
- Zero LLM cost during compaction (currently: 2+ LLM calls per compaction)
- Incremental anchor detection (currently: batch analysis of all turns)
- Structured session memory that serves as a searchable index into history
- No information loss — full history always available via SessionSearch
- No silent misclassification from flaky pattern matching

---

## Problem Analysis: Current System Weaknesses

### 1. Expensive Anchor Detection

The current `AnchorDetector::detect_batch()` sends ALL conversation turns to the LLM in a single prompt asking it to classify each turn. For a 50-turn session, this means serializing 50 turns worth of assistant responses, tool calls (with full parameter JSON), tool results, and error states into one massive prompt. The LLM then returns a JSON array classifying each turn.

**Cost:** One large LLM call (potentially tens of thousands of tokens) every time compaction triggers. Timeout scales at `15s + 2s × num_turns`.

### 2. Lossy Compression

After compaction, summarized turns are replaced by a single LLM-generated narrative paragraph. This summary is a best-effort compression — the LLM decides what to include and what to omit. Key decisions, error context, and subtle reasoning may be lost. The agent has no way to recover this information after compaction.

### 3. Re-detection on Every Compaction

Previous anchors are persisted for the UI viewer, but they are NOT fed back into subsequent compaction cycles. The detector starts fresh each time, re-analyzing all current turns. An anchor detected at turn 40 in the first compaction may or may not be re-detected in the second compaction.

### 4. All-or-Nothing Summarization

The current selector splits turns into "keep" and "summarize" with no middle ground. Turns before the anchor are fully summarized (lossy), turns after are fully kept (lossless). There's no concept of partially-compressed context — a structured index of what happened without full verbatim content.

### 5. Narrative Summaries Are Fragile

The LLM-generated summary is a prose paragraph. It cannot be incrementally updated, searched, or referenced by turn number. If the agent needs to know "what happened at turn 12," the summary either mentions it or it's gone.

### 6. Existing Heuristics Are Already Flaky

The current codebase already has pattern-matching heuristics that silently misclassify. `PreservationContext::detect_build_status()` checks if tool output contains the substrings "pass" or "fail" — but "pass" appears in "password", "bypass", "passenger", and environment variables like `DB_PASS`. `extract_goal_from_message()` matches action verbs like "build", "remove", "please" in user messages — so "please show me the build output" gets classified as a goal. These heuristics appear to work because their failures are silent, not because they're correct.

---

## Proposed Architecture: Three-Tier Memory

### Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     ACTIVE CONTEXT WINDOW                        │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ SYSTEM REMINDERS (stable prefix)                         │  │
│  │ - CLAUDE.md / environment info / fspec guidance           │  │
│  │ - Never compacted, always first                           │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ WARM TIER: Structured Session Memory Document             │  │
│  │ - Incrementally updated after each turn                   │  │
│  │ - File modifications, tool outcomes, fspec milestones     │  │
│  │ - Each entry has a [turn N] reference for cold retrieval  │  │
│  │ - Populated ONLY from structured tool call/result data    │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ HOT TIER: Recent Turns (verbatim)                         │  │
│  │ - Last 3-5 complete conversation turns                    │  │
│  │ - Full fidelity: user messages, tool calls, responses     │  │
│  │ - Rotated on compaction                                   │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
                            │
                            │ SessionSearch (on demand)
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│ COLD TIER: Full Persisted History                                │
│ - Every message ever sent/received, stored in persistence        │
│ - Searchable via SessionSearch tool (regex, time filters)        │
│ - Agent retrieves specific turns when warm tier references them  │
│ - Zero cost when not accessed                                    │
└─────────────────────────────────────────────────────────────────┘
```

### Tier 1: Hot Tier (Recent Turns)

**What:** The last 3-5 complete conversation turns, kept verbatim in the context window.

**Behavior:**
- Always present in the active message array
- Full fidelity — user messages, assistant responses, tool calls with parameters, tool results with output
- When compaction triggers, the oldest hot turns are "demoted" — their structural data is appended to the warm tier, then removed from the message array
- The number of hot turns is configurable (default: 5)

**Why 5 turns:** This provides enough immediate context for the agent to continue work without needing to search. Most follow-up actions reference the last 2-3 turns. 5 gives a small buffer.

### Tier 2: Warm Tier (Structured Session Memory)

**What:** A structured document that lives as a user message in the context window, between system reminders and hot turns. It serves as both a compressed summary AND an index into cold storage.

**Critical constraint:** The warm tier contains ONLY data extracted from structured fields (tool names, parameters, success booleans, fspec command names). It does NOT contain any entries derived from natural language interpretation of user messages or assistant responses. The user messages and assistant reasoning are available verbatim in the hot tier (recent) or via SessionSearch (historical).

**Format:**

```markdown
## Session Memory

### Current State
- **Work Unit:** AUTH-001 (implementing)
- **Last Tool Outcome:** Edit succeeded on src/auth/validator.ts
- **Active Files:** src/auth/validator.ts, src/auth/__tests__/validator.test.ts

### File Modification Log
- [turn 8] Created src/auth/validator.ts (Write)
- [turn 14] Modified src/auth/validator.ts (Edit)
- [turn 19] Created src/auth/__tests__/validator.test.ts (Write)
- [turn 25] Modified src/config/redis.ts (Edit)

### Tool Failure Log
- [turn 16] Edit failed on src/config/config.ts → succeeded on retry [turn 17]
- [turn 28] Bash failed (exit 1) → succeeded [turn 30]

### Milestones (fspec)
- [turn 10] create-feature: user-authentication
- [turn 15] update-work-unit-status: AUTH-001 → testing
- [turn 22] link-coverage: user-authentication / "Login with valid credentials"
- [turn 35] update-work-unit-status: AUTH-001 → implementing
```

**Key properties:**
- **Machine-readable sections** — not narrative prose. Each section has a clear purpose.
- **Turn references** — every entry includes `[turn N]` so the agent can use SessionSearch to retrieve full detail.
- **Incrementally updated** — after each turn completes, new entries are appended to the relevant sections. No batch reprocessing.
- **Bounded growth** — sections have maximum entry counts with pruning rules (see Pruning Strategy below).
- **No interpretation** — every entry is derived from structured data. File paths come from `tool_call.parameters.file_path`. Tool names come from `tool_call.tool`. Fspec commands come from `tool_call.parameters.command`. Success/failure comes from `tool_result.success`. Nothing comes from parsing prose.

**What's NOT in the warm tier:**

The previous design included "Key Decisions" and "Resolved Errors" sections that would require interpreting natural language (e.g., detecting that "chose bcrypt over argon2" was a decision, or that an error was semantically "resolved"). These are removed. The warm tier tracks structural facts: which files changed, which tools succeeded/failed, which fspec milestones were reached. The reasoning and context behind those facts lives in the hot tier and cold tier.

### Tier 3: Cold Tier (SessionSearch)

**What:** The complete, unmodified conversation history stored in the persistence layer, accessible via the SessionSearch tool.

**Behavior:**
- Every message is already persisted by the existing NAPI persistence system
- The agent uses SessionSearch when it needs detail beyond what the warm tier provides
- Typical retrieval patterns:
  - `search("bcrypt")` — find all mentions of a technology choice
  - `show(current, max_turns: 3)` — see recent context around a specific point
  - `search("error.*redis")` — find all Redis-related errors
- **No changes needed to SessionSearch** — it already supports all required operations
- Cold tier has zero context window cost — it only consumes tokens when the agent actively queries it

---

## Structural Anchor Detection

### Why Structural Signals Instead of LLM

The current system asks the LLM "is this turn important?" The LLM is good at nuance but terrible at cost — one call per compaction, analyzing all turns. The previous design proposal attempted to replace this with "heuristics" that included natural language pattern matching (e.g., checking if user messages contain phrases like "now let's", "switch to", "new approach"). That approach is flaky — it silently misclassifies with no feedback mechanism.

This design takes a stricter approach: detect ONLY what can be proven from structured data fields. No substring matching against natural language content.

### What We Detect

| Signal Source | Data Field | Detection | Anchor Type |
|---------------|-----------|-----------|-------------|
| Tool call name | `tool_call.tool` | Exact match: `"Edit"`, `"Write"` | File modification (warm tier entry, not anchor) |
| Tool success | `tool_result.success` | Boolean `true`/`false` | Error tracking (warm tier entry) |
| Fspec command | `tool_call.parameters.command` | Exact match against allowlist | FeatureMilestone |
| Prior turn failure + current success | `previous_turn.tool_results` + `current_turn.tool_results` | Boolean: any prior failure + all current success | ErrorResolution |
| Bash exit code | `tool_result.success` for Bash tool | Boolean (exit code 0 = success) | Part of ErrorResolution compound check |

### What We Do NOT Detect

| Dropped Detection | Why It Was Removed |
|---|---|
| **UserCheckpoint** (direction change) | Required substring matching against user messages ("now let's", "switch to", etc.). False positives: "let's work on fixing this test" (continuation, not pivot). False negatives: "Scratch that." No reliable structural signal exists for intent changes. |
| **TaskCompletion** (code change + tests pass) | Required substring matching "pass"/"PASS"/"✓" against Bash output. "pass" appears in "password", "bypass", `DB_PASS`. Test runners have diverse output formats. Exit code 0 already captured by `tool_result.success`, which is sufficient. |
| **Key Decisions** | No structural signal. Detecting that "chose bcrypt over argon2" is a decision requires understanding natural language semantics. This is inherently an LLM task. |

### Remaining Anchor Types

After removing flaky detections, two anchor types remain:

**1. FeatureMilestone** — Detected with 100% precision from fspec tool calls:

```rust
fn detect_fspec_milestone(turn: &ConversationTurn) -> Option<AnchorPoint> {
    for tc in &turn.tool_calls {
        if tc.tool != "Fspec" {
            continue;
        }

        let command = tc.parameters.get("command")
            .and_then(|v| v.as_str());

        let milestone_command = match command {
            Some("update-work-unit-status") => true,
            Some("create-feature") => true,
            Some("generate-scenarios") => true,
            Some("link-coverage") => true,
            Some("update-work-unit-estimate") => true,
            _ => false,
        };

        if milestone_command {
            // Check that the tool call succeeded
            let succeeded = turn.tool_results.iter()
                .any(|tr| tr.success);

            if succeeded {
                return Some(AnchorPoint {
                    turn_index: 0, // Set by caller
                    anchor_type: AnchorType::FeatureMilestone,
                    weight: 0.75,
                    confidence: 1.0,
                    description: format!(
                        "fspec {}{}",
                        command.unwrap_or("unknown"),
                        extract_fspec_args(tc),
                    ),
                    timestamp: turn.timestamp,
                });
            }
        }
    }
    None
}

/// Extract human-readable args from fspec tool call parameters
fn extract_fspec_args(tc: &ToolCall) -> String {
    // Extract work unit ID or feature name from args
    if let Some(args) = tc.parameters.get("args") {
        if let Some(args_str) = args.as_str() {
            // Parse the JSON args string to extract positional args
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(args_str) {
                if let Some(positional) = parsed.get("_").and_then(|v| v.as_array()) {
                    let parts: Vec<&str> = positional.iter()
                        .filter_map(|v| v.as_str())
                        .collect();
                    if !parts.is_empty() {
                        return format!(": {}", parts.join(" → "));
                    }
                }
            }
        }
    }
    String::new()
}
```

**2. ErrorResolution** — Detected from structural success/failure transitions:

```rust
fn detect_error_resolution(
    current_turn: &ConversationTurn,
    previous_turn: Option<&ConversationTurn>,
) -> Option<AnchorPoint> {
    // Requires: previous turn had at least one tool failure
    let previous_had_failure = previous_turn
        .map(|pt| pt.tool_results.iter().any(|tr| !tr.success))
        .unwrap_or(false);

    if !previous_had_failure {
        return None;
    }

    // Requires: current turn has file modifications
    let has_file_modification = current_turn.tool_calls.iter()
        .any(|tc| tc.tool == "Edit" || tc.tool == "Write");

    if !has_file_modification {
        return None;
    }

    // Requires: current turn has ALL tool results succeeding
    let all_current_succeeded = !current_turn.tool_results.is_empty()
        && current_turn.tool_results.iter().all(|tr| tr.success);

    if !all_current_succeeded {
        return None;
    }

    let modified_files = extract_modified_file_paths(current_turn);

    Some(AnchorPoint {
        turn_index: 0, // Set by caller
        anchor_type: AnchorType::ErrorResolution,
        weight: 0.9,
        confidence: 1.0,
        description: format!(
            "Error resolved: modified {}",
            modified_files.join(", "),
        ),
        timestamp: current_turn.timestamp,
    })
}
```

**Note on ErrorResolution precision:** This detection checks three structural conditions (previous failure, current file modification, current all-success). It can produce false positives in one scenario: the previous turn failed for an unrelated reason, and the current turn edits a file and succeeds on an unrelated task. This is acceptable because the consequence of a false positive anchor is only that the turn gets slightly higher preservation priority during compaction — it doesn't cause data loss or incorrect behavior. The detection has zero false negatives for the pattern it describes.

### When Detection Runs

**Current system:** Detection runs at compaction time, analyzing ALL turns in a batch LLM call.

**Proposed system:** Detection runs **after each turn completes**, inline. The result is stored immediately in the `BackgroundSession`'s anchor list AND appended to the warm tier's "Milestones" section.

This means:
- `/anchors` shows real-time anchors (not just post-compaction)
- The warm tier always has an up-to-date milestone history
- Compaction never needs to detect anchors — they're already known
- Detection cost is zero — a few field comparisons per turn

---

## Warm Tier: Incremental Update Logic

### After Each Turn

When a turn completes (assistant response fully streamed, all tool calls resolved), the warm tier updater runs. Every field it reads is a structured data field — no natural language parsing.

```rust
fn update_warm_tier(
    warm_tier: &mut SessionMemoryDocument,
    turn: &ConversationTurn,
    turn_index: usize,
    anchor: Option<&AnchorPoint>,
) {
    // 1. Update Current State (from structured tool call data only)
    update_current_state(&mut warm_tier.current_state, turn);

    // 2. Append to File Modification Log
    for tc in &turn.tool_calls {
        if tc.tool == "Edit" || tc.tool == "Write" {
            if let Some(file_path) = tc.file_path() {
                // Check if the tool call succeeded
                let succeeded = turn.tool_results.iter().any(|tr| tr.success);
                warm_tier.file_modifications.push(FileModEntry {
                    turn_index,
                    file_path,
                    operation: if tc.tool == "Write" { "Created/Wrote" } else { "Edited" },
                    succeeded,
                });
            }
        }
    }

    // 3. Append to Tool Failure Log (structural: tool_result.success == false)
    for (i, tr) in turn.tool_results.iter().enumerate() {
        if !tr.success {
            let tool_name = turn.tool_calls.get(i)
                .map(|tc| tc.tool.as_str())
                .unwrap_or("unknown");
            let file_info = turn.tool_calls.get(i)
                .and_then(|tc| tc.file_path());
            warm_tier.tool_failures.push(ToolFailureEntry {
                turn_index,
                tool_name: tool_name.to_string(),
                file_path: file_info,
                resolved_at: None, // Filled in by ErrorResolution anchor detection
            });
        }
    }

    // 4. If ErrorResolution anchor detected, mark recent failures as resolved
    if let Some(anchor) = &anchor {
        if anchor.anchor_type == AnchorType::ErrorResolution {
            for failure in warm_tier.tool_failures.iter_mut().rev() {
                if failure.resolved_at.is_none() {
                    failure.resolved_at = Some(turn_index);
                    break; // Resolve the most recent unresolved failure
                }
            }
        }
    }

    // 5. Append fspec milestone (if anchor detected)
    if let Some(anchor) = anchor {
        warm_tier.milestones.push(MilestoneEntry {
            turn_index,
            anchor_type: anchor.anchor_type,
            description: anchor.description.clone(),
        });
    }

    // 6. Prune if necessary
    warm_tier.prune_if_needed();
}

fn update_current_state(state: &mut CurrentState, turn: &ConversationTurn) {
    // Active files: union of files from Edit/Write/Read tool calls in this turn
    for tc in &turn.tool_calls {
        if tc.tool == "Edit" || tc.tool == "Write" || tc.tool == "Read" {
            if let Some(file_path) = tc.file_path() {
                if !state.active_files.contains(&file_path) {
                    state.active_files.push(file_path);
                }
            }
        }
    }

    // Cap active files at 10 most recent
    if state.active_files.len() > 10 {
        let drain_count = state.active_files.len() - 10;
        state.active_files.drain(..drain_count);
    }

    // Work unit: extracted from fspec tool calls if present
    for tc in &turn.tool_calls {
        if tc.tool == "Fspec" {
            if let Some(args) = tc.parameters.get("args").and_then(|v| v.as_str()) {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(args) {
                    if let Some(positional) = parsed.get("_").and_then(|v| v.as_array()) {
                        if let Some(first) = positional.first().and_then(|v| v.as_str()) {
                            // Heuristic-free: only set if it looks like a work unit ID
                            // (uppercase letters + dash + digits, e.g., AUTH-001)
                            if is_work_unit_id(first) {
                                state.work_unit = Some(first.to_string());
                            }
                        }
                    }
                }
            }
            // Extract status from update-work-unit-status
            if let Some(cmd) = tc.parameters.get("command").and_then(|v| v.as_str()) {
                if cmd == "update-work-unit-status" {
                    if let Some(args) = tc.parameters.get("args").and_then(|v| v.as_str()) {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(args) {
                            if let Some(positional) = parsed.get("_").and_then(|v| v.as_array()) {
                                if let Some(status) = positional.get(1).and_then(|v| v.as_str()) {
                                    state.work_unit_status = Some(status.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Last tool outcome: what was the last tool call and did it succeed
    if let Some(last_tc) = turn.tool_calls.last() {
        let last_success = turn.tool_results.last()
            .map(|tr| tr.success)
            .unwrap_or(false);
        let file_info = last_tc.file_path()
            .map(|f| format!(" on {f}"))
            .unwrap_or_default();
        state.last_tool_outcome = format!(
            "{}{} — {}",
            last_tc.tool,
            file_info,
            if last_success { "succeeded" } else { "failed" },
        );
    }
}

/// Check if a string looks like a work unit ID (e.g., AUTH-001, BUG-042)
fn is_work_unit_id(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    parts.len() == 2
        && parts[0].chars().all(|c| c.is_ascii_uppercase())
        && parts[1].chars().all(|c| c.is_ascii_digit())
}
```

### Data Structures

```rust
struct SessionMemoryDocument {
    current_state: CurrentState,
    file_modifications: Vec<FileModEntry>,
    tool_failures: Vec<ToolFailureEntry>,
    milestones: Vec<MilestoneEntry>,
}

struct CurrentState {
    active_files: Vec<String>,
    work_unit: Option<String>,
    work_unit_status: Option<String>,
    last_tool_outcome: String,
}

struct FileModEntry {
    turn_index: usize,
    file_path: String,
    operation: &'static str, // "Created/Wrote" or "Edited"
    succeeded: bool,
}

struct ToolFailureEntry {
    turn_index: usize,
    tool_name: String,
    file_path: Option<String>,
    resolved_at: Option<usize>, // Turn index where ErrorResolution anchor was detected
}

struct MilestoneEntry {
    turn_index: usize,
    anchor_type: AnchorType,
    description: String,
}
```

### Pruning Strategy

The warm tier must not grow unboundedly. Pruning rules:

- **File Modification Log:** Max 25 entries. When exceeded, remove entries for files NOT in current `active_files` list, oldest first. If still over limit, remove oldest entries regardless.
- **Tool Failure Log:** Max 10 entries. Remove resolved failures first (those with `resolved_at` set), oldest first. Then remove oldest unresolved failures.
- **Milestones:** Max 10 entries. Remove oldest milestones first, keeping the most recent.
- **Active Files:** Max 10 entries. Drop oldest when exceeded (already handled in `update_current_state`).

### Serialization

The warm tier is serialized as a Markdown string and injected as a user message in the context window. This makes it readable to both humans and LLMs. The Markdown format is chosen because:
- LLMs understand Markdown natively
- Sections are clearly delineated
- Turn references `[turn N]` are unambiguous
- It can be parsed back into structured data if needed

### Token Budget for Warm Tier

With all sections at maximum capacity:
- Current State: ~50 tokens
- File Modification Log (25 entries × ~15 tokens): ~375 tokens
- Tool Failure Log (10 entries × ~15 tokens): ~150 tokens
- Milestones (10 entries × ~20 tokens): ~200 tokens
- Section headers + formatting: ~50 tokens

**Total maximum: ~825 tokens.** This is well under the 4000-token budget from the previous design, leaving substantial headroom. The warm tier is lean because it only contains structural facts, not prose.

---

## Compaction Flow: New vs Current

### Current Flow (Expensive)

```
Compaction triggers (90% context threshold)
  ↓
convert_messages_to_turns() — serialize ALL messages into turns
  ↓
AnchorDetector::detect_batch() — ONE LARGE LLM CALL analyzing all turns
  ↓
TurnSelector::select_turns_with_recent() — split into keep/summarize
  ↓
generate_llm_summary() — SECOND LLM CALL for narrative summary (with retry)
  ↓
Reconstruct messages: [system] + [kept turns] + [summary] + [continuation]
  ↓
Clear prompt cache
```

**Cost:** 2+ LLM calls, 10-30 seconds, potentially 50k+ tokens in prompts.

### New Flow (Zero LLM Cost)

```
Compaction triggers (90% context threshold)
  ↓
Identify hot turns to demote (oldest hot turns beyond keep limit)
  ↓
For each demoted turn:
  - Verify warm tier entries exist (should already be there from incremental updates)
  - If any entries missing (e.g., crash recovery), backfill from turn's structured data
  - Remove from message array
  ↓
Re-serialize warm tier document into context
  ↓
Reconstruct messages: [system] + [warm tier message] + [hot turns]
  ↓
Clear prompt cache
```

**Cost:** Zero LLM calls, microseconds, pure data structure manipulation.

### What About the Summary?

The current system generates an LLM summary to preserve context. In the new system, the warm tier IS the summary — but it's structured, incrementally built, and always up to date. It doesn't need to be generated at compaction time because it's been maintained continuously.

The warm tier is deliberately more limited than an LLM summary. It captures WHAT happened (files modified, tools failed/succeeded, fspec milestones reached) but not WHY. The reasoning, decisions, and context live in the hot tier (recent turns, verbatim) and cold tier (full history via SessionSearch). This is a feature, not a bug — the warm tier is reliable precisely because it doesn't try to compress semantic information.

### Handling Edge Cases

**Empty warm tier:** If compaction triggers before any structural events occurred (unusual — would mean a session with no file edits, no tool failures, no fspec commands), the warm tier would be nearly empty. This is acceptable. The hot tier still provides recent context, and SessionSearch provides full history. An empty warm tier is better than a hallucinated one.

**Rapid compaction cycles:** If the context window fills quickly (e.g., large file reads), compaction may trigger multiple times. Each cycle simply rotates hot turns out and the warm tier entries are already present. No cascading LLM calls.

**Session recovery after crash:** The warm tier is derived from structured data that can be reconstructed from persisted messages. On session restore, the warm tier can be rebuilt by replaying tool calls from the persistence layer.

---

## Integration Points with Existing Code

### What Changes

| Component | Current | Proposed |
|-----------|---------|----------|
| `AnchorDetector` (anchor.rs) | LLM-based batch detection | Structural per-turn detection |
| `ContextCompactor` (compactor.rs) | Orchestrates LLM calls | Simple tier rotation |
| `TurnSelector` (selector.rs) | Anchor-based split | Not needed — tiers handle this |
| `execute_compaction` (interactive_helpers.rs) | Full pipeline with LLM | Tier rotation + warm tier serialization |
| `CompactionHook` (compaction_hook.rs) | Triggers compaction | Unchanged — still triggers |
| `TokenTracker` (model.rs) | Token counting | Unchanged |
| `SessionSearch` | Agent tool | Also used internally for warm tier seeding on recovery |
| `BackgroundSession` (session_manager.rs) | Stores anchors post-compaction | Stores anchors incrementally |
| `PreservationContext` (model.rs) | Extracts context with flaky keyword matching | Replaced by warm tier's CurrentState |

### What Stays the Same

- **CompactionHook** — still monitors token usage and triggers compaction
- **TokenTracker** — still tracks effective tokens with cache discount
- **Persistence layer** — still stores all messages and anchors
- **SessionSearch tool** — unchanged, but now has a more important role
- **Anchor UI viewer** (`AnchorView.tsx`) — unchanged, but shows real-time anchors
- **Compaction progress UI** — simplified (no "Analyzing anchors..." phase)

### What Gets Removed

- **LLM anchor detection prompt** — `build_anchor_analysis_prompt()`, `build_batch_anchor_analysis_prompt()`
- **LLM response parsing** — `parse_llm_anchor_response()`, `parse_batch_llm_anchor_response()`, `extract_json_from_response()`
- **LLM summary generation** — `generate_llm_summary()`, `build_summary_prompt()`, `build_state_checkpoint_prompt()`
- **Retry logic** — `RETRY_DELAYS_MS`, `FALLBACK_SUMMARY`
- **Synthetic anchors** — `AnchorPoint::synthetic_checkpoint()` (no longer needed as fallback for LLM failures)
- **PreservationContext** — `extract_goal_from_message()`, `detect_build_status()` (flaky keyword matching)
- **TurnSelector** — entire module (tier rotation replaces anchor-based selection)
- **UserCheckpoint anchor type** — removed (no reliable structural signal)
- **TaskCompletion anchor type** — removed (relied on substring matching test output)

### New Components

1. **`SessionMemoryDocument`** — Rust struct representing the warm tier, with serialization to/from Markdown
2. **`StructuralAnchorDetector`** — Replaces LLM-based `AnchorDetector`, runs per-turn, detects only FeatureMilestone and ErrorResolution
3. **`WarmTierUpdater`** — Logic for appending entries and pruning
4. **`TierCompactor`** — Replaces `ContextCompactor`, handles tier rotation

### Anchor Type Changes

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnchorType {
    /// Error was resolved (previous failure + current modification + current all-success)
    ErrorResolution,
    /// fspec milestone reached (status change, feature creation, etc.)
    FeatureMilestone,
}
```

The `TaskCompletion` and `UserCheckpoint` variants are removed. `TaskCompletion` required substring matching against Bash output ("pass", "PASS", "✓") which produces false positives on "password", "bypass", `DB_PASS`. `UserCheckpoint` required substring matching against user messages which is fundamentally unreliable.

---

## Migration Strategy

### Phase 1: Structural Anchor Detection (Standalone)

Add `StructuralAnchorDetector` alongside the existing LLM detector. Wire it into the stream loop to run after each turn. Store results in the existing `anchor_points` Vec. This can ship independently — it just adds more anchors without changing compaction.

**Scope:** New `StructuralAnchorDetector` struct with `detect_fspec_milestone()` and `detect_error_resolution()`. Called from stream loop after each turn completes. No changes to existing compaction pipeline.

**Validation:** Compare structural anchors against LLM-detected anchors in production for several sessions. The structural detector should produce a subset of LLM anchors (FeatureMilestone and ErrorResolution only), with zero false positives.

### Phase 2: Warm Tier Document

Implement `SessionMemoryDocument` and `WarmTierUpdater`. Wire into the stream loop to update after each turn. Inject the serialized document into the context window as a user message. This runs in parallel with the existing compaction — it adds context without replacing anything.

**Scope:** New `SessionMemoryDocument` struct with Markdown serialization. New `WarmTierUpdater` with incremental append and pruning. Injected as a user message before the first hot turn.

**Validation:** Monitor warm tier token count across sessions. Verify it stays under ~825 tokens at maximum capacity. Verify all entries are traceable to specific structured data fields.

### Phase 3: Replace Compaction Pipeline

Replace `execute_compaction` with the new tier rotation logic. Remove the LLM-based anchor detection and summary generation calls. The warm tier document becomes the sole summary mechanism. The old `ContextCompactor`, `TurnSelector`, and LLM-based `AnchorDetector` can be deprecated.

**Scope:** New `TierCompactor` that rotates hot turns, verifies warm tier entries, and reconstructs the message array. Remove all LLM calls from compaction path. Remove `PreservationContext` and its flaky keyword matching.

**Validation:** Compaction should complete in under 100ms (currently 10-30 seconds). Context window should contain system reminders + warm tier + 5 hot turns after compaction. No LLM calls during compaction.

### Phase 4: Agent Guidance

Update the system prompt to inform the agent about the tiered memory system:
- The warm tier document is always in context and contains turn references
- SessionSearch should be used when the agent needs detail beyond the warm tier
- Turn references like `[turn 12]` can be used with `SessionSearch::show(current, max_turns: 2)` to retrieve full context
- The warm tier contains structural facts only — for reasoning and context, use SessionSearch

### Phase 5: Cleanup

Remove dead code:
- `AnchorDetector` (LLM-based)
- `ContextCompactor` (LLM orchestrator)
- `TurnSelector` (anchor-based selection)
- `PreservationContext` (keyword-based extraction)
- `AnchorType::TaskCompletion`, `AnchorType::UserCheckpoint`
- `AnchorPoint::synthetic_checkpoint()`
- All LLM prompt building and response parsing functions
- `extract_json_from_response()` utility
- Retry and fallback constants

---

## Risk Assessment

### Risk: Fewer Anchor Types Than Before

**Severity:** Low. We go from 4 anchor types (ErrorResolution, TaskCompletion, UserCheckpoint, FeatureMilestone) to 2 (ErrorResolution, FeatureMilestone).

**Analysis:** TaskCompletion was the most common anchor but also the most flaky — it relied on substring matching "pass" in Bash output. Its removal means some "edit + test pass" moments won't be flagged as anchors. However, they ARE still tracked in the warm tier as file modifications and in the hot tier as verbatim turns. The anchor badge was cosmetic for the `/anchors` viewer; compaction doesn't need it because tier rotation doesn't use anchors at all.

UserCheckpoint was the least reliable — substring matching against natural language. Its removal has no practical impact on compaction quality.

### Risk: Warm Tier Misses Important Context

**Severity:** Medium. The warm tier deliberately excludes semantic information (decisions, reasoning, user intent).

**Mitigation:** This is by design. The warm tier provides a structural index ("what files changed, what tools ran, what fspec milestones were reached"). The LLM reading the warm tier in context can infer continuity from this. For deeper context, SessionSearch provides full verbatim history. The agent is explicitly guided to use SessionSearch for anything beyond the warm tier.

**Comparison to current system:** The current LLM summary also misses important context — it just does so silently and unreliably. The warm tier misses it explicitly and predictably, and the cold tier preserves it fully.

### Risk: Warm Tier Grows Too Large

**Severity:** Very Low. With all sections at maximum capacity, the warm tier is ~825 tokens. This is far below the previous design's 4000-token budget. The warm tier is lean because it only contains structured facts, not prose.

**Mitigation:** Pruning is automatic and simple (FIFO within each section, with resolved failures pruned first). The token count is deterministic — can be calculated exactly from entry counts.

### Risk: Agent Doesn't Use SessionSearch When Needed

**Severity:** Medium. If the agent encounters a turn reference like `[turn 12]` but doesn't search for it, it operates with less context than the current system would provide.

**Mitigation:** The warm tier entries are designed to be self-contained for most purposes. Turn references are supplementary — the entry itself contains the key structural fact ("Edited src/auth/validator.ts"). SessionSearch is only needed when the agent needs the full reasoning behind a change, which is rare for continuation work.

### Risk: Turn References Become Stale

**Severity:** Very Low. Turn indices in the warm tier refer to positions in the persisted session history, which is never modified. SessionSearch reads from persistence, not the active message array. So `[turn 12]` always maps to the same historical content.

### Risk: ErrorResolution False Positives

**Severity:** Very Low. The ErrorResolution detector requires three structural conditions simultaneously: (1) previous turn had a tool failure, (2) current turn has a file modification, (3) ALL current turn tool results succeed. The only false positive scenario is when an unrelated failure in the previous turn coincides with unrelated successful edits in the current turn. Even then, the consequence is only that the turn gets slightly higher preservation priority — no data loss or behavioral change.

---

## Token Budget Comparison

### Current System (Post-Compaction)

| Component | Tokens |
|-----------|--------|
| System reminders | ~3,000 |
| Kept turns (15-20 turns × ~500 tokens) | ~7,500-10,000 |
| LLM summary | ~500-1,000 |
| Continuation message | ~50 |
| **Total** | **~11,000-14,000** |

### Proposed System (Post-Compaction)

| Component | Tokens |
|-----------|--------|
| System reminders | ~3,000 |
| Warm tier document | ~400-825 |
| Hot turns (5 turns × ~500 tokens) | ~2,500 |
| **Total** | **~5,900-6,325** |

The proposed system uses **50-55% less context** after compaction, leaving significantly more room for the agent's work. The warm tier is more information-dense than 15-20 verbatim turns because it captures only structural facts, and it's smaller than the previous design's estimate because it doesn't attempt to capture semantic information.

---

## Open Questions

1. **Should the warm tier document be a user message or a system message?** User messages are more natural for Claude to read as context. System messages might be ignored or treated differently by some providers. Recommendation: user message, clearly labeled as `## Session Memory`.

2. **How should the warm tier handle multi-session context?** If the agent is working across related sessions, should the warm tier reference other sessions? SessionSearch already supports cross-session search. Recommendation: keep warm tier single-session only; cross-session context is a cold tier concern.

3. **Should pruning be more sophisticated?** E.g., keeping file modification entries for currently active files regardless of age, or keeping all unresolved failures regardless of count. Recommendation: start simple (FIFO), add sophistication if real-world usage reveals problems.

4. **What about the `/compact` manual command?** Should it still exist? In the new system, compaction is just tier rotation — should `/compact` force a rotation even below the threshold? Recommendation: yes, keep `/compact` as a manual trigger. Users sometimes want to proactively free context space.

5. **Should we add more structural anchor types later?** For example, detecting git checkpoint creation (another tool with structured parameters), or detecting ConnectMCP calls (new capability acquired). Recommendation: yes, but only for tool calls with exact-match structural signals. Never add anchors that require interpreting prose.

6. **Should the warm tier include Read tool entries?** Currently, Read calls update `active_files` but don't get their own File Modification Log entry. Reading a file isn't a modification, but it is a structural signal of interest. Recommendation: no — the File Modification Log should only track mutations. Read-based interest is captured in `active_files` already.
