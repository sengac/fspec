# Epic Review: HOOK-013 — Agent Lifecycle Hooks — Extend fspec-hooks.json with Rust Agent Core Events

**Date:** 2026-03-20
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 4 (HOOK-014, HOOK-015, HOOK-016, HOOK-017)

## Summary
- 🔴 Critical: 2 issues across 1 work unit (HOOK-014)
- 🟡 Warnings: 10 issues across 4 work units
- 🟢 Observations: ~15 across all work units

## Work Unit Results

---

### HOOK-014: Hook Config Data Model — Loading, Merging & Compilation — ⚠️ WARN

#### 🔴 Critical Issues (Must Fix)
1. **Silent error swallowing via `unwrap_or_default()` in `loader.rs` (lines 173, 197).** Both `extract_definitions()` and `extract_groups()` call `serde_json::from_value(...).unwrap_or_default()`, which silently discards deserialization errors when a hook event entry has the wrong JSON shape. If a user writes `"session_start": {"name": "setup", "command": "./setup.sh"}` (object, not array), their hook is silently ignored with zero error. **Fix:** Return `Result<Vec<...>>` and propagate errors with `.with_context()`.
2. **Code is not reachable from the application.** The entire `lifecycle_hooks` module is not wired into the actual application. No consumer in `codelet/napi/`, `codelet/cli/`, or `codelet/providers/` imports or calls anything from `lifecycle_hooks`. **Note:** This is BY DESIGN — HOOK-013's description explicitly states this story builds the engine, and future work units will integrate it into the agent loop. This is NOT actually a critical issue — the reviewer flagged it but it's expected.

#### 🟡 Warnings (Should Fix)
1. Rule[6] `global.shell` has no Gherkin scenario (only test assertions).
2. Rule[7] "No conditions on agent hooks" has no explicit scenario.
3. No test for malformed event entry shapes (related to Critical #1).

#### 🟢 Observations
- Two bonus tests beyond scenarios (defensive testing)
- All files well within 300-line limit
- No unwrap()/todo!()/unimplemented!() in production code
- @step comments are character-for-character exact matches

#### Coverage: 13/13 scenarios covered (100%)

---

### HOOK-015: Hook Execution Engine & Output Interpretation — ✅ PASS

#### 🔴 Critical Issues: None

#### 🟡 Warnings (Should Fix)
1. **`#[allow(dead_code)]` on `suppress_output` field** (`response.rs:17-18`) — field is deserialized but never read. Should have a doc comment explaining why.
2. **`event_name_from_payload` re-parses JSON** (`executor.rs:120-135`) — wasteful; event name is known at call site.
3. **`interpret_pre_tool_result` doesn't propagate `reason` for JSON-based decisions** — fragile split between `interpret_pre_tool_result` and `extract_reason()`.

#### 🟢 Observations
- `unwrap_or_default()` for JSON serialization (benign — simple Serialize structs)
- All files well under 300 lines
- Excellent integration test approach (real child processes, not mocks)
- Clean module architecture

#### Coverage: 19/19 scenarios covered (100%)

---

### HOOK-016: Session & Notification Lifecycle Integration — ✅ PASS

#### 🔴 Critical Issues: None

#### 🟡 Warnings (Should Fix)
1. **No production callers for any engine function** — `run_session_start`, `run_session_end`, `run_user_prompt`, `run_notification`, `run_pre_tool`, and `run_post_tool` are only called from test files. **Note:** Same as HOOK-014 — BY DESIGN, integration is future work.

#### 🟢 Observations
- All @step text exact matches
- All rules traceable to scenarios
- No unanswered questions

#### Coverage: 6/6 scenarios covered (100%)

---

### HOOK-017: Tool Use Hook Integration (pre/post_tool_use) — ✅ PASS

#### 🔴 Critical Issues: None

#### 🟡 Warnings (Should Fix)
1. **`#[allow(dead_code)]` on `suppress_output` field** (`response.rs:17`) — same as HOOK-015 finding.
2. **`run_pre_tool` / `run_post_tool` are not yet called from the agent loop** — same integration-pending note.
3. **`.unwrap()` in test helper scripts** (test code, not production — noted only).

#### 🟢 Observations
- Clean module decomposition (11 files, all under 300 lines)
- `serde_json::to_string(&payload).unwrap_or_default()` — benign but could be more communicative
- Excellent example map → scenario traceability
- Architecture notes match implementation

#### Coverage: 4/4 scenarios covered (100%)

---

## Cross-Cutting Themes

### 1. Integration Not Wired (Expected — NOT a Bug)
All 4 reviewers flagged that the lifecycle hooks engine has zero call sites in the agent loop. This is **by design** — HOOK-013's description explicitly states this story builds the engine. Future work will wire `run_session_start()`, etc. into `session_manager.rs` and `stream_loop.rs`.

### 2. `unwrap_or_default()` Pattern
Used in `loader.rs` (problematic — silently drops config errors) and `engine.rs`/`tool_engine.rs` (benign — serializing known-good structs). The `loader.rs` usage should be fixed.

### 3. `suppress_output` Dead Code
The `HookJsonResponse.suppress_output` field exists for Claude Code JSON protocol compatibility but is never read. The `#[allow(dead_code)]` should be replaced with a doc comment explaining the rationale.

## Actionable Fixes Required
1. **loader.rs**: Replace `unwrap_or_default()` with proper error propagation in `extract_definitions()` and `extract_groups()`
2. **response.rs**: Add doc comment to `suppress_output` explaining it's a protocol placeholder
3. *(Optional)* executor.rs: Pass event name as parameter instead of re-parsing JSON

## Fix Results

### HOOK-014: loader.rs unwrap_or_default()
- 🔴 Issue 1: Silent error swallowing via `unwrap_or_default()` → ✅ Fixed: `extract_definitions()` and `extract_groups()` now return `Result<Vec<...>>` with `.with_context()` error messages. Callers propagate with `?`.
- 🔴 Issue 2: Code not reachable from app → ⏭️ Not a bug — by design, integration is future work per HOOK-013 description. **Now fully wired into session_manager.rs.**

### HOOK-015: response.rs suppress_output
- 🟡 Issue 1: `#[allow(dead_code)]` with no explanation → ✅ Fixed: Removed the dead field entirely (YAGNI).

### HOOK-013: pre_tool_use wiring (post-review session)
- ✅ All 19 active tool `call()` methods verified with pre_tool_hook check
- ✅ WebSearchTool — was missing hook check, now wired
- ✅ McpToolWrapper — was missing hook check, now wired
- ✅ ConnectMcpTool — was missing hook check, now wired
- ✅ grep.rs `is_context` dead field removed
- ✅ pre_tool_hook.rs `unreachable!()` replaced with direct pattern match
- ✅ git.rs/napi_bindings.rs spurious `#[allow(dead_code)]` removed from NAPI functions
- ✅ Zero `#[allow(dead_code)]` remaining in production code

### All Cards: Build & Test Verification
- All 44 lifecycle hooks tests pass: ✅
- Build succeeds (`cargo test` compiles): ✅
- `cargo clippy -- -D warnings`: ✅ zero warnings
- Feature file valid: ✅
- Tags valid: ✅

## Final Verification
- All tests pass: ✅ (15 + 19 + 6 + 4 + 6 pre_tool_hook = 50)
- Build succeeds: ✅
- Coverage complete: 42/42 (100%) ✅
- Feature files valid: ✅
- Tags valid: ✅
- All 22 `impl Tool for` accounted for: 19 wired + 3 stubs ✅
