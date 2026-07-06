# PROV-139 — Streaming toggle in profile settings form (schema + /provider view UI + persistence)

> **Scope:** This card delivers the *option* and its *persistence* only. The
> runtime behaviour of the flag (issuing a `stream=false` request and
> synthesizing chunks) is owned by the child card **PROV-140**, which depends
> on this one.

## 1. Goal

Add a boolean **Streaming** field — **enabled (`true`) by default** — to the
OpenAI profile definition, and expose it as an editable option in the profile
**create/edit form** of the Rust ratatui **Provider Settings** screen
(reached via the `/provider` view). The flag must round-trip through:

1. The wire type `ProfileDefinition` (RPC + NAPI projection).
2. The on-disk persistence type `ProfileDef`.
3. `~/.fspec/fspec-config.json` under `providers.openai.profiles.<name>`.

…while **preserving** the existing sibling keys that the profile
read-modify-write already protects (`customModels[]`, the compaction-threshold
fields, and any unknown keys).

## 2. Background — how OpenAI streaming works

Confirmed via the OpenAI API reference and the OpenAI-compatible guide:

- `stream` is a **boolean field in the request JSON body** of
  `POST /v1/chat/completions` — **not** a URL query parameter.
- `stream: true` → server returns Server-Sent Events (a sequence of
  `chat.completion.chunk` objects, terminated by `data: [DONE]`).
- `stream: false` (or omitting it) → server returns one whole
  `chat.completion` object.
- `stream_options` (e.g. `{"include_usage": true}`) is **only valid when
  `stream: true`** and must be dropped when streaming is disabled.

The practical driver for a per-profile toggle: many third-party
"OpenAI-compatible" endpoints (vLLM, Ollama, Fireworks, local servers)
behave inconsistently around streaming. Profiles are OpenAI-only and are the
natural home for a per-endpoint connection setting.

## 3. Current state (what exists today)

### 3.1 Profile is represented in THREE parallel layers

| Layer | Type | File |
|---|---|---|
| Wire (RPC + NAPI) | `ProfileDefinition` | `codelet/rpc-types/src/lib.rs` (~line 449) |
| On-disk / persistence | `ProfileDef` | `codelet/sessions/src/profile_persistence.rs` (~line 34) |
| Conversion bridge | `profile_def_from_wire` | `codelet/sessions/src/conversions.rs` |

`ProfileDefinition` today:

```rust
pub struct ProfileDefinition {
    pub base_url: String,
    pub api_key: String,
    pub context_window: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub compaction_threshold_type: Option<String>,
    pub compaction_threshold_value: Option<u32>,
}
```

`ProfileDef` today:

```rust
pub struct ProfileDef {
    pub base_url: String,
    pub api_key: String,
    pub context_window: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub compaction_threshold: Option<CompactionThreshold>,
}
```

### 3.2 The profile form in the /provider view

`codelet/fspec-tui/src/views/provider_settings/profile_form.rs` owns
`ProfileForm` and the key routing. The form currently exposes **five**
connection fields, in display order:

```rust
pub const PROFILE_FORM_FIELDS: [&str; 5] = [
    "Base URL",
    "API Key",
    "Context Window",
    "Max Output Tokens",
    "Compaction Threshold",
];
```

Navigation model (already implemented):
- A name field (`is_editing_name`) sits above the connection fields.
- `Up`/`Down` move focus; `Up` from the first connection field re-enters the
  name; `Down` from the name enters the first field.
- Printable ASCII chars edit the focused field; `Backspace`/`Delete` deletes.
- `Enter` builds a `ProfileDefinition` via `build_definition()` and emits
  `Action::SaveProfile { provider_id, profile_name, old_profile_name,
  definition }`; `Esc` cancels.
- `build_definition()` returns `None` (guard) when base URL, API key, or the
  trimmed name is empty.

Related files:
- `provider_settings/profile_form_render.rs` — renders the field rows.
- `provider_settings/profile_form_paste.rs` — paste routing.
- `app/dispatch_provider_settings_profiles.rs` — dispatches `SaveProfile`.
- `sessions/src/conversions.rs` — `profile_def_from_wire`.
- `sessions/src/profile_persistence.rs` — `save_profile` / `save_profile_at`
  (read-modify-write that preserves `customModels[]` + siblings).

## 4. Design — the change this card makes

### 4.1 Field type & default

Add an **optional boolean** so that older on-disk profiles (which have no such
key) are treated as **streaming enabled** — matching the "true by default"
requirement without a schema migration.

- `ProfileDefinition.streaming: Option<bool>` (wire).
- `ProfileDef.streaming: Option<bool>` (persistence).
- `profile_def_from_wire` copies it through.
- **Absence ⇒ enabled.** Introduce a single canonical helper so the semantics
  cannot drift, e.g. `ProfileDefinition::streaming_enabled(&self) -> bool`
  returning `self.streaming.unwrap_or(true)`.

> **Persisted key name:** use camelCase `streaming` in
> `fspec-config.json` for consistency with the existing `customModels` /
> `compactionThreshold` conventions. Confirm the exact serialized key against
> the sibling keys already written by `save_profile_at` before implementing.

### 4.2 Form field

Add a sixth entry to `PROFILE_FORM_FIELDS`: **"Streaming"**. It is a **boolean
toggle**, not a free-text field:

- Rendered as `Streaming: Enabled` / `Streaming: Disabled` (or `[x]`/`[ ]`).
- **Toggle keys:** `Space` (and/or `Left`/`Right`) flip the value when the
  field is focused. Printable-char typing does NOT append text to this field.
- New create-mode form seeds it to **enabled**.
- Edit-mode form seeds it from the stored definition via
  `streaming_enabled()`.
- `build_definition()` sets `streaming: Some(<bool>)` from the form value.

Update the sibling render/paste modules so the new field renders and is not
treated as a text field by paste.

### 4.3 Persistence round-trip

- `save_profile_at` must write the `streaming` key while preserving
  `customModels[]`, compaction fields, and unknown keys (extend the existing
  read-modify-write, do not replace the object).
- Loading a profile that lacks the key yields `streaming: None` ⇒ enabled.

## 5. Explicit non-goals (owned by PROV-140)

- Threading the flag into `OpenAIProvider` construction.
- Making `supports_streaming()` honor it.
- Any change to the agent loop / `rig_agent` / the vendored rig-core patch.
- Issuing an actual `stream=false` request.

This card is **done** when the option exists, is editable in the /provider
form, and survives a save→reload cycle in `fspec-config.json` — with no
runtime behaviour change yet.

## 6. Invariants to preserve (from the RPC-002 epic)

1. **Single source of truth for wire types** — the flag lives in
   `rpc-types` and is re-exported by NAPI; do not duplicate the type.
2. **`napi(object)` projection stays a plain struct** — use a flat
   `Option<bool>`, mirroring how the compaction-threshold override is carried
   as flat optional fields.
3. **File-size discipline** — every touched/new module file stays under
   300 LoC. `profile_form.rs` is already at 298 LoC; adding the toggle will
   likely require extracting the boolean-field logic into a sibling module
   (e.g. `profile_form_streaming.rs`) rather than growing the file.
4. **Read-modify-write preservation** — `customModels[]` and sibling keys must
   survive a profile save.
5. **Cross-transport parity** — persistence behaviour identical whether the
   save flows through the embedded or WebSocket backend.
6. **OpenAI-only** — profiles remain gated to the `openai` provider
   (`profiles_supported`).

## 7. Suggested test surface (finalize during Example Mapping)

- `ProfileDefinition::streaming_enabled()` returns `true` when `None`,
  echoes `Some(true)`/`Some(false)`.
- `profile_def_from_wire` copies `streaming` through.
- New create-mode `ProfileForm` seeds Streaming = enabled.
- Edit-mode `ProfileForm::from_definition` seeds Streaming from the stored
  value (including a disabled profile).
- Toggling the Streaming field with `Space` flips the value; typing a
  printable char does NOT mutate it.
- `build_definition()` emits `streaming: Some(<bool>)` matching the form.
- Rendering shows `Enabled`/`Disabled` per the current value.
- `save_profile_at` writes the `streaming` key and preserves `customModels[]`
  + compaction fields on round-trip (temp-dir integration test).
- A profile file with no `streaming` key loads as enabled.

## 8. Key files index

| Concern | Location |
|---|---|
| Wire type | `codelet/rpc-types/src/lib.rs` (`ProfileDefinition`) |
| Persistence type + save | `codelet/sessions/src/profile_persistence.rs` (`ProfileDef`, `save_profile_at`) |
| Wire→disk bridge | `codelet/sessions/src/conversions.rs` (`profile_def_from_wire`) |
| Form state + keys | `codelet/fspec-tui/src/views/provider_settings/profile_form.rs` |
| Form render | `codelet/fspec-tui/src/views/provider_settings/profile_form_render.rs` |
| Form paste | `codelet/fspec-tui/src/views/provider_settings/profile_form_paste.rs` |
| Save dispatch | `codelet/fspec-tui/src/app/dispatch_provider_settings_profiles.rs` |
