# PROV-119: Default model selection is not persisted across restarts

## Summary

The default model set via `set_default_model` is stored **only in an in-memory
`RwLock<Option<String>>`** on `SessionManager`. It is never written to disk, and
no startup code loads a persisted value. As a result, **every fresh process**
starts with `default_model = None`, so the first `create_session` is declined
(PROV-101: no anthropic fallback) on every launch until the user manually
re-selects a model in `/model`.

This is the **secondary** bug. The primary, user-visible "selecting a model does
nothing" defect is tracked in **MODEL-006**. PROV-119 ensures the choice
*survives a restart* once MODEL-006 makes selection effective within a session.

## Evidence (from `~/.fspec/logs/fspec-combined.log.2026-06-24`)

The log contains **two separate bootstrap runs**:

```
1:  INFO fspec combined mode bootstrapping ... port=37911
26: ERROR create_session declined: no default model set (PROV-101: no anthropic fallback)
...
68: INFO fspec combined mode bootstrapping ... port=36265   <-- fresh process
93: ERROR create_session declined: no default model set (PROV-101: no anthropic fallback)
```

In the first run the user successfully sets the default model (line 67:
`set_default_model OK model=anthropic/claude-opus-4-8`). Yet the **second**,
freshly-bootstrapped process (line 93) again reports `no default model set` —
proving the value did not persist across the process boundary.

## Root Cause

File: `codelet/sessions/src/session_manager.rs`

```rust
// line ~156
default_model: RwLock<Option<String>>,
...
// line ~185 (in `new()`)
default_model: RwLock::new(None),
...
// line ~220
pub fn set_default_model(&self, model: &str) {
    if !model.is_empty() {
        *self.default_model.write().expect("...") = Some(model.to_string());
    }
}
// line ~230
pub fn get_default_model(&self) -> Option<String> {
    self.default_model.read().expect("...").clone()
}
```

`set_default_model` performs **only** an in-memory write — there is no disk
persistence. A grep of every non-test caller of `set_default_model` /
`get_default_model` confirms **no startup code loads a persisted default**:

```
codelet/napi/src/bridges.rs:249            .get_default_model()
codelet/napi/src/scheduler/mod.rs:96       .get_default_model()
codelet/rpc/src/lib.rs:1108                handle.set_default_model(&model);
codelet/sessions/src/handle_impl.rs:89     self.get_default_model()  (create_session guard)
codelet/sessions/src/session_manager.rs:610 self.set_default_model(model);
```

None of these read from or write to a config file. The field is initialized to
`None` at construction and stays that way until a runtime selection occurs.

## Expected Behaviour

1. When `set_default_model` is called with a non-empty model string, the value
   is persisted to the user's config store (alongside provider credentials /
   other persisted provider config that `list_provider_credentials` already
   reads).
2. On `SessionManager` construction (or the appropriate bootstrap point), the
   persisted default model is loaded into the in-memory `default_model` field.
3. A subsequent fresh process therefore starts with a populated
   `default_model`, and the first `create_session` succeeds without manual
   re-selection.
4. Empty / whitespace model strings are never persisted (preserve the PROV-101
   no-fallback invariant — an empty string must not become a stored default).

## Suggested Implementation Direction

- Identify the existing on-disk provider/credential config location used by
  `list_provider_credentials` (RPC `FspecService.list_provider_credentials`)
  and store the default model in the same user-scoped config so it is
  discoverable and consistent.
- Add a load step at startup that reads the persisted default and calls the
  in-memory setter (do **not** bypass the empty-string guard).
- Keep persistence best-effort and non-fatal: a failed write must log but must
  not crash session creation. A missing/corrupt file loads as `None`
  (current behaviour) — i.e. graceful degradation.

### Investigation pointers for the worker

- `codelet/sessions/src/session_manager.rs` — the in-memory field and setters.
- `codelet/sessions/src/handle_impl.rs:993` — `set_default_model` trait
  delegation (PROV-118), the natural place to also persist.
- Search for where provider credentials are read/written
  (`list_provider_credentials`) to reuse the same config path/format.
- `codelet/sessions/tests/prov118_no_session_default_model.rs` and
  `codelet/sessions/tests/prov101_no_selection_fallbacks.rs` for existing
  behavioural expectations to preserve.

## Acceptance Criteria (for Example Mapping)

- **Rule:** A non-empty default model set via `set_default_model` is persisted
  to disk.
- **Rule:** On startup, a previously persisted default model is loaded into the
  in-memory `default_model`.
- **Rule:** Empty / whitespace model strings are never persisted (PROV-101
  invariant preserved).
- **Rule:** Persistence failures are non-fatal (logged, not panicking).
- **Example:** Set default `anthropic/claude-opus-4-8` → restart process →
  `get_default_model()` returns `anthropic/claude-opus-4-8` → first
  `create_session` succeeds.
- **Example:** No config file present on first launch →
  `get_default_model()` returns `None` (unchanged current behaviour).

## Key Files

| File | Role |
|------|------|
| `codelet/sessions/src/session_manager.rs` | in-memory `default_model` field, `set_default_model`, `get_default_model`, `new()` |
| `codelet/sessions/src/handle_impl.rs` | `set_default_model` trait delegation (PROV-118), `create_session` guard reading `get_default_model` |
| provider credential config (via `list_provider_credentials`) | existing persisted config to extend |
| `codelet/sessions/tests/prov118_no_session_default_model.rs` | existing default-model behaviour tests |

## Dependencies

- Logically pairs with **MODEL-006** (the in-session re-trigger). PROV-119 makes
  the choice durable across restarts; MODEL-006 makes the choice effective
  within the current session. They can be implemented independently but PROV-119
  is only fully valuable once MODEL-006 lands.

## Out of Scope

- Re-triggering `create_session` after selection within a running process —
  tracked in **MODEL-006**.
