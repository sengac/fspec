# Research: DeepSearch & AgentManager Model Inheritance

**Date:** 2026-04-17
**Investigator:** AI Agent (BUG-125 research)
**Scope:** `codelet/napi/src/session_manager.rs`, `codelet/napi/src/deep_search_handler.rs`, `codelet/napi/src/agent_manager_handler.rs`

---

## Executive Summary

Both **AgentManager** and **DeepSearch** correctly inherit the parent/supervisor session's model at handler registration time — neither falls back to any "default model" from fspec config or elsewhere. However, a **stale capture bug** exists: if the user changes their model mid-session, DeepSearch and AgentManager handlers continue using the model that was active when the session was first created. Additionally, DeepSearch does not respect `facade_override` for MODEL-004 custom models.

---

## 1. How Model Inheritance Works Today

### AgentManager (spawn subordinate sessions)

**Registration** (`session_manager.rs:4978-4996`):
```rust
// AMGR-013: Use selected_model_string() which preserves the original
// "provider/model" registry format (e.g. "anthropic/claude-opus-4-6")
let full_model_string = inner_session.provider_manager().selected_model_string()
    .map(|s| s.to_string());
let spawner_context_window = inner_session.provider_manager().raw_model_context_window();
let spawner_max_output = inner_session.provider_manager().raw_model_max_output_tokens();
let agent_manager_handler = crate::agent_manager_handler::create_handler(
    session.project.clone(),
    full_model_string,          // ← captured ONCE
    spawner_context_window,     // ← captured ONCE
    spawner_max_output,         // ← captured ONCE
);
codelet_tools::set_agent_manager_handler(session.id, Some(agent_manager_handler));
```

**Spawn** (`agent_manager_handler.rs:92-114`): The captured `model_string` is passed directly to `create_session_with_id()`. If `None`, spawn fails with `"Cannot spawn: no model configured on spawner session"`.

**Verdict:** ✅ Inherits from parent, not config default. ❌ But value is stale if model changes.

### DeepSearch (ephemeral sub-agents)

**Registration** (`session_manager.rs:4946-4976`):
```rust
// BUG-102: Capture provider and model from current session
let deep_search_provider = inner_session.current_provider_name().to_string();
let deep_search_model = inner_session.current_model_id().map(|s| s.to_string());
let deep_search_context_window = inner_session.provider_manager().raw_model_context_window();
let deep_search_max_output = inner_session.provider_manager().raw_model_max_output_tokens();
```

These captured values are moved into the handler closure and passed to `execute_deep_search()` → `build_and_run_agent()` → `ProviderManager::with_provider_and_model()`.

**Verdict:** ✅ Inherits from parent, not config default. ❌ But value is stale if model changes. ❌ Does not check `facade_override()`.

---

## 2. The Stale Capture Bug

### What happens when the user changes models mid-session

The `session_set_model()` function (`session_manager.rs:6659-6725`) updates:
- Session metadata (`set_model()`)
- Inner `ProviderManager` (via `select_model()` or `set_model_direct()`)
- Cached model limits (`set_model_limits()`)

But it does **NOT** re-register:
- `set_deep_search_handler()` — never called after initial registration
- `set_agent_manager_handler()` — never called after initial registration

Evidence: grep for `set_deep_search_handler` and `set_agent_manager_handler` shows only two call sites each:
1. Initial registration during session creation (lines 4976, 4996)
2. Cleanup on session destroy (lines 5514, 5515 — sets to `None`)

### The agent_loop is NOT affected

The main agent_loop dynamically reads the current model on **every iteration** (`session_manager.rs:4739-4751`):
```rust
let (current_provider, current_model) = {
    let inner = session.inner.lock().await;
    let provider = inner.provider_manager()
        .facade_override()
        .map(|s| s.to_string())
        .unwrap_or_else(|| inner.current_provider_name().to_string());
    let model = inner.current_model_id().map(|s| s.to_string());
    (provider, model)
};
```

This means the main LLM calls always use the **current** model, but sub-agents spawned via DeepSearch or AgentManager use the **original** model from session creation.

### Reproduction scenario

1. User creates session with `anthropic/claude-sonnet-4-20250514`
2. Handler closures capture `claude` + `claude-sonnet-4-20250514`
3. User switches to `google/gemini-2.5-pro` via TUI model selector
4. `session_set_model()` updates the ProviderManager ✅
5. Main agent_loop now uses Gemini ✅
6. User invokes DeepSearch → sub-agent still uses Claude Sonnet ❌
7. User invokes AgentManager spawn → subordinate still uses Claude Sonnet ❌

---

## 3. The facade_override Gap (MODEL-004)

For custom models registered with `facade_override` (e.g., a model registered under `openai` provider but routed to `claude` backend):

- **agent_loop** (line 4744): Checks `facade_override()` first ✅
- **DeepSearch handler** (line 4953): Uses `current_provider_name()` directly, ignoring facade_override ❌

This means DeepSearch would try to use the nominal provider (e.g., OpenAI) instead of the actual backend (e.g., Claude), causing API call failures.

AgentManager is **not** affected because it uses `selected_model_string()` which is in registry format, and `create_session_with_id()` resolves the model through the full registry path including any facade overrides.

---

## 4. What `default_model` Is Actually For

The `default_model` field on `SessionManager` (`session_manager.rs:3154`) is only used for:
- **Bridge session creators** (`init_bridge_session_and_terminal_creators()`, line 6206) — when the dashboard "New Session" button is clicked
- **Scheduled sessions** (SCHED-004) — when cron jobs need to spawn sessions without a user-selected model

It is **never** referenced by AgentManager or DeepSearch handlers. This is correct behavior.

---

## 5. Historical Context

These exact inheritance mechanisms were built by previous bug fixes:

| Work Unit | What it fixed | Status |
|-----------|--------------|--------|
| **BUG-102** | DeepSearch sub-agent "Model is required" error → now captures provider/model from parent | ✅ done |
| **BUG-104** | DeepSearch Codex streaming requirement → added streaming path | ✅ done |
| **AMGR-013** | AgentManager `current_provider_name()` vs registry format mismatch → now uses `selected_model_string()` | ✅ done |
| **MODEL-005** | Per-model context window and max output tokens inheritance | ✅ done |

None of these addressed the stale capture issue because at the time, mid-session model switching may not have been a supported feature.

---

## 6. Recommended Fix

### Option A: Re-register handlers on model change (Recommended)

In `session_set_model()` and `session_set_model_profile()`, after successfully updating the ProviderManager, re-register both handlers with the new model values:

```rust
// After model change succeeds:
// 1. Re-register DeepSearch handler with new provider/model
let new_provider = inner.current_provider_name().to_string();
let new_model = inner.current_model_id().map(|s| s.to_string());
let new_context_window = inner.provider_manager().raw_model_context_window();
let new_max_output = inner.provider_manager().raw_model_max_output_tokens();
// Build and register new DeepSearch handler closure...

// 2. Re-register AgentManager handler with new model string
let new_model_string = inner.provider_manager().selected_model_string()
    .map(|s| s.to_string());
// Build and register new AgentManager handler...
```

### Option B: Dynamic resolution via session lookup

Instead of capturing values in closures, have handlers look up the current model from the session at invocation time. This requires passing the session ID into the handler and looking up the session/inner lock each time.

**Pros:** Always current, no re-registration needed.
**Cons:** Requires locking session inner on every DeepSearch/AgentManager call, more complex error handling.

### facade_override fix

For DeepSearch, change line 4953 from:
```rust
let deep_search_provider = inner_session.current_provider_name().to_string();
```
to:
```rust
let deep_search_provider = inner_session.provider_manager()
    .facade_override()
    .map(|s| s.to_string())
    .unwrap_or_else(|| inner_session.current_provider_name().to_string());
```

This mirrors the agent_loop's pattern at line 4744-4747.

---

## 7. Impact Assessment

| Scenario | Severity | Frequency |
|----------|----------|-----------|
| User never changes model mid-session | No impact | Most common |
| User changes model, then uses DeepSearch | **High** — sub-agent uses wrong model, user confusion | Moderate |
| User changes model, then spawns via AgentManager | **High** — subordinate uses wrong model | Moderate |
| Custom model with facade_override + DeepSearch | **High** — API calls to wrong backend, likely failure | Low (MODEL-004 niche) |

The bug is silent — there's no error, the sub-agents just use the old model without any warning.
