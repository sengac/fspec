# PROV-116 — Profile delete restores cursor to parent provider row

**Date:** 2026-06-23
**Source:** DeepSearch of `src/tui/` (TypeScript reference) vs
`codelet/fspec-tui/src/` (Rust port), cross-checked by reading the actual source.
Every claim carries a `file:line` reference.

---

## 1. The verified parity gap

### TypeScript (the parity target)
`src/tui/hooks/useProviderSettingsState.ts` — `removeProfile` (445-452):

```ts
const removeProfile = useCallback(async (providerId, profileName) => {
  await deleteProfile(providerId, profileName);
  navigateToProviderRef.current = providerId;   // ← sets the navigate target
  await reload();
}, [...]);
```

After the reload rebuilds `navItems`, a `useEffect` (367-379) reads
`navigateToProviderRef`, finds the parent provider row index and
`setSelectedIndex(idx)` (+ adjusts scroll). Net effect: **after deleting a profile
the cursor jumps back to the parent provider row** (the PROV-036 behavior, which
previously fixed the tree collapsing/cursor-loss after a destructive action).

Contrast: after a **save** TS does NOT set `navigateToProviderRef`, so the cursor
is left where the memo lands (no auto-target). PROV-116 is about **delete only**.

### Rust (current behavior — the gap)
`codelet/fspec-tui/src/app/dispatch_provider_settings_profiles.rs` —
`handle_delete_profile` (119-145):

```rust
match backend.delete_profile(provider_id.clone(), profile_name.clone()).await {
    Ok(()) => {
        // status message ...
        if let Ok(list) = backend.list_provider_credentials().await {
            let _ = action_tx.send(Action::ProviderCredentialsLoaded(list));
        }
    }
    // ...
}
// NOTE: nothing ever calls set_navigate_target(provider_id) on this path.
```

The reload fold `handle_provider_credentials_loaded` (26-44) DOES call
`view.apply_pending_navigate()` (line 43) — but `pending_navigate_provider` was
never set for the delete path, so the consume is a no-op and the cursor is left
wherever the rebuilt nav tree happens to place it.

### The mechanism already exists
`codelet/fspec-tui/src/views/provider_settings/nav_tree_ops.rs`:
- `set_navigate_target(provider_id)` — line 152: sets
  `pending_navigate_provider = Some(provider_id)`.
- `apply_pending_navigate()` — line 161: on the next nav rebuild, takes the target
  and moves the cursor to that provider's row.
- State field `pending_navigate_provider: Option<String>` — `mod.rs:89`.

The **OAuth-disconnect** dispatch path (PROV-112) already uses `set_navigate_target`
so the cursor returns to the parent provider after the Logout row disappears
(`handle_provider_credentials_loaded` comment, lines 40-43). The profile-delete
path simply needs the same wiring.

---

## 2. Required behavior (parity)

On a **successful** profile delete, before/with the credential-reload refresh,
call `set_navigate_target(provider_id)` so that after the nav tree repaints the
cursor lands on the **parent provider row** (e.g. the `openai` row), matching TS.

- Applies to the delete path only (both `Action::DeleteProfile` and
  `Action::ConfirmDeleteProfile` route through `handle_delete_profile`).
- Save path is intentionally NOT changed (TS does not navigate on save).
- A failed delete (`Err`) must NOT set the navigate target (no cursor jump on
  failure) — matches TS, where the `await deleteProfile` throw skips the assignment.

---

## 3. Where to wire it

`handle_delete_profile` runs the backend call inside a spawned tokio task and
communicates back via `action_tx` Actions. `set_navigate_target` mutates the view,
which lives on `App` (not `Send` into the task). Two clean options for the worker:

1. Set the target **synchronously before** spawning the task (optimistic): call
   `self.navigator.provider_settings.set_navigate_target(provider_id)` in
   `handle_delete_profile` before `tokio::spawn`. The pending target is consumed by
   the next `apply_pending_navigate()` in the reload fold. Risk: if the delete
   fails, the target is set but the reload still fires the same list — acceptable
   only if a failed delete does not reload. **Verify the failure path.**
2. Carry the provider_id through to the reload: emit it with a dedicated Action or
   set the target inside `handle_provider_credentials_loaded` keyed on a
   "last deleted provider" marker set only on `Ok`.

Prefer the approach that guarantees **no cursor jump on delete failure**. The
implementing worker decides, but the acceptance tests must assert both the
success-jumps and failure-does-not-jump behaviors.

---

## 4. Edge cases to cover in tests

- Delete a profile from an expanded `openai` provider with multiple profiles →
  after refresh the cursor is on the `openai` provider row.
- Delete the only profile → cursor on the `openai` provider row (now showing just
  the `+ Add Profile` child).
- A failed `delete_profile` (MockBackend scripted `Err`) → cursor does NOT jump
  (and tokens/profiles preserved).

---

## 5. ACDD constraints

- Strict 100% ACDD: feature file → failing tests (witnessed RED) → impl.
- Offline tests only; MockBackend scripted Ok/Err + call counters; no real
  `~/.fspec`, no env mutation.
- Files < 300 LoC; clippy `-D warnings`; cargo fmt clean; NO git; do not touch
  user WIP (main.rs / session_manager.rs).
- Reuse the existing `set_navigate_target` / `apply_pending_navigate` mechanism;
  do not invent a parallel one.
