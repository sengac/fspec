# PROV-118 — Selecting a model with no active session must set (and persist) the default model

## Problem statement (observed, log-confirmed)

In the full-screen `/model` view, when **no session exists**, pressing Enter on a
model row appears to do nothing: the selector closes but the model is never
applied, and reopening shows no `(current)` marker. The next attempt to use the
agent still fails because there is still no session.

This is a **chicken-and-egg deadlock**:

1. At boot, `create_session` is **declined** because no default model is set
   (PROV-101: no anthropic fallback). → there is **no session**.
2. The `/model` selector therefore opens with `session_id = None`.
3. Selecting a model emits `Action::ModelSelected(None, provider, model)`.
4. `handle_model_selected` sees `session_id = None` and **returns early,
   persisting nothing**.
5. Selector closes. Nothing saved. Reopen → still no model → still no session.

### Definitive runtime trace (`~/.fspec/logs/fspec-combined.log.2026-06-24`)

```
ERROR codelet_sessions::handle_impl: create_session declined: no default model set (PROV-101: no anthropic fallback)
INFO  model_select: Enter: focused row resolved selectable=true provider_key=anthropic model_id=claude-opus-4-8
INFO  model_select: Enter -> EMIT Action::ModelSelected session_id=None provider_key=anthropic model_id=claude-opus-4-8
INFO  model_select: navigator action_tx.send result ok=true
INFO  model_select: handle_model_selected ENTER session_id=None provider_id=anthropic model_id=claude-opus-4-8
WARN  model_select: handle_model_selected: session_id is None -> RETURNING EARLY, model NOT persisted
INFO  model_select: navigator apply_action: closing ModelSelector view -> Agent
```

The Enter→Emit→action-bus→handler dispatch wiring is **100% working** (that was
PROV-117, correctly done). The defect is purely the **no-session persistence
path** inside `handle_model_selected`.

## How TypeScript does it (reference — the correct contract)

Path: `ModelSelectorScreen.tsx` → `AgentView.handleModelSelect` →
`modelSelectionService.selectModel`.

- `modelSelectionService.selectModel` branches on `sessionId`:
  - **Step 2 (`if (sessionId)`):** call NAPI `sessionSetModel` to mutate the
    LIVE session. **Skipped when `sessionId` is null.** This is the ONLY thing
    the session guard gates.
  - **Step 3 (always on success):** update the Zustand `modelStore` via
    `onSetCurrentModel(selection)` — seeds `currentModel` for the NEXT session.
  - **Step 4 (always on success):** persist to config —
    `writeConfig('user', { tui: { lastUsedModel: modelString } })` →
    `~/.fspec/fspec-config.json`.
- TS `create_session` does **NOT** read a "default model" from config. It
  REQUIRES `currentModel` from the store (throws if absent) and passes the model
  explicitly. The `tui.lastUsedModel` key feeds the store **at startup** via
  `modelInitializationService.initializeModels()` (`loadPersistedModelString` →
  `restorePersistedModel`; first-available fallback otherwise).

**Key takeaway:** TS's session guard gates ONLY the live-session write. The
store update + `tui.lastUsedModel` config write happen **unconditionally**. The
Rust port wrongly turned that guard into a total early-return.

### TS reference file map
| Concern | File | Symbol / line |
|---|---|---|
| Selection entry | `src/tui/components/ModelSelectorScreen.tsx` | `:208` `onSelectModel` |
| Wiring (passes nullable sessionId) | `src/tui/components/AgentView.tsx` | `handleModelSelect` `:3014-3040` |
| Null-session handling | `src/tui/services/modelSelectionService.ts` | `:72-197` (Rust write gated by `if (sessionId)`; store + config write unconditional) |
| Config write | `src/utils/config.ts` | `writeConfig('user', { tui: { lastUsedModel } })` `:99-119` |
| Config read / startup seed | `src/tui/services/modelInitializationService.ts` | `:70-78`, `:107-116`, `:219-229` |
| Config key | — | `tui.lastUsedModel` (e.g. `anthropic/claude-sonnet-4`) |

## Rust architecture today (what exists, what is missing)

The Rust port differs from TS: `create_session` reads an **in-memory
`SessionManager::default_model`** (not config). So to break the deadlock, a
no-session model selection must call `set_default_model` on the SessionManager
so the next `create_session` succeeds. For restart parity (TUI-035), it should
ALSO persist `tui.lastUsedModel` to `~/.fspec/fspec-config.json`.

### Rust file map
| Concern | File | Symbol / line |
|---|---|---|
| PROV-101 decline (the log line) | `codelet/sessions/src/handle_impl.rs` | `create_session` `:82-108`; emission `:89-94` |
| Isolated-session decline (Err) | `codelet/sessions/src/handle_impl.rs` | `create_isolated_session` `:816-842` (check `:822`) |
| Default-model storage + accessors | `codelet/sessions/src/session_manager.rs` | field `default_model: RwLock<Option<String>>` `:156`; `set_default_model` `:220`; `get_default_model` `:230`; implicit set on session create `:610` |
| None early-return (THE BUG) | `codelet/fspec-tui/src/app/dispatch_model_thinking_dialogs.rs` | `handle_model_selected` `:51`; None branch returns early; dispatch arm `:220` |
| Backend trait (NO set_default_model) | `codelet/fspec-tui/src/transport/mod.rs` | `trait FspecBackend` `:64`; `set_session_model` `:180` |
| Transport impls | `codelet/fspec-tui/src/transport/embedded.rs` `:199`, `websocket.rs` `:393` | `set_session_model` |
| RPC service trait/impl (NO default-model method) | `codelet/rpc/src/lib.rs` | `create_session` `:69/835`; `set_session_model` `:180/1082` |
| Config persistence (`fspec-config.json`) | `codelet/sessions/src/profile_persistence.rs`, `profile_sections.rs` | profile/provider settings; `tui.lastUsedModel` per TUI-035 |
| PROV-101 no-fallback policy | `spec/attachments/PROV-101/no-fallback-policy.md` | rows #1/#2 |

## Required change (implementation plan — for the worker)

Follow ACDD: tests FIRST (red), then implement (green). Crate-scoped cargo tests
only (disk pressure — DO NOT run the full workspace test build).

1. **`SessionManagerHandle`** (`codelet/core/src/session_manager_handle.rs`):
   expose `set_default_model(&self, model: &str)` delegating to
   `SessionManager::set_default_model` (which already exists at
   `session_manager.rs:220` and ignores empty strings).

2. **RPC service** (`codelet/rpc/src/lib.rs`): add
   `async fn set_default_model(model: String)` to the service trait + impl,
   delegating to the optional `SessionManagerHandle` (no-op when absent). Mirror
   the existing `set_session_model` shape.

3. **`FspecBackend` trait** (`codelet/fspec-tui/src/transport/mod.rs`): add
   `async fn set_default_model(&self, model: String) -> Result<()>` with a
   **default no-op body** (so mock backends compile unchanged — mirror how
   `set_session_model` / `add_custom_model` provide defaults).

4. **Transports**: implement the new method in
   `transport/embedded.rs` and `transport/websocket.rs`, delegating to the new
   RPC call.

5. **`handle_model_selected`**
   (`codelet/fspec-tui/src/app/dispatch_model_thinking_dialogs.rs:51`): in the
   `session_id == None` branch, instead of returning early, spawn a backend
   write `backend.set_default_model(format!("{provider_id}/{model_id}"))`. Keep
   the existing session branch unchanged. (Optional: also update the selector's
   `(current)` marker for the no-session case if feasible.)

6. **(Restart parity, TUI-035)** Persist `tui.lastUsedModel` to
   `~/.fspec/fspec-config.json` on the no-session selection, AND load it into
   `SessionManager::default_model` at startup so the default survives restart.
   Without this the no-session selection only fixes the CURRENT process. If this
   expands scope too much, split into a follow-up card and keep PROV-118 to the
   in-process default-model fix (step 1-5).

**Net effect:** once `default_model` is populated by a no-session selection, the
PROV-101 decline at `handle_impl.rs:89` no longer fires and the next
`create_session(role)` succeeds with the chosen model.

## Scope boundaries / non-goals

- Does NOT change the PROV-101 no-fallback policy — we are not re-adding a
  hardcoded anthropic fallback; we are letting the USER's explicit selection
  become the default.
- Does NOT touch the Enter dispatch wiring (PROV-117, done and correct).
- The existing session-present path in `handle_model_selected` is unchanged.

## Supervisor-only files (worker must NOT edit)

`canonical.rs`, `dispatch.rs` (app orchestrator), `commands/mod.rs`,
`types/mod.rs`, `main.rs`. The worker MAY edit:
`transport/mod.rs`, `transport/embedded.rs`, `transport/websocket.rs`,
`app/dispatch_model_thinking_dialogs.rs`, `codelet/rpc/src/lib.rs`,
`codelet/core/src/session_manager_handle.rs`, and add new test files.
