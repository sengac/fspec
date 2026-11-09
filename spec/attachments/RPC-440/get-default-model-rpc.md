# Research: `get_default_model` RPC

## Why

fspec-paseo (the Paseo provider plugin for fspec) can *set* fspec's default model over the wire
(`set_default_model`, idx 30), but there's no way to *read* it back. As a result:

- Paseo builds its model picker from `list_providers` and has to guess the default (it takes the first
  model of the first reachable provider).
- When Paseo opens an fspec session, it calls `set_session_model` with that guess, which overrides
  the default fspec would otherwise have used (`sessions/src/handle_creation.rs:9` uses
  `manager.get_default_model()` when creating a session).
- A "default model" control in Paseo's fspec settings screen would write a value that nothing in
  Paseo reads back.

Tracked on the fspec-paseo side as FSPEC-046 (*Follow fspec's default model in Paseo*), which is
blocked on this card.

## What already exists

- `SessionManager::get_default_model(&self) -> Option<String>` at `rust/sessions/src/session_manager.rs:374`.
  This is the in-memory value restored from `<data_dir>/default-model.json` / `tui.lastUsedModel`
  (PROV-119, PROV-122).
- `set_default_model` is already exposed end-to-end: the trait (`rpc/src/service_declaration/providers_sessions.rs:28`),
  `SessionManagerHandle::set_default_model` (`core/src/session_manager_handle.rs:175`),
  `handle_impl.rs:1282` and the service impl `rpc/src/service_implementation/providers.rs:35`.

## Proposed surface

```rust
/// Returns the daemon's default model as "provider/model", or None when no
/// default is set (PROV-101 no-fallback policy: never invent one).
async fn get_default_model() -> Option<String>;
```

The plumbing mirrors `set_default_model`: the trait method, `SessionManagerHandle::get_default_model`,
handle_impl delegating to `SessionManager::get_default_model`, the service impl, and the transports
(embedded and websocket) plus NAPI if the TS side needs it.

## Wire-compatibility constraint (important)

tarpc variant indices are the trait declaration order. External clients like fspec-paseo encode those
indices by position (the codec pins `set_default_model=30`, `list_provider_credentials=88`,
`get_work_unit_details=123`, and so on). **Append the new method at the end of the trait (index 124)**
so no existing index shifts. Inserting it next to `set_default_model` would renumber every later method
and break existing clients.

## Acceptance ideas

- With a default set, `get_default_model()` returns it in the same `provider/model` form that
  `set_default_model` accepts.
- With no default, it returns `None`.
- After `set_default_model("x/y")`, `get_default_model()` returns `Some("x/y")`, and it still does
  after a daemon restart (persistence is already in place).
- Every existing method keeps its variant index (124 → 125 methods, with the new one last).
