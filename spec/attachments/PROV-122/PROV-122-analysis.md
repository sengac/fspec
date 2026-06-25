# PROV-122 — Model selection never persists `tui.lastUsedModel` to `fspec-config.json`

## User-reported symptom

> "When I select a new model, it's not persisting that to the `lastUsedModel` in
> `~/.fspec/fspec-config.json`."

Confirmed. With an **active session**, selecting a model writes **nothing** to
disk. With **no session**, it writes only the legacy `default-model.json`, never
the canonical `tui.lastUsedModel` key.

---

## Root cause (Rust `codelet/`)

PROV-120 implemented the **READ / restore** half (startup reads
`tui.lastUsedModel` first, falling back once to legacy `default-model.json`). The
matching **WRITE** half was never implemented. A repo-wide search shows the only
non-test references to `lastUsedModel` live in the read module
`last_used_model_persistence.rs`. **There is no writer.**

### Path 1 — live session (the user's exact case): writes NOTHING

`codelet/sessions/src/handle_impl.rs` → `set_model` (≈ line 999):

```rust
let model = format!("{provider_id}/{model_id}");           // line 1018
// ... resolve creds, apply_model_selection, recompute limits ...
session.set_model(Some(provider_id.into()), Some(model_id.into())); // line 1050
session.set_model_limits(/* ... */);
Ok(())                                                      // line 1056 — NO DISK WRITE
```

Only in-memory `ProviderManager` / session state + env vars are mutated. Nothing
is persisted. On restart the selection is gone.

### Path 2 — no session: writes the WRONG file

`codelet/sessions/src/session_manager.rs` → `set_default_model` (line 227):

```rust
pub fn set_default_model(&self, model: &str) {
    if !model.is_empty() {
        *self.default_model.write()... = Some(model.to_string());
        // PROV-119: persists to <data_dir>/default-model.json ONLY
        if let Err(e) = crate::default_model_persistence::save_default_model(model) { ... }
    }
}
```

This writes `<data_dir>/default-model.json` (the legacy fallback store), **not**
`tui.lastUsedModel`. So even the no-session path does not produce the canonical
key that PROV-120's read path prefers.

### Documentation/test claims it works — but it doesn't

`last_used_model_persistence.rs` header comment claims `tui.lastUsedModel` is
"the single source of truth" and "new writes go to `tui.lastUsedModel`". A test
comment at `prov120_startup_model_persistence.rs:67` asserts "subsequent model
selection writes are written to fspec-config.json tui.lastUsedModel". **Both are
aspirational — the write-back is unimplemented.**

---

## TypeScript reference (the correct behavior to port)

`src/tui/services/modelSelectionService.ts` → `selectModel()` (lines 170–191).
After a **successful** session update (and also in the no-session case), it does a
key-preserving read-merge-write:

```ts
const modelString = buildModelString(
  { providerId: selection.providerId, profileName: selection.profileName },
  selection.modelId
);
const existingConfig = await loadConfig();              // read
await writeConfig('user', {                             // merge + write
  ...existingConfig,
  tui: { ...existingConfig?.tui, lastUsedModel: modelString },
});
// failure is logged but NON-FATAL (does not fail the selection)
```

- Write happens **only on success**.
- Spreads existing config — only `tui.lastUsedModel` is overwritten; other keys
  preserved.
- `buildModelString()` (`src/tui/utils/model-selection.ts:49`) produces
  `anthropic/claude-sonnet-4` or the profile-qualified
  `openai:work-vllm/Qwen/Qwen3-80B`.
- Persistence failure is caught and logged, never fatal.

Restore side: `loadPersistedModelString()`
(`src/tui/services/modelInitializationService.ts:70`) reads
`config?.tui?.lastUsedModel`.

---

## Existing Rust primitives to reuse (do NOT reinvent)

All already in `codelet/sessions/src/profile_sections.rs` and used by the
PROV-108 profile writer — mirror that pattern exactly:

| Primitive | Location | Purpose |
|---|---|---|
| `fspec_user_dir()` | `profile_sections.rs:183` | Resolves `~/.fspec` via `FSPEC_USER_DIR`/`HOME`. |
| `read_config_value(path)` | `profile_sections.rs:377` | Reads `fspec-config.json` → `serde_json::Value`; `None` on missing/malformed. |
| `write_config_value(path, &root)` | `profile_sections.rs:385` | Pretty-print + trailing newline; **preserves key order** (`preserve_order` serde feature). |
| `save_custom_model_at` | `profile_sections.rs:300` | Reference read-merge-write that preserves unrelated keys. |

These are `pub(crate)` to the `sessions` crate, so the new writer must live in the
`sessions` crate (extend `last_used_model_persistence.rs`).

---

## Proposed fix

### 1. New writer in `last_used_model_persistence.rs`

```rust
/// Path-injectable core: read-merge-write `tui.lastUsedModel` into
/// `<user_dir>/fspec-config.json`, preserving all other keys. Empty/whitespace
/// model is a no-op (PROV-101 invariant). Creates the file/dir if absent.
pub fn save_persisted_model_string_to(user_dir: &Path, model: &str) -> Result<(), String>;

/// Convenience: env-resolved user dir. Best-effort; caller logs + swallows.
pub fn save_persisted_model_string(model: &str) -> Result<(), String>;
```

Behavior:
- `model.trim().is_empty()` → `Ok(())` no-op (never persist empty — PROV-101).
- Read existing config via `read_config_value`; if missing/malformed, start from
  `serde_json::json!({})` so a fresh install still gets a file.
- Ensure `root["tui"]` is an object, set `tui.lastUsedModel = model`, leaving all
  other keys (including existing `tui.*`) untouched.
- `write_config_value(path, &root)`.

### 2. Call from the live-session path

In `handle_impl.rs::set_model`, after the successful state mutation (just before
`Ok(())` at line 1056), best-effort persist using the already-built `model`
string (`format!("{provider_id}/{model_id}")`):

```rust
if let Err(e) = crate::last_used_model_persistence::save_persisted_model_string(&model) {
    tracing::warn!(error = %e, model, "set_model: failed to persist lastUsedModel (non-fatal)");
}
```

### 3. Call from the no-session path

In `session_manager.rs::set_default_model` (or the `FspecBackend` wrapper), add a
best-effort `save_persisted_model_string(model)` alongside the existing
`save_default_model(model)` so the canonical key is written too. **Keep** the
`default-model.json` write for back-compat (PROV-120 fallback + PROV-119 tests
depend on it).

### 4. Round-trip fidelity (critical)

Whatever string is written MUST be re-readable by `load_persisted_model_string`
and re-selectable at startup. The live path writes `provider_id/model_id`; verify
this matches the format the startup restore + model-selector expect, including the
**profile-qualified** form (e.g. `openai:<profile>/<model>`) so profile selections
round-trip identically to the TS `buildModelString` output. Add a test that writes
via the selection path and reads back via `load_persisted_model_string`.

---

## Scope / non-goals

- **In scope:** writer helper + wiring both selection paths + round-trip test +
  key-preservation test + empty-string no-op test + non-fatal-failure test.
- **Out of scope:** removing `default-model.json` (kept for back-compat). Changing
  the read path (PROV-120 already correct).
- **PROV-101 invariant preserved:** never persist empty/whitespace; no hardcoded
  fallback model.

## Affected files

- `codelet/sessions/src/last_used_model_persistence.rs` (add writer)
- `codelet/sessions/src/handle_impl.rs` (`set_model` call site ≈ 1050)
- `codelet/sessions/src/session_manager.rs` (`set_default_model` ≈ 227) and/or
  `handle_impl.rs::set_default_model` (≈ 995)
- New/updated test file (1:1 with the new feature file)
