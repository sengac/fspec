# AST Research — TUI-001 get_model_info data feed

## Goal
Resolve friendly model name + capability flags + compaction-threshold size badge
in the SessionHeader by fixing the server-side data feed (no renderer changes).

## Findings (AstGrep over Rust sources)

### Stub to fix
`fn get_model_info(&self, session_id: &SessionId) -> codelet_rpc_types::ModelInfo`
- Location: `sessions/src/handle_impl.rs:870`
- Currently hardcodes `supports_reasoning: false`, `supports_vision: false`,
  `display_name: model_id` (raw slug), and uses raw `cached_context_window`.
- `get_session_model` (same file, ~184) already reads `provider_id` + `model_id`
  via `s.provider_id.read()` / `s.model_id.read()` and exposes
  `cached_compaction_threshold` — the exact data needed.

### Registry lookup helper to reuse
`pub fn cloud_model_entries(...) -> Vec<ModelEntry>` — `sessions/src/cloud_models.rs:46`
- Already does `canonical_to_models_dev(canonical_id)` slug mapping (gemini→google)
  and `registry.list_models(dev_id)` / `m.name` / `m.reasoning` /
  `m.has_capability(Capability::Vision)`.
- `canonical_to_models_dev` is `pub` and reusable.

### Registry API
`providers/src/models/registry.rs:78` — `pub fn get_model(&self, provider, model) -> Result<&ModelInfo, _>`
Catalog `ModelInfo` (`providers/src/models/types.rs`): `.name`, `.reasoning`,
`.has_capability(Capability::Vision)`, `.limit.context`.

### Wire struct (add field)
`rpc-types/src/lib.rs:285` — `pub struct ModelInfo` currently has
`display_name`, `supports_reasoning`, `supports_vision`, `context_window`.
Add `compaction_threshold: u32`.

### Size-badge selection
`fspec-tui/src/views/agent/header_build.rs:74` — uses `m.context_window` directly;
must become `if compaction_threshold > 0 { compaction_threshold } else { context_window }`
fed through `format_context_window` (already in same file, line 200).

### ModelInfo literal constructions to update (new field)
- `sessions/src/handle_impl.rs:881`
- `rpc/src/lib.rs` (default impl path — uses `ModelInfo::default()`, no literal)
- `fspec-tui/tests/app_bootstrap_rpc018.rs:85,128`
- `fspec-tui/tests/view_agent_unit_rpc018.rs:114,253`
- `fspec-tui/tests/view_agent_unit_rpc029.rs:188,220,285,428,480,531`
(napi/tests use `codelet_providers::models::ModelInfo`, a DIFFERENT struct — untouched.)

## Plan
1. Add `compaction_threshold: u32` to `codelet_rpc_types::ModelInfo` (Default derives keep 0).
2. Add pure helper `resolve_model_info(registry, provider_id, model_id, context_window, compaction_threshold)` in `cloud_models.rs` reusing `canonical_to_models_dev` + `registry.get_model`.
3. Wire `get_model_info` in `handle_impl.rs` to build the registry, read provider/model + cached_compaction_threshold, and call the helper.
4. Update `header_build.rs::build_left_line` size-badge value selection.
5. Update all `ModelInfo { .. }` literals in tests to add the new field.
