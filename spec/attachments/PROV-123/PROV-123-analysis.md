# PROV-123 — Active-session model switch does not update the global default; new sessions inherit the stale startup model

## User-reported symptom

> "When I start a new agent while one already has a model selected, it creates it
> with the model that was *persisted to JSON* — which seems to persist after
> fspec closes down, not when it is selected."

Diagnosis: persistence at selection time **does** work (PROV-122). What is broken
is that a **new session created in the same running process** does not pick up the
model you just selected in the active session — it reads a stale in-memory default
that only gets refreshed at the *next* startup. That is why it feels
shutdown-driven.

---

## The two sources of truth (Rust) and how they drift

| Source | Read by | Written by |
|---|---|---|
| `SessionManager.default_model: RwLock<Option<String>>` | **new session creation** (`get_default_model`) | `SessionManager::new` (from disk), bootstrap, `create_session_with_id` |
| `fspec-config.json` `tui.lastUsedModel` (disk) | **bootstrap only** (`load_persisted_model_string`) | `set_model` (PROV-122), `set_default_model` (PROV-122) |

The active-session switch writes the **disk** source but not the **in-memory**
source that new sessions read. They drift for the lifetime of the process.

### Exact code

**New session reads the in-memory RwLock — not disk, not the active session:**

`codelet/sessions/src/handle_impl.rs`
```rust
fn create_session(&self, role: Option<String>) -> SessionId {
    // ...
    let Some(model) = self.get_default_model() else {        // line 89 — RwLock read
        // PROV-101 decline (empty SessionId)
        return SessionId::new(String::new());
    };
    // ... SessionManager::create_session(self, &model, &project)
}

fn create_isolated_session(&self, role: Option<String>) -> Result<IsolatedSessionInfo, String> {
    // ...
    let model = self.get_default_model().ok_or_else(|| /* decline */)?;  // line 822 — RwLock read
    // ...
}
```

`codelet/sessions/src/session_manager.rs`
```rust
pub fn get_default_model(&self) -> Option<String> {     // line 258 — clones RwLock, no disk read
    self.default_model.read()...clone()
}
```

**Active-session switch updates only that session + disk, NOT the RwLock:**

`codelet/sessions/src/handle_impl.rs::set_model` (≈ 999–1069)
```rust
session.set_model(Some(provider_id.into()), Some(model_id.into())); // line 1050 — this session only
session.set_model_limits(/* ... */);                                // line 1051
// PROV-122: persist to disk tui.lastUsedModel — but does NOT touch default_model RwLock
if let Err(e) = crate::last_used_model_persistence::save_persisted_model_string(&model) { // line 1061
    tracing::warn!(...);
}
Ok(())
```

There is **no** `self.set_default_model(...)` call in `set_model`, so
`get_default_model()` keeps returning the bootstrap value.

**The RwLock is populated at exactly three places:**
1. `SessionManager::new()` → `load_default_model()` from `default-model.json` (`session_manager.rs:188`).
2. TUI bootstrap → `set_default_model(resolved)` from `tui.lastUsedModel` (`fspec-tui/src/app/bootstrap.rs:67–84`).
3. Non-isolated `create_session_with_id` → `self.set_default_model(model)` (`session_manager.rs:638`). (The **isolated** path at `:909` does NOT — only `set_active_session`.)

None of these fire on an active-session `set_model`.

---

## The failing timeline

1. Start fspec → bootstrap sets `default_model = X` (last persisted model).
2. Session A created → A uses `X`.
3. User selects **Y** in session A → A switches to Y; disk `tui.lastUsedModel = Y`;
   **`default_model` RwLock still = X.**
4. User creates session B → `create_session` reads `get_default_model() = X` →
   **B gets X (stale), not Y.** ❌
5. Only a process restart shows Y (bootstrap re-reads `tui.lastUsedModel`).

---

## TypeScript reference (the correct behavior)

There is **one** global `modelStore.currentModel` (Zustand) — no per-session TUI
model. `selectModel()` (`src/tui/services/modelSelectionService.ts`) **always**
updates it on success:

```ts
// Step 3 — Always update Zustand store on success
//          (keeps store in sync for new sessions).
onSetCurrentModel?.(selection);   // -> modelStore.setCurrentModel (global)
// ... then writeConfig persists tui.lastUsedModel
```

New session creation (`AgentView.handleCreateSession`) reads that
**just-updated** `currentModel` and passes it to `createSession`. So select Y in
A → global store = Y → new session B inherits **Y**. The in-memory global and the
persisted value never drift.

---

## Proposed fix (TS-parity)

In `handle_impl.rs::set_model`, after the successful in-memory switch, **also
update the global default** so new sessions inherit it:

```rust
session.set_model(Some(provider_id.into()), Some(model_id.into()));
session.set_model_limits(/* ... */);

// PROV-123: keep the global default in sync so a NEW session created in this
// same process inherits the just-selected model (TS-parity with
// modelSelectionService "keeps store in sync for new sessions").
// set_default_model updates the RwLock AND persists default-model.json +
// tui.lastUsedModel, so it SUPERSEDES the standalone PROV-122 persist call.
SessionManager::set_default_model(self, &model);
Ok(())
```

- This **replaces** the standalone `save_persisted_model_string(&model)` line 1061
  (now redundant — `set_default_model` already writes `tui.lastUsedModel` per
  PROV-122). Net: one source of truth.
- `set_default_model` is already best-effort/non-fatal on its disk writes and
  ignores empty strings (PROV-101 invariant preserved).

### Model-string format note (verify)
`set_model` builds `model = format!("{provider_id}/{model_id}")`. Confirm this is
the same shape `get_default_model` consumers / `create_session` expect, including
**profile-qualified** ids (`openai:<profile>/<model>`). If `set_model` does not
currently produce the profile-qualified form for profile selections, the global
default must still store a string that a new session can resolve identically — add
a test that selects a profile model in an active session, creates a new session,
and asserts the new session resolves the same profile model.

---

## Scope / non-goals

- **In scope:** `set_model` updates the global `default_model` (RwLock + disk via
  `set_default_model`); regression test proving a new session created after an
  active-session switch inherits the new model; round-trip/format test for
  profile-qualified ids; empty-string still a no-op.
- **Out of scope / intentional:** the **isolated** create path NOT writing the
  global default (`session_manager.rs:909`) is correct — isolated sessions are
  ephemeral and must not mutate the global default. Do NOT change that.
- **Invariants preserved:** PROV-101 (no empty/whitespace persisted, no hardcoded
  fallback); PROV-120/122 read+write paths unchanged in contract.

## Affected files
- `codelet/sessions/src/handle_impl.rs` — `set_model` (≈ 1050–1061): add
  `set_default_model` call, remove now-redundant standalone persist.
- New/updated test file (1:1 with the new feature file), e.g.
  `codelet/sessions/tests/prov123_active_selection_updates_default.rs`. Use a
  clean temp HOME / FSPEC_USER_DIR.

## Expected behavior after fix
Select Y in active session A → `get_default_model()` returns Y → a new session B
created in the same process uses **Y** (no restart required), matching TS.
