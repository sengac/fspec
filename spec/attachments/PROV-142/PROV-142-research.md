# PROV-142 — Per-profile Auto-Continue default: Research & Scope

## 1. Goal

Add a new field to the OpenAI profile create/edit form (the "Edit Profile"
overlay in the Provider Settings view) that lets the user persist a
`/continue` default on a profile. When a session is created against that
profile, the session's auto-continue state is seeded from the profile's
stored value.

**Field shape (per user's direction, confirmed 2026-08-20):** a single
**numeric-only** field named `Auto-Continue`. The value encodes both the
on/off state and the budget:

- `0` (or empty) → auto-continue **off** (today's behavior; the session
  starts with `continue_enabled = false`).
- `n` where `n >= 1` → auto-continue **on** with budget `n`
  (equivalent to having typed `/continue n` at session start).

Non-numeric input (e.g. `abc`) **rejects the save** with a hint mirroring
`/continue`'s invalid-argument rejection. The `on`/`off` words are NOT
accepted — the field is numeric-only (user decision, 2026-08-20).

Example: `Auto-Continue: 300` on a profile means every new session
against that profile starts as if the user had run `/continue 300`.

This is a **single field**, not a boolean + budget pair. It mirrors the
existing numeric fields (`Context Window`, `Max Output Tokens`) in the
form and keeps the form compact.

## 2. What `/continue` does today (baseline)

- **Command grammar** — `rust/cli/src/interactive/auto_continue.rs`:
  - `/continue` (bare) — toggle on/off with `DEFAULT_CONTINUE_BUDGET` (10)
  - `/continue on` — explicit on, default budget
  - `/continue off` — explicit off
  - `/continue <n>` (n ≥ 1) — arm with budget n
  - `/continue 0` — rejected with hint "use /continue off"
- **State** — two fields on `BackgroundSession`
  (`rust/sessions/src/background_session.rs:397-401`):
  - `continue_enabled: AtomicBool` (defaults to `false`)
  - `continue_budget: AtomicU32` (defaults to `10`)
- **Sync into the inner `Session`** —
  `rust/sessions/src/background_session.rs:1330-1332` copies the atomics
  into `session.continue_enabled` / `session.continue_budget` before each
  dispatched user message.
- **Consumed by** — `rust/cli/src/interactive/stream_loop.rs` at the
  `FinalResponse` settle point via `decide_continuation` (the pure decision
  function in `auto_continue.rs`).
- **TUI dispatch** — `rust/fspec-tui/src/app/dispatch_slash_continue.rs`
  parses the command, applies it via the shared
  `apply_continue_command` helper, and calls
  `FspecBackend::set_continue_state(session_id, enabled, budget)` over the
  RPC.
- **RPC surface** — `rust/rpc/src/lib.rs:316` / `:1505` exposes
  `set_continue_state` and `get_continue_state` on the
  `FspecService` trait; `rust/sessions/src/handle_impl.rs:1457-1478`
  implements them by delegating to `BackgroundSession::set_continue_state`.

**Key observation:** the state is per-session and in-memory only. There is
no persistence of `/continue` state across sessions today, and no
per-profile association.

## 3. Where the profile form lives (TUI side)

- **Form state** — `rust/fspec-tui/src/views/provider_settings/profile_form.rs`
  - `PROFILE_FORM_FIELDS: [&str; 6]` — the six labels in display order
    (Base URL, API Key, Context Window, Max Output Tokens, Compaction
    Threshold, Streaming).
  - `ProfileForm` struct — one raw string per text field plus a `streaming: bool`
    for the boolean toggle, plus `field_index`, `is_editing_name`, `is_new`.
  - `new_create()` / `from_definition()` — seed the form for create vs.
    edit mode.
  - `build_definition()` — parse the raw strings into a
    `ProfileDefinition` (returns `None` if base URL / API key / name are
    empty).
  - `route_key()` / `handle_form_key()` — Esc/Enter/Tab/Up/Down handling;
    everything else is delegated to
    `super::profile_form_streaming::route_edit_key`.
- **Streaming toggle routing** —
  `rust/fspec-tui/src/views/provider_settings/profile_form_streaming.rs`
  - `STREAMING_FIELD_INDEX: usize = 5`
  - `is_streaming_field(idx)` — predicate
  - `streaming_label(bool)` — "Enabled"/"Disabled"
  - `route_edit_key(form, code)` — while the Streaming field is focused,
    Space/Left/Right flip the boolean and every other key is swallowed;
    otherwise the key edits the focused text field.
- **Rendering** — `rust/fspec-tui/src/views/provider_settings/profile_form_render.rs`
  - `placeholder_for(idx)` — per-field dim placeholder when empty.
  - `field_line(form, idx, label)` — builds one line; the API Key field
    (idx 1) is masked via `mask_secret`.
  - `render_form(area, buf, title, form)` — paints the title, name line,
    and every field in `PROFILE_FORM_FIELDS` order.
- **Parse helpers** — `rust/fspec-tui/src/views/provider_settings/profile_form_parse.rs`
  - `opt_num(Option<u32>) -> String` — format for prefill.
  - `profile_compaction_trigger(raw)` — range-checked parse for the
    compaction threshold field.
  - `render_threshold(kind, value)` — format for prefill.
- **Save dispatch** — `rust/fspec-tui/src/app/dispatch_provider_settings_profiles.rs`
  - `handle_save_profile(...)` — spawns a tokio task that calls
    `backend.save_profile(provider_id, profile_name, definition)` (or
    `backend.rename_profile` on an edit-mode rename), then refreshes the
    provider list so the view repaints.

## 4. Wire shape (RPC types)

- **`ProfileDefinition`** — `rust/rpc-types/src/lib.rs:476-489`:
  ```rust
  pub struct ProfileDefinition {
      pub base_url: String,
      pub api_key: String,
      pub context_window: Option<u32>,
      pub max_output_tokens: Option<u32>,
      pub compaction_threshold_type: Option<String>,
      pub compaction_threshold_value: Option<u32>,
      pub streaming: Option<bool>,
  }
  ```
  - `#[cfg_attr(feature = "napi", napi_derive::napi(object))]` — the
    `napi(object)` projection requires a plain struct (no nested objects,
    no enums).
  - `streaming: Option<bool>` — `None` (key absent on disk) means
    streaming is ENABLED; only `Some(false)` is written as
    `"streaming": false`. The canonical predicate is
    `ProfileDefinition::streaming_enabled()` at `lib.rs:496`.

## 5. On-disk shape (persistence)

- **`ProfileDef`** — `rust/sessions/src/profile_persistence.rs:35-45`:
  ```rust
  pub struct ProfileDef {
      pub base_url: String,
      pub api_key: String,
      pub context_window: Option<u32>,
      pub max_output_tokens: Option<u32>,
      pub compaction_threshold: Option<CompactionThreshold>,
      pub streaming: Option<bool>,
  }
  ```
  - `customModels` is intentionally absent — owned by the RPC-347
    custom-model write path and preserved verbatim by `save_profile`.
- **Wire → disk conversion** —
  `rust/sessions/src/conversions.rs:165-186`
  (`profile_def_from_wire`). The single place the wire shape and the
  on-disk shape meet.
- **Save path** — `rust/sessions/src/profile_persistence.rs::save_profile`
  → `save_profile_at` — read-modify-write on
  `~/.fspec/fspec-config.json` under
  `providers.openai.profiles.<name>`.
- **Load path** — `rust/sessions/src/profile_sections.rs:232`
  (`load_local_server_profiles`) returns `Vec<LocalServerProfile>`; the
  TUI's `profiles_config::load_openai_profile_configs` reads the same file
  for the form prefill.
- **OpenAI-only guard** —
  `rust/sessions/src/profile_persistence.rs::profiles_supported`
  (`provider_id == "openai"`). Non-openai providers are rejected with a
  canonical error message.

## 6. How a session picks up a profile

- **Model string** — a profile model is encoded as
  `openai:<profile_name>/<model_id>` (see
  `rust/sessions/src/model_parsing.rs::parse_model_string`).
- **Session creation** —
  `rust/sessions/src/session_creation_helper.rs::create_background_session_inner`
  (RPC-425). This is the shared helper used by both
  `create_session_with_id` and `create_session_from_manifest`. It:
  1. Calls `apply_model_selection(&mut provider_manager, model)` — which
     internally calls `apply_profile_env_vars` for profile models
     (`rust/sessions/src/model_resolution.rs:97`).
  2. Constructs the `BackgroundSession` via
     `BackgroundSession::new(...)` — which initializes
     `continue_enabled: false` and `continue_budget: 10`
     (`rust/sessions/src/background_session.rs:552-553`).
  3. Applies the persisted default thinking level and model limits.
  4. Returns the session to the caller.

**The seed point** for the new per-profile default is step 2/3 in
`create_background_session_inner` — after the `BackgroundSession` is
constructed but before it is returned. The helper already receives
`is_profile_model` (via `ParsedModelInfo`) and the raw `model` string, so
it can re-parse the profile name and look up the stored profile to read
the new `auto_continue` field.

## 7. Scope of changes

### 7.1 Wire type (`rust/rpc-types/src/lib.rs`)

Add one field to `ProfileDefinition`:

```rust
/// CONT-002 / PROV-142: per-profile auto-continue default. The value
/// encodes both the on/off state and the budget:
/// - `None` (key absent on disk) or `Some(0)` → auto-continue OFF
///   (session starts with `continue_enabled = false`, today's behavior).
/// - `Some(n)` with `n >= 1` → auto-continue ON with budget `n`
///   (session starts as if the user had run `/continue n`).
/// Carried as a flat `Option<u32>` so the `napi(object)` projection stays
/// a plain struct, mirroring the `context_window` / `max_output_tokens`
/// fields above.
pub auto_continue: Option<u32>,
```

Add a helper predicate mirroring `streaming_enabled`:

```rust
/// PROV-142: canonical "is auto-continue on?" predicate. Returns `true`
/// only when `auto_continue` is `Some(n)` with `n >= 1`; `None` and
/// `Some(0)` both mean off.
pub fn auto_continue_enabled(&self) -> bool {
    self.auto_continue.map_or(false, |n| n >= 1)
}
```

### 7.2 On-disk type (`rust/sessions/src/profile_persistence.rs`)

Add the same field to `ProfileDef`:

```rust
pub auto_continue: Option<u32>,
```

### 7.3 Wire → disk conversion (`rust/sessions/src/conversions.rs`)

Extend `profile_def_from_wire` to copy the new field through:

```rust
crate::profile_persistence::ProfileDef {
    // ... existing fields ...
    auto_continue: wire.auto_continue,
}
```

### 7.4 TUI form state (`rust/fspec-tui/src/views/provider_settings/profile_form.rs`)

- Extend `PROFILE_FORM_FIELDS` from `[&str; 6]` to `[&str; 7]`:
  ```rust
  pub const PROFILE_FORM_FIELDS: [&str; 7] = [
      "Base URL",
      "API Key",
      "Context Window",
      "Max Output Tokens",
      "Compaction Threshold",
      "Streaming",
      "Auto-Continue",
  ];
  ```
- Add one field to `ProfileForm`:
  ```rust
  pub auto_continue: String,
  ```
- `new_create()` seeds `auto_continue: String::new()` (empty = off).
- `from_definition()` prefills from `opt_num(def.auto_continue)`.
- `focused_text_mut()` — add the new text field (index 6 →
  `&mut self.auto_continue`).
- `field_value()` — add index 6 → `&self.auto_continue`.
- `build_definition()` — parse the raw string into `Option<u32>`:
  - empty → `None` (off)
  - `"0"` → `Some(0)` (explicit off, written to disk)
  - `"n"` with `n >= 1` → `Some(n)` (on with budget n)
  - non-numeric → save is **rejected** with a hint mirroring
    `/continue <invalid>` (user decision, 2026-08-20). The form surfaces
    the error and does not persist.

### 7.5 TUI toggle routing (`rust/fspec-tui/src/views/provider_settings/profile_form_streaming.rs`)

No changes needed — the new field is a **text field**, not a boolean
toggle. The existing `route_edit_key` already routes non-Streaming fields
to the text-editing branch (Backspace/Char). The new field at index 6
falls through to the text branch automatically.

### 7.6 TUI rendering (`rust/fspec-tui/src/views/provider_settings/profile_form_render.rs`)

- `placeholder_for(idx)` — add:
  - `6 => "0 (off) or n (budget)"` (or shorter: `"0 = off, n = budget"`)
- No other changes — the existing `field_line` loop already iterates over
  `PROFILE_FORM_FIELDS`.

### 7.7 Session seeding (`rust/sessions/src/session_creation_helper.rs`)

After the `BackgroundSession` is constructed (around line 204), add:

```rust
// PROV-142: seed the session's auto-continue state from the profile's
// stored default (if the model is a profile model and the profile
// carries the field).
if is_profile_model {
    if let Some(profile_name) = crate::model_parsing::extract_profile_name(model) {
        if let Some(profile) = crate::profile_sections::load_local_server_profiles()
            .into_iter()
            .find(|p| p.name == profile_name)
        {
            let n = profile.auto_continue.unwrap_or(0);
            let enabled = n >= 1;
            let budget = if enabled { n } else { 10 }; // default when off
            session.set_continue_state(enabled, budget);
            tracing::info!(
                session_id = %uuid,
                profile = %profile_name,
                enabled,
                budget,
                "PROV-142: seeded auto-continue state from profile"
            );
        }
    }
}
```

**Note:** this requires either (a) a new helper
`crate::model_parsing::extract_profile_name(model)` that reuses the same
`find(':')` / `find('/')` logic as `session_manager.rs:987-988`, or (b)
plumbing the already-parsed profile name through `SessionCreationParams`.
Option (b) is cleaner — add a `profile_name: Option<&str>` field to
`ParsedModelInfo` and populate it in `parse_model_string`.

**Alternative seed point:** `BackgroundSession::new` itself could read the
profile if it had access to the model string. But the helper is the
single shared entry point for both `create_session_with_id` and
`create_session_from_manifest`, so it is the right place.

### 7.8 NAPI surface (`rust/napi/src/models/napi_bindings.rs`)

The `napi(object)` projection of `ProfileDefinition` is auto-generated
from the struct definition, so adding the field to the Rust struct is
sufficient. The `index.d.ts` file is regenerated by the NAPI build.

### 7.9 No changes needed

- `rust/sessions/src/handle_impl.rs::save_profile` /
  `rename_profile` — they delegate to `profile_def_from_wire` and
  `profile_persistence::save_profile`, both of which will carry the new
  field automatically.
- `rust/fspec-tui/src/app/dispatch_provider_settings_profiles.rs` — the
  save dispatch is generic over `ProfileDefinition`; no changes.
- `rust/cli/src/interactive/auto_continue.rs` — the `/continue` command
  grammar and the pure decision function are unchanged. The profile
  default is a *seed* for the session state; the user can still override
  it at runtime with `/continue`.

## 8. Edge cases & invariants

1. **`auto_continue: Some(300)`** — session starts with
   `continue_enabled = true`, `continue_budget = 300`.
2. **`auto_continue: Some(0)`** — session starts with
   `continue_enabled = false` (explicit off).
3. **`auto_continue: None` (key absent on disk)** — session starts with
   `continue_enabled = false` (today's behavior, zero change).
4. **Non-openai provider** — profiles are openai-only
   (`profiles_supported` guard); the new field is simply not reachable
   for other providers.
5. **Existing profiles on disk** — the `autoContinue` key is absent →
   `None` → session starts with `continue_enabled = false` (today's
   behavior). No migration needed.
6. **User overrides at runtime** — `/continue on`, `/continue off`,
   `/continue <n>` still work and override the profile-seeded state for
   the lifetime of the session. The profile default is a *seed*, not a
   *lock*.
7. **Goal mode interaction** — CONT-003's goal mode implies
   auto-continue; the profile default does not interact with goal mode
   (a goal is set via `/goal`, not via the profile).
8. **Non-numeric input in the form** — the save is **rejected** with a
   hint mirroring `/continue <invalid>` → "invalid argument — usage:
   /continue [on|off|<n>]". (User decision, 2026-08-20.)

## 9. Test plan (ACDD)

### 9.1 Feature file

New feature file:
`spec/features/provider-settings-profile-auto-continue.feature`

Scenarios:

1. **Given** a new profile form, **When** the user focuses the
   Auto-Continue field and types `300`, **Then** the field shows `300`
   and the save persists `autoContinue: 300`.
2. **Given** a profile with `autoContinue: 300`, **When** the user opens
   the edit form, **Then** the Auto-Continue field shows `300`.
3. **Given** a profile with `autoContinue: 0`, **When** the user opens
   the edit form, **Then** the Auto-Continue field shows `0`.
4. **Given** a profile with no `autoContinue` key, **When** the user
   opens the edit form, **Then** the Auto-Continue field is empty (with
   the placeholder `0 (off) or n (budget)`).
5. **Given** a profile with `autoContinue: 300`, **When** a session is
   created against that profile, **Then** the session's
   `continue_enabled` is `true` and `continue_budget` is `300`.
6. **Given** a profile with `autoContinue: 0`, **When** a session is
   created against that profile, **Then** the session's
   `continue_enabled` is `false`.
7. **Given** a profile with no `autoContinue` key, **When** a session is
   created against that profile, **Then** the session's
   `continue_enabled` is `false` (today's behavior).
8. **Given** a profile form with the Auto-Continue field focused,
   **When** the user types `abc` and presses Enter, **Then** the save is
   rejected (or the field is treated as off) with a hint.
9. **Given** a non-openai provider, **When** the user attempts to save a
   profile with the new field, **Then** the save is rejected with the
   canonical "Profiles are only supported for the OpenAI API provider"
   error (unchanged).

### 9.2 Test files

- `rust/rpc-types/tests/prov142_auto_continue_flag.rs` — wire shape
  round-trip (serde JSON), `auto_continue_enabled()` predicate.
- `rust/sessions/tests/prov142_auto_continue_persistence.rs` —
  `profile_def_from_wire` conversion, on-disk JSON shape (absent key →
  `None`, `Some(0)` → `"autoContinue": 0`, `Some(300)` →
  `"autoContinue": 300`).
- `rust/fspec-tui/tests/prov142_auto_continue_form.rs` — form state
  transitions (text input, parse, prefill from definition, build
  definition).
- `rust/sessions/tests/prov142_session_seed.rs` — session creation seeds
  `continue_enabled` / `continue_budget` from the profile.

### 9.3 Coverage linking

Link each scenario to its test file + line range via
`fspec link-coverage`.

## 10. Out of scope

- **Per-model (not per-profile) auto-continue default** — the
  `CustomModelDefinition` shape is separate and owned by the model
  selector CRUD; this story is scoped to the profile form only.
- **Goal mode default** — `/goal` is a separate command with its own
  budget semantics (CONT-003); not part of this story.
- **Cross-session persistence of `/continue` state** — the profile
  default is a *seed*; the runtime `/continue` state is still
  per-session and in-memory only.
- **TUI footer indicator** — the existing
  `live-continue-status-indicator` (CONT-007) already reflects the
  session's `continue_enabled` / `continue_budget`; no changes needed.

## 11. Effort estimate

- **Wire + persistence + conversion:** ~20 LoC (3 files)
- **TUI form (state, rendering, parse):** ~40 LoC (3 files)
- **Session seeding:** ~30 LoC (1 file, plus a small helper in
  `model_parsing.rs`)
- **Tests:** ~200 LoC (4 test files)
- **Feature file + scenarios:** ~80 LoC

**Total: ~370 LoC** — **3 story points** (1-2 hours).

## 12. Open questions (red cards)

1. **@human (ANSWERED 2026-08-20):** Placeholder for the Auto-Continue
   field → `"0 (off) or n (budget)"`.
2. **@human (ANSWERED 2026-08-20):** Non-numeric input → **reject the
   save** with a hint, mirroring `/continue <invalid>`.
