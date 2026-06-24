# PROV-110 — AST research: profile form UI parity targets

AST queries run with the AstGrep tool against the Rust port.

## 1. Existing mode enum to extend
`pub enum ProviderSettingsMode { ... }` —
codelet/fspec-tui/src/views/provider_settings/mod.rs:43

Current variants: `List`, `Detail { provider_id, sub }`. PROV-110 adds
`CreateProfile { provider_id, form }` and `EditProfile { provider_id,
profile_name, form }`. Every exhaustive match on this enum must gain arms:
- mod.rs `footer_hint` (line 150), `handle_key` (line 219), `render` (line 277)
- nav_tree_ops.rs `delete_target_provider_id` (line 84)

## 2. Reusable form-key pattern (model_selector parity)
`pub fn handle_key(&mut self, key: KeyEvent) -> FormOutcome { ... }` —
codelet/fspec-tui/src/views/model_selector/form.rs:201

CustomModelForm uses raw-string fields, `focused_text_mut()`, an
`Up/Down` field_index walk, printable-ASCII `(' '..='~')` filter, and a
parse-on-build `build_definition()`. PROV-110 mirrors this shape but adds the
TS-specific name-editing gate (`is_editing_name`, `is_new`) and the
five-field `PROFILE_FORM_FIELDS` order.

`pub fn parse_compaction_trigger(raw) -> (Option<String>, Option<u32>)` —
codelet/fspec-tui/src/views/model_selector/form.rs:298 — REUSED verbatim for
the compactionThreshold field (no duplicate parser).

## 3. Wire type produced on save
`ProfileDefinition { base_url, api_key, context_window, max_output_tokens,
compaction_threshold_type, compaction_threshold_value }` —
codelet/rpc-types/src/lib.rs:395. Emitted via
`Action::SaveProfile { provider_id, profile_name, definition }` —
codelet/fspec-tui/src/components/mod.rs:711 (already added by PROV-109).

## 4. TS parity sources (verified, not fabricated)
- src/tui/inputHandlers/profileFormModeHandler.ts (key handling)
- src/tui/utils/providerSettingsHelpers.ts (initializeNewProfile/EditProfile,
  filterPrintableChars, DEFAULT_PROFILE_BASE_URL)
- src/tui/constants/providerSettings.ts (PROFILE_FORM_FIELDS order)
