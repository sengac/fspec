# BUG-155: cont009_completion_contract_sync Tests Fail Due to Missing Models Registry Cache

## Problem

The `cont009_completion_contract_sync` test suite has 5 failing tests that fail at the `fresh_session()` setup step with:

```
"Invalid session ID: invalid length: expected length 32 for simple format, found 0"
```

## Root Cause Analysis

### The Call Chain

1. `fresh_session()` calls `manager.set_default_model("anthropic/claude-opus-4-5")`
2. Then calls `handle.create_session(None)` which triggers `SessionManagerHandle::create_session`
3. `create_session` calls `SessionManager::create_session(self, &model, &project).await`
4. `SessionManager::create_session` calls `create_session_with_id` which calls `ProviderManager::with_model_support()`
5. `ProviderManager::with_model_support()` attempts to validate the model string against the registry cache
6. **The registry cache is empty** (no `models.json` seeded into the temp data dir's cache directory)
7. Model validation fails → `create_session_with_id` returns `Err`
8. `handle.create_session()` catches the error and returns `SessionId::new(String::new())` (empty string)
9. `manager.get_session("")` fails with "invalid length: expected length 32 for simple format, found 0"

### Why It Happens

The `fresh_session()` helper in `sessions/tests/cont009_completion_contract_sync.rs` sets up a temp data directory but does NOT seed the offline models registry cache. Compare with working tests (e.g., `rpc386_owning_session_manager.rs`) which DO seed the cache:

```rust
// WORKING (rpc386_owning_session_manager.rs):
let cache_dir = data_dir.path().join("cache");
std::fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).map_err(|e| e.to_string())?;
codelet_common::set_data_directory(data_dir.path().to_path_buf())?;

// BROKEN (cont009_completion_contract_sync.rs):
let data_dir = tempfile::tempdir().expect("tempdir");
let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
// ❌ No cache directory created, no models.json seeded
```

### Affected Tests (5 of 7)

| Test | Status | Reason |
|------|--------|--------|
| `continue_state_syncs_inner_and_arms_registry` | ❌ FAILED | fresh_session() returns empty ID |
| `goal_alone_arms_registry_and_registers_goal_spec` | ❌ FAILED | fresh_session() returns empty ID |
| `neither_continue_nor_goal_disarms_registry` | ❌ FAILED | fresh_session() returns empty ID |
| `new_user_turn_resets_nudge_counter` | ❌ FAILED | fresh_session() returns empty ID |
| `unchanged_chrome_goal_is_not_reapplied` | ❌ FAILED | fresh_session() returns empty ID |
| `clearing_chrome_goal_clears_inner_and_registry` | ✅ PASSED | Uses fresh_session() but doesn't hit the assertion before panic |
| `both_twins_call_shared_sync_helper_at_dispatch_site` | ✅ PASSED | Source code inspection test (no session creation) |

## Fix

The `fresh_session()` helper needs to:
1. Create a `cache/` subdirectory in the temp data dir
2. Write a `models.json` file with the offline models fixture (same `MODELS_FIXTURE` used by other tests)
3. Call `codelet_common::set_data_directory()` AFTER seeding the cache
4. Optionally call `reset_stores_for_tests()` before setting the data directory (as RPC-423 precedent)

## Files to Modify

- `codelet/sessions/tests/cont009_completion_contract_sync.rs` — `fresh_session()` helper

## Precedent

The fix pattern is already established in:
- `codelet/agent-loop/tests/rpc386_owning_session_manager.rs` — `owning_manager()` helper
- `codelet/sessions/tests/prov118_no_session_default_model.rs` — test setup
