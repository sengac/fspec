# AST Research: PROV-142 Per-profile Auto-Continue Default Integration Points

**Work unit:** PROV-142
**Date:** 2026-08-20
**Method:** GraphSearch (ast_search) + Read over `rust/rpc-types`, `rust/sessions`, `rust/fspec-tui`, `rust/core`

## Goal

Identify the exact code sites where the per-profile `autoContinue` field must be
added (wire shape, on-disk shape, conversion, TUI form, session seeding) and the
existing machinery it should reuse (PROV-139 streaming flag as the template).

## Findings

### 1. Wire shape — `rust/rpc-types/src/lib.rs:476-499`

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

impl ProfileDefinition {
    pub fn streaming_enabled(&self) -> bool {
        self.streaming.unwrap_or(true)
    }
}
```

`#[cfg_attr(feature = "napi", napi_derive::napi(object))]` — the projection
requires a plain struct, so the new field must be a flat `Option<u32>`.
**Add:** `pub auto_continue: Option<u32>` + `pub fn auto_continue_enabled(&self) -> bool`
(`self.auto_continue.map_or(false, |n| n >= 1)`). The NAPI `index.d.ts` is
regenerated from the struct.

### 2. On-disk shape — `rust/sessions/src/profile_persistence.rs:35-45`

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

**Add:** `pub auto_continue: Option<u32>`.

### 3. Save merge — `rust/sessions/src/profile_persistence.rs:158-181` (`merge_profile`)

Each optional field is written via `set_or_remove(profile, "key", def.field.map(Value::from))`
— `None` REMOVES the key. **Add:**
`set_or_remove(profile, "autoContinue", def.auto_continue.map(Value::from));`
so `Some(0)` is written as `"autoContinue": 0` and `None` leaves the key absent.

### 4. Wire → disk bridge — `rust/sessions/src/conversions.rs:165-186` (`profile_def_from_wire`)

Single place the wire and on-disk shapes meet. **Add:** `auto_continue: wire.auto_continue`
to the `ProfileDef` construction.

### 5. Read path — `rust/sessions/src/profile_sections.rs:83-106` (`LocalServerProfile`)

Deserialized from `providers.openai.profiles.<name>` with camelCase renames and
`default`. **Add:**
`#[serde(rename = "autoContinue", default, deserialize_with = "de_opt_u32_lenient")]
pub auto_continue: Option<u32>`.
`load_local_server_profiles()` (line 232) resolves `~/.fspec/fspec-config.json`
via `fspec_user_dir()` which honors the `FSPEC_USER_DIR` env override — the
seeding test isolates the config this way (PROV-141 precedent,
`rust/sessions/tests/prov141_session_creation_without_global_credentials.rs:74-102`).

### 6. TUI form state — `rust/fspec-tui/src/views/provider_settings/profile_form.rs`

- `PROFILE_FORM_FIELDS: [&str; 6]` (lines 29-36) → extend to `[&str; 7]` with
  `"Auto-Continue"` appended after `"Streaming"`.
- `ProfileForm` struct (lines 41-58) → add `pub auto_continue: String` (raw text,
  parsed on build — same pattern as `context_window` / `max_output_tokens`).
- `new_create()` (lines 62-75) → seed `auto_continue: String::new()`.
- `from_definition()` (lines 81-97) → prefill `opt_num(def.auto_continue)`
  (helper already exists in `profile_form_parse.rs:21`).
- `focused_text_mut()` (lines 101-109) → the `_` arm currently maps BOTH index 4
  and 5 to `compaction_threshold` — index 5 is the Streaming bool (routed
  earlier by `route_edit_key`), so add `6 => &mut self.auto_continue` and keep
  `_ => compaction_threshold` for index 4.
- `field_value()` (lines 157-166) → add `6 => &self.auto_continue`; the `_` arm
  (Streaming label) stays for index 5.
- `build_definition()` (lines 170-185) → parse `self.auto_continue.trim()`:
  empty → `None`; `"0"` → `Some(0)`; `"n"` (n ≥ 1) → `Some(n)`; non-numeric →
  save REJECTED with a hint (user decision 2026-08-20). `build_definition`
  currently returns `Option<ProfileDefinition>` where `None` means "required
  field empty — keep form open". For invalid auto-continue the form must surface
  a HINT, so the view needs a way to show it: `handle_form_key` (lines 213-250)
  already has a `None` arm that calls `restore_mode` + `Consumed`; extend the
  view with a status/hint string (e.g. `view.set_status(...)`) in that arm when
  the rejection cause is the auto-continue field.
- **No routing changes needed:** `profile_form_streaming::route_edit_key`
  (lines 38-48) only special-cases `field_index == 5`; index 6 falls through to
  the text-editing branch (Backspace/Char) automatically.

### 7. TUI rendering — `rust/fspec-tui/src/views/provider_settings/profile_form_render.rs`

`placeholder_for(idx)` (lines 54-62) → add `6 => "0 (off) or n (budget)"`.
`render_form` iterates `PROFILE_FORM_FIELDS` generically — no other changes.

### 8. Session seeding — `rust/sessions/src/session_creation_helper.rs`

`create_background_session_inner` (lines 106-278) destructures
`ParsedModelInfo { model, registry_provider, is_profile_model, ... }` (line 37).
The `BackgroundSession` is constructed at line ~185 and the persisted thinking
level is applied at lines 206-213. **Seed point:** after line 213 (or right after
session construction), when `is_profile_model`, extract the profile name from
`model` (`model[colon_idx+1..slash_idx]`, same logic as
`model_resolution.rs:71-77`), look it up via
`crate::profile_sections::load_local_server_profiles().into_iter().find(|p| p.name == profile_name)`,
and call `session.set_continue_state(enabled, budget)`:

- `enabled = n >= 1`, `budget = n` when enabled, `10` (DEFAULT_CONTINUE_BUDGET)
  when off.
- `BackgroundSession::set_continue_state` (`background_session.rs:1227-1230`)
  stores `continue_enabled` / `continue_budget` atomics; `budget.max(1)` is
  applied internally, so budget 10 is safe.
- Observable via `SessionManagerHandle::get_continue_state`
  (`core/src/session_manager_handle.rs:305`, impl at
  `sessions/src/handle_impl.rs:1473`) — the test asserts through this handle.

**Profile-name plumbing:** `ParsedModelInfo` already carries `model: &str` and
`is_profile_model: bool`, so the helper can re-extract the profile name inline
(`model.rsplitn(2, ':').next()` is wrong — use the same `find(':')` / `find('/')`
slice as `model_resolution.rs:71-77`). No new field on `ParsedModelInfo` is
required.

### 9. No changes needed

- `handle_impl.rs::save_profile` / `rename_profile` — delegate to
  `profile_def_from_wire` + `profile_persistence::save_profile`; carry the new
  field automatically.
- `dispatch_provider_settings_profiles.rs::handle_save_profile` — generic over
  `ProfileDefinition`; no changes.
- `rust/cli/src/interactive/auto_continue.rs` — `/continue` grammar and
  `decide_continuation` unchanged; the profile default is a seed only.
- TUI footer indicator (CONT-007) — already reflects session state.

## Test plan mapping (ACDD)

| Test file | Scenarios covered |
|---|---|
| `rust/rpc-types/tests/prov142_auto_continue_flag.rs` | wire serde round-trip, `auto_continue_enabled()` predicate (scenario 11) |
| `rust/sessions/tests/prov142_auto_continue_persistence.rs` | `profile_def_from_wire` bridge (scenario 10), save writes `autoContinue` key / removes when None |
| `rust/fspec-tui/tests/prov142_auto_continue_form.rs` | form seed/prefill/input/save-reject (scenarios 1-6) |
| `rust/sessions/tests/prov142_session_seed.rs` | session creation seeds continue state (scenarios 7-9), PROV-141 harness pattern |
