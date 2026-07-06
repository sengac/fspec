# AST Research — PROV-139 (profile streaming toggle)

AST/structural analysis of the code the streaming flag must thread through.
Tool: AstGrep + ripgrep over `codelet/`.

## Target symbols located

| Symbol | File:Line | Notes |
|---|---|---|
| `pub struct ProfileDefinition` | `rpc-types/src/lib.rs:449` | Wire type. 6 flat fields today; add `streaming: Option<bool>`. `#[napi(object)]` gated. |
| `pub struct ProfileDef` | `sessions/src/profile_persistence.rs:34` | On-disk type. Add `streaming: Option<bool>`. |
| `fn profile_def_from_wire` | `sessions/src/conversions.rs:165` | Wire→disk bridge; must copy `streaming` (currently copies base_url/api_key/context_window/max_output_tokens/compaction_threshold). |
| `pub const PROFILE_FORM_FIELDS: [&str; 5]` | `fspec-tui/src/views/provider_settings/profile_form.rs:28` | Add "Streaming" → `[&str; 6]`. |
| `pub struct ProfileForm` | `fspec-tui/src/views/provider_settings/profile_form.rs:47` | Add `streaming: bool`. |
| `fn new_create` | `profile_form.rs:66` | Seed `streaming = true`. |
| `fn from_definition` | `profile_form.rs:84` | Seed `streaming = def.streaming_enabled()`. |
| `fn build_definition` | `profile_form.rs:170` | Emit `streaming: Some(self.streaming)`. |
| form render loop | `profile_form_render.rs:118` | `for (idx, label) in PROFILE_FORM_FIELDS.iter()...` — render Enabled/Disabled for the Streaming index. |
| paste routing | `profile_form_paste.rs` | Streaming index must be excluded from text paste. |

## Field-routing functions in ProfileForm (all switch on field_index)

- `focused_text_mut()` (profile_form.rs:102) — maps index→`&mut String`. The
  Streaming field has no backing String; toggle logic must branch BEFORE this.
- `field_value(idx)` (profile_form.rs:158) — display value per index.
- `move_down` / `move_up` (profile_form.rs:112/121) — navigation; the new
  field extends the `PROFILE_FORM_FIELDS.len() - 1` bound automatically.
- `push_char` / `backspace` (profile_form.rs:142/134) — must NOT append to the
  Streaming field; the toggle (Space/Left/Right) is handled in `route_key`
  (profile_form.rs:195).

## Persistence

- `save_profile` / `save_profile_at` (`sessions/src/profile_persistence.rs`) —
  read-modify-write that preserves `customModels[]` + siblings. Must serialize
  the `streaming` key (camelCase) and reload it.
- `profiles_supported` guard (openai-only) unchanged.

## Conclusion

The change is a flat `Option<bool>` threaded through 3 type layers + the form.
No new bounded context; low structural risk. The main care points are (1)
keeping `profile_form.rs` under 300 LoC by extracting the boolean-field logic,
and (2) the read-modify-write preservation of `customModels[]`.
