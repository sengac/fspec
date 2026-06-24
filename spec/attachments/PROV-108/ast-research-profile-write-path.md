# PROV-108 AST Research — Backend profile write path

Goal: add `save_profile` / `delete_profile` mirroring the RPC-347 custom-model
write stack across 5 crates. AST queries below pin the exact sites to mirror.

## Wire type (rpc-types)
AstGrep `rust`: `pub struct CustomModelDefinition { $$$FIELDS }`
- Hit: `codelet/rpc-types/src/lib.rs:366`
- New `ProfileDefinition` struct added beside it; same `#[cfg_attr(feature="napi", napi(object))]`
  + `Serialize, Deserialize, Default, PartialEq, Eq` derive convention.
- Flat compaction fields (`compaction_threshold_type` / `_value`) mirror the
  CustomModelDefinition convention so `napi(object)` stays a flat struct.

## Persistence core (sessions/profile_sections.rs)
AstGrep `rust`: `fn save_custom_model_at($$$ARGS) -> std::io::Result<()> { $$$BODY }`
- Hit: `codelet/sessions/src/profile_sections.rs:300`
- Reuses helpers `fspec_user_dir` (L183), `read_config_value` (L377),
  `write_config_value` (L385) — promoted to `pub(crate)` for the NEW module
  `profile_persistence.rs` (avoids bloating profile_sections.rs, already 485 prod LoC).

## Conversion (sessions/conversions.rs)
- `custom_model_def_from_wire` (L132) is the template for `profile_def_from_wire`,
  folding flat compaction fields into `profile_sections::CompactionThreshold`.

## SessionManagerHandle trait + override
AstGrep `rust`: `fn add_custom_model(&self, $$$ARGS) -> Result<(), String> { $$$BODY }`
- Hit: `codelet/sessions/src/handle_impl.rs:1059` (concrete override)
- Trait default no-op: `codelet/core/src/session_manager_handle.rs:166-202`
- openai-guard returns Err on save (handle_impl L1068); delete is a no-op for non-openai.

## RPC FspecService
AstGrep `rust`: `async fn delete_custom_model(self, $$$ARGS) -> Result<(), String> { $$$BODY }`
- Hit: `codelet/rpc/src/lib.rs:1095` (impl), trait decl at L187-211.
- No-handle path returns `Ok(())`.

## NAPI bindings
- `codelet/napi/src/models/napi_bindings.rs:306-344` add/update/delete_custom_model
  are the thin-pass-through template for save/delete_profile bindings.

## Reference integration test
- `codelet/sessions/tests/custom_model_rpc_surface.rs` — offline pattern using
  `FSPEC_USER_DIR` temp dir + `#[serial]`; mirrored as `profile_rpc_surface.rs`.
