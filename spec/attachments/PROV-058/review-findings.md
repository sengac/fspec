# Epic Review: PROV-058 — Add prompt caching for GitHub Copilot provider connections

**Date:** 2026-04-10
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1

## Summary
- 🔴 Critical: 1 issue (dead code + needless allocation — fixed)
- 🟡 Warnings: 4 issues (DRY violation, file over 300 lines, `unwrap_or` fallback, missing response-side doc — all fixed)
- 🟢 Observations: 2 (strategy differs from opencode; rig-core handles cached_tokens transparently)

## Work Unit Results

### PROV-058: Add prompt caching for GitHub Copilot provider connections — FAIL→PASS (after fixes)

#### 🔴 Critical Issues Found
1. **Dead code: `let _ = model_id;` on line 47 of `prompt_cache.rs`** — `model_id` was allocated as `String` then immediately discarded. The eligibility check already happened in the `match` guard. Wasted allocation + misleading code.

#### 🟡 Warnings Found
1. **DRY violation: `model_id.starts_with("claude-")` duplicated** — Same prefix check existed in `prompt_cache.rs:28` and `behavior_facade.rs:151`. Both independently determining "is this a Claude model?"
2. **`prompt_cache.rs` was 332 lines — exceeded 300-line limit** — 90 production + 242 test lines in single file.
3. **`unwrap_or(body)` fallback in `classify_and_cache_body`** — If `serde_json::to_vec` failed after mutation, silently fell back to pre-mutation body. Should use `expect()`.
4. **No documentation that cached_tokens is handled by rig-core** — Rule [5] says cached_tokens should be propagated, but no code or doc in copilot module explains this is handled transparently by rig-core's OpenAI parser.

#### 🟢 Observations
1. **Caching strategy intentionally differs from opencode** — fspec follows Copilot CLI (1 system msg + last tool + last non-user msg); opencode caches up to 2 system msgs + last 2 non-system msgs but no tools. This is documented in the research attachment.
2. **Good test coverage for request-side injection** — All 5 Gherkin scenarios had correctly mapped tests with exact @step comment matching.

## Fix Results

### PROV-058: Add prompt caching for GitHub Copilot provider connections
- 🔴 Dead code `let _ = model_id;` → ✅ Fixed: Refactored `inject_cache_control` to use `is_some_and(is_claude_model)` — no String allocation, no unused variable.
- 🟡 DRY violation (duplicated prefix checks) → ✅ Fixed: Extracted `copilot/model_family.rs` with shared `is_claude_model()`, `is_gpt_model()`, `is_gemini_model()` utilities. Both `prompt_cache.rs` and `behavior_facade.rs` now use the shared functions.
- 🟡 File over 300 lines → ✅ Fixed: Extracted tests to `copilot/prompt_cache_tests.rs` (227 lines). Production code now 118 lines.
- 🟡 `unwrap_or(body)` fallback → ✅ Fixed: Changed to `expect("serde_json::to_vec on a Value should never fail")` in `refreshing_client.rs`.
- 🟡 Missing cached_tokens documentation → ✅ Fixed: Added "Response-side cached token tracking" section to `prompt_cache.rs` module doc explaining rig-core handles it. Also added to feature file architecture docstring.
- Coverage links updated to point to correct files and line ranges after refactoring.

### New Files Created
- `codelet/providers/src/copilot/model_family.rs` (70 lines) — Shared model family detection utilities with 3 functions + tests
- `codelet/providers/src/copilot/prompt_cache_tests.rs` (227 lines) — Extracted test module

### Files Modified
- `codelet/providers/src/copilot/prompt_cache.rs` — Refactored from 332 → 118 lines. Decomposed monolithic `inject_cache_control` into 4 focused functions: `inject_cache_control`, `tag_first_system_message`, `tag_last_non_user_message`, `tag_last_tool`.
- `codelet/providers/src/copilot/behavior_facade.rs` — Now uses `model_family::is_claude_model` etc. instead of inline `starts_with`.
- `codelet/providers/src/copilot/mod.rs` — Added `model_family` module declaration and re-exports.
- `codelet/providers/src/copilot/refreshing_client.rs` — Changed `unwrap_or(body)` to `expect()`.
- `spec/features/copilot-prompt-caching.feature` — Fixed trailing blank line validation error, added cached_tokens note to architecture docstring.

## Final Verification
- All copilot tests pass: ✅ (91 passed, 0 failed)
- Feature file valid: ✅
- Coverage 100%: ✅ (5/5 scenarios linked)
- All files under 300 lines: ✅
- No DRY violations: ✅
- No dead code: ✅
