# Fixes Required from ACDD Compliance Reviews

Generated from parallel review agents for **RPC-424** (model parsing helper) and **RPC-425** (session creation helper).

---

## 🔴 Critical Issues (Must Fix)

### 1. DRY Violation: `model_resolution.rs` Duplicates `parse_model_string` Logic

**Work Unit:** RPC-424  
**Severity:** 🔴 Critical  
**Files:** `codelet/sessions/src/model_resolution.rs` (lines 32–63), `codelet/sessions/src/model_parsing.rs` (lines 58–106)

**Problem:**  
The `apply_model_selection` function in `model_resolution.rs` contains an almost identical copy of the model string parsing logic that `parse_model_string` in `model_parsing.rs` already provides. Both files independently implement:

| Check | model_resolution.rs | model_parsing.rs |
|-------|---------------------|------------------|
| Empty/slash validation | Line 32 | Line 60 |
| Profile detection | Line 39 | Line 67 |
| Codex detection | Line 40 | Line 68 |
| Provider/model extraction | Lines 42–53 | Lines 70–85 |
| Empty-part validation | Lines 55–59 | Lines 88–93 |
| Custom model check | Lines 61–63 | Lines 95–97 |

This defeats the entire purpose of RPC-424 — the goal was to have **one** source of truth for model parsing, but `model_resolution.rs` still has its own copy.

**Fix:**  
Replace the inline parsing block in `apply_model_selection` (lines 32–63) with a call to `parse_model_string`:

```rust
// BEFORE (model_resolution.rs:32-63):
if model.is_empty() || !model.contains('/') {
    return Err(format!(...));
}
let is_profile_model = model.contains(':') && model.find(':') < model.find('/');
let is_codex_model = model.starts_with("codex/");
let (registry_provider, model_part) = if is_profile_model { ... } else { ... };
if registry_provider.is_empty() || model_part.is_empty() { ... }
let is_custom_model = !is_profile_model && !is_codex_model && ...;

// AFTER:
let parsed = crate::model_parsing::parse_model_string(model)?;
let registry_provider = parsed.registry_provider;
let model_part = parsed.model_part;
let is_profile_model = parsed.is_profile_model;
let is_codex_model = parsed.is_codex_model;
let is_custom_model = parsed.is_custom_model;
```

### 2. Semantic Divergence Between the Two Parsers

**Work Unit:** RPC-424  
**Severity:** 🔴 Critical  
**Files:** `codelet/sessions/src/model_resolution.rs` (line 49), `codelet/sessions/src/model_parsing.rs` (line 79)

**Problem:**  
For profile models, the two parsers extract `registry_provider` differently:

- **model_resolution.rs:49** — Extracts text *before* the colon: `&model[..colon_idx]` → returns `"profile"` for input `"profile:anthropic/claude-opus-4"`
- **model_parsing.rs:79** — Extracts text *between* colon and slash: `&model[colon_idx + 1..slash_idx]` → returns `"anthropic"` for input `"profile:anthropic/claude-opus-4"`

This means the two modules interpret the same input differently. The `model_resolution.rs` version is **wrong** — it passes `"profile"` as the registry provider instead of `"anthropic"`, which would cause `set_model_direct` to fail or route to the wrong provider.

**Fix:**  
Once Fix #1 is applied (using `parse_model_string`), this divergence is automatically resolved since `parse_model_string` extracts the correct provider (`"anthropic"`).

### 3. Post-Helper Code Duplication in Session Creation

**Work Unit:** RPC-425  
**Severity:** 🔴 Critical  
**Files:** `codelet/sessions/src/session_manager.rs` (lines 678–722 vs 842–876)

**Problem:**  
After calling `create_background_session_inner`, both `create_session_with_id` and `create_session_from_manifest` have ~40 lines of nearly identical post-processing code:

| Step | create_session_with_id | create_session_from_manifest |
|------|----------------------|-----------------------------|
| spawn_agent_loop | Line 680–681 | Line 843–844 |
| get_info() | Line 685 | Line 847 |
| insert(uuid, session) | Lines 691–694 | Lines 853–856 |
| session_created_tx.send | Line 700 | Line 859 |
| set_default_model | Line 706 | Line 861 |
| set_active_session | Line 707 | Line 862 |
| maybe_start_scheduler | Line 708 | Line 863 |
| isolation_state_change broadcast | Lines 713–716 | Lines 867–870 |
| spawn_footer_poller | Lines 719–720 | Lines 873–874 |
| broadcast_metadata_update | Line 722 | Line 876 |

The only real difference is `set_owning_manager` (present in `create_session_from_manifest` line 840, absent in `create_session_with_id`).

**Fix:**  
Extract the post-processing into a shared method on `SessionManager`:

```rust
async fn post_session_creation(
    &self,
    uuid: Uuid,
    session: Arc<BackgroundSession>,
    input_rx: mpsc::Receiver<PromptInput>,
    mcp_injection_rx: mpsc::Receiver<McpInjection>,
    model: &str,
    project: &str,
    created_info: SessionInfo,
) {
    session.set_owning_manager(self.self_weak.get().cloned().unwrap_or_default());
    self.hooks().spawn_agent_loop(session.clone(), input_rx, mcp_injection_rx);
    self.sessions.write().expect("lock").insert(uuid, session);
    let _ = self.session_created_tx.send(created_info);
    self.set_default_model(model);
    self.set_active_session(uuid);
    self.maybe_start_scheduler(project);
    // ... isolation_state_change, footer_poller, broadcast_metadata_update
}
```

Both callers would then call this single method after `create_background_session_inner`.

### 4. `set_owning_manager` Inconsistency

**Work Unit:** RPC-425  
**Severity:** 🔴 Critical  
**Files:** `codelet/sessions/src/session_manager.rs`

**Problem:**  
`create_session_from_manifest` calls `session.set_owning_manager(...)` at line 840, but `create_session_with_id` does NOT. This means sessions created via `create_session_with_id` won't have the owning-manager back-reference, while sessions resumed from manifest will. This is a behavioral bug that could cause AgentManager handler binding to fall back to the singleton incorrectly for newly-created sessions.

**Fix:**  
Once Fix #3 is applied (shared post-processing method), `set_owning_manager` is called for ALL session creation paths uniformly.

### 5. Tests Are Static File-Content Checks, Not Behavioral Tests

**Work Unit:** RPC-424 & RPC-425  
**Severity:** 🔴 Critical  
**Files:** 
- `codelet/sessions/src/model_parsing.rs` (lines 236–278)
- `codelet/sessions/tests/rpc425_session_creation_refactor.rs`

**Problem:**  
The "integration" tests in both work units read source files as strings and check for substring presence:

```rust
// model_parsing.rs:236-278
let sm_content = std::fs::read_to_string(...).join("session_manager.rs");
assert!(sm_content.contains("crate::model_parsing::parse_model_string"));
```

These do NOT actually exercise the session creation or model parsing logic. They verify that certain strings exist in the source files, not that the code works correctly. This provides zero runtime confidence.

**Fix:**  
Replace file-content tests with actual behavioral tests:

**For RPC-424 (model parsing):**
The inline unit tests (lines 115–232) ARE behavioral tests and are sufficient. The meta-test `all_three_call_sites_use_shared_helper` should be removed or replaced with a compile-time check.

**For RPC-425 (session creation):**
Replace `rpc425_session_creation_refactor.rs` with integration tests that actually construct sessions through the helper:

```rust
// Instead of reading files, actually call the helper:
#[tokio::test]
async fn helper_creates_background_session_with_correct_fields() {
    let tmp_dir = setup_temp_directory();
    let params = SessionCreationParams { /* ... */ };
    let pm = ProviderManager::with_model_support().await.unwrap();
    let result = create_background_session_inner(params, pm).await.unwrap();
    assert!(result.session.uuid() == expected_uuid);
    assert!(result.session.model_id.read().unwrap() == "claude-sonnet-4");
    // Verify lifecycle hooks loaded, MCP initialized, etc.
}
```

---

## 🟡 Warnings (Should Fix)

### 6. `create_isolated_session_with_id` Doesn't Use the Shared Helper

**Work Unit:** RPC-425  
**Severity:** 🟡 Warning  
**Files:** `codelet/sessions/src/session_manager.rs` (lines 1001–1119)

**Problem:**  
`create_isolated_session_with_id` has its own inline session creation logic that duplicates:
- `Session::from_provider_manager` (line 1004)
- `inject_context_reminders_with_isolation` (line 1017)
- `load_lifecycle_hooks` (line 1019–1030)
- `BackgroundSession::new` (lines 1032–1045)
- `set_base_thinking_level` (line 1061)
- `resolve_compaction_threshold` / `set_model_limits` (lines 1068–1079)
- `register_pre_tool_hook` (lines 1081–1109)
- `init_mcp_session` (line 1119)

This is ~100+ lines of duplicated logic. The helper already supports `worktree_path`, `base_commit`, and `isolation` parameters via `SessionCreationParams`. Only the provider manager creation differs (isolated sessions need a separate provider manager path).

**Fix:**  
Refactor `create_isolated_session_with_id` to use `create_background_session_inner`:

```rust
// Create provider manager with isolation-specific logic (the ONLY difference)
let provider_manager = if is_codex {
    ProviderManager::with_codex_model().await?
} else {
    ProviderManager::with_model_support().await?
};

// Use the shared helper for everything else
let params = SessionCreationParams {
    uuid, name, project, project_path,
    parsed_model, provider_id, model_id,
    worktree_path: Some(worktree_path),
    base_commit: Some(base_commit),
    isolation: Some(isolation_ctx),
    chunks_tx, status_changes_tx,
};
let result = create_background_session_inner(params, provider_manager).await?;
```

### 7. Rule [2] Not Fully Implemented

**Work Unit:** RPC-425  
**Severity:** 🟡 Warning  
**Files:** `spec/features/extract-shared-session-creation.feature`, `codelet/sessions/src/session_creation_helper.rs`

**Problem:**  
The example map Rule [2] states: "The helper must accept a callback or strategy for manifest handling (save vs skip-save)." The current implementation does NOT use a callback or strategy pattern — it simply places `save_session` in one call site and omits it in the other. The helper has no knowledge of manifest handling at all.

**Fix:**  
Either:
- (a) Add a callback parameter to `create_background_session_inner` for manifest handling, OR
- (b) Update Rule [2] to match the actual implementation (caller decides manifest save/skip-save)

Option (b) is recommended since manifest saving is a caller concern, not a session creation concern.

### 8. `session_creation_helper.rs` Exceeds 300 Lines

**Work Unit:** RPC-425  
**Severity:** 🟡 Warning  
**Files:** `codelet/sessions/src/session_creation_helper.rs`

**Problem:**  
The file is 359 lines (including inline tests). The coding standards require files under 300 lines. The inline `#[cfg(test)]` module adds ~78 lines; removing it brings the production code to ~281 lines, which is acceptable.

**Fix:**  
Move inline tests to `codelet/sessions/tests/session_creation_helper_tests.rs` and remove the `#[cfg(test)]` module from the source file.

### 9. Coverage Implementation Line Ranges Are Overly Broad

**Work Unit:** RPC-425  
**Severity:** 🟡 Warning  
**Files:** `spec/features/extract-shared-session-creation.feature.coverage`

**Problem:**  
Scenarios 3 and 4 link to ALL lines 1–278 of `session_creation_helper.rs`. This is not useful for traceability — the ranges should target the specific functions/lines relevant to each scenario.

**Fix:**  
Narrow coverage line ranges:
- Scenario 3 (session setup behavior): Lines 178–271 (lifecycle hooks, BackgroundSession::new, pre-tool hook, MCP init)
- Scenario 4 (model limits): Lines 206–231 (thinking level, compaction threshold, set_model_limits)

### 10. Duplicate Inline Tests

**Work Unit:** RPC-425  
**Severity:** 🟡 Warning  
**Files:** `codelet/sessions/src/session_creation_helper.rs` (lines 280–359), `codelet/sessions/tests/rpc425_session_creation_refactor.rs`

**Problem:**  
`session_creation_helper.rs` contains inline `#[cfg(test)]` tests that duplicate tests already in `rpc425_session_creation_refactor.rs`. Specifically, `shared_helper_preserves_session_behavior` and `model_limits_and_thinking_level_are_set` appear in both locations with nearly identical assertions.

**Fix:**  
Remove inline tests from `session_creation_helper.rs` (also fixes issue #8). Keep tests only in the dedicated test file.

### 11. Two Tests Lack Corresponding Gherkin Scenarios

**Work Unit:** RPC-424  
**Severity:** 🟡 Warning  
**Files:** `codelet/sessions/src/model_parsing.rs` (lines 204–232)

**Problem:**  
The tests `reject_model_string_with_empty_provider` (line 205) and `reject_model_string_with_empty_model_part` (line 221) have `/// Feature:` doc comments referencing the feature file, but there are no matching scenarios in `extract-model-parsing-helper.feature`.

**Fix:**  
Either:
- (a) Add scenarios for these edge cases to the feature file, OR
- (b) Remove the `/// Feature:` doc comments from these tests

Option (a) is recommended — these are valid edge cases that should be in the spec.

### 12. Coverage Line Range Slightly Off for Integration Test

**Work Unit:** RPC-424  
**Severity:** 🟡 Warning  
**Files:** `spec/features/extract-model-parsing-helper.feature.coverage`

**Problem:**  
The "All three call sites use the shared helper" scenario maps test lines 235–275, but the actual test function spans lines 236–278. Lines 276–278 (closing braces and assertion) are outside the coverage range.

**Fix:**  
Update coverage to lines 236–278.

### 13. `session_manager.rs` Exceeds File Size Limit

**Work Unit:** RPC-424 & RPC-425  
**Severity:** 🟡 Warning  
**Files:** `codelet/sessions/src/session_manager.rs`

**Problem:**  
The file is 1,291 lines — well beyond the 300-line project guideline. This is a pre-existing condition, but the extraction work (RPC-424, RPC-425) should have been paired with a reduction of the caller sites.

**Fix:**  
After applying fixes #1, #3, and #6, the three session creation functions will be significantly shorter. Consider splitting `session_manager.rs` into:
- `session_manager.rs` — SessionManager struct and high-level orchestration
- `session_creation.rs` — Session creation logic (moved from helper)
- `session_lifecycle.rs` — Agent loop spawning, footer polling, metadata broadcast

### 14. `unwrap_or("")` Fragile Pattern

**Work Unit:** RPC-424  
**Severity:** 🟡 Warning  
**Files:** `codelet/sessions/src/model_parsing.rs` (line 84)

**Problem:**  
`parts.get(1).copied().unwrap_or("")` silently defaults to empty string when there's no second part. The `splitn(2, '/')` call already guarantees exactly 2 parts when `'/'` exists (validated on line 60), so `parts[1]` would never panic. The `unwrap_or("")` is unnecessary and masks the invariant.

**Fix:**  
Replace with direct indexing since the invariant is guaranteed:

```rust
// BEFORE:
let parts: Vec<&str> = model.splitn(2, '/').collect();
(parts[0], parts.get(1).copied().unwrap_or(""))

// AFTER:
let parts: Vec<&str> = model.splitn(2, '/').collect();
(parts[0], parts[1])  // Safe: splitn(2) always returns exactly 2 parts when '/' exists
```

---

## Priority Order

1. **Fix #1 + #2** (DRY violation + semantic divergence) — These are the same fix; replacing the inline parser with `parse_model_string` resolves both
2. **Fix #4** (`set_owning_manager` inconsistency) — Quick fix, just add the missing call
3. **Fix #3** (post-helper duplication) — Extract shared post-processing method
4. **Fix #5** (static file-content tests) — Replace with behavioral tests
5. **Fix #6** (isolated session doesn't use helper) — Major refactoring, do after #3
6. **Fix #7–#14** — Warnings, address during code review of above fixes
