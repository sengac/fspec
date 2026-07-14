# AST Research for RPC-424: Model Parsing Extraction

## Duplicated Code Locations

### Function 1: `create_session_with_id` (lines ~630-669)
- **File**: `codelet/sessions/src/session_manager.rs`
- **Model validation**: L630-635 — checks `model.contains('/')` and `model.is_empty()`
- **is_profile_model**: L637 — `model.contains(':') && model.find(':') < model.find('/')`
- **is_codex_model**: L638 — `model.starts_with("codex/")`
- **registry_provider, model_part**: L640-653 — if/else block for profile vs standard parsing
- **Empty check**: L655-660 — validates neither is empty
- **is_custom_model**: L662-664 — checks custom provider registry
- **provider_id, model_id tuple**: L666-669

### Function 2: `create_session_from_manifest` (lines ~931-970)
- **File**: `codelet/sessions/src/session_manager.rs`
- **Model validation**: L931-936 — identical to L630-635
- **is_profile_model**: L938 — identical to L637
- **is_codex_model**: L939 — identical to L638
- **registry_provider, model_part**: L941-954 — identical to L640-653
- **Empty check**: L956-961 — identical to L655-660
- **is_custom_model**: L963-965 — identical to L662-664
- **provider_id, model_id tuple**: L967-970

### Function 3: `create_isolated_session_with_id` (lines ~1211-1246)
- **File**: `codelet/sessions/src/session_manager.rs`
- **Model validation**: L1211-1216 — identical
- **is_profile_model**: L1218 — identical
- **is_codex_model**: L1219 — identical
- **registry_provider, model_part**: L1221-1234 — identical
- **Empty check**: L1236-1241 — identical
- **is_custom_model**: **NOT PRESENT** — isolated function omits this check
- **provider_id, model_id tuple**: L1243-1246

## Existing Modules
- `codelet/sessions/src/model_resolution.rs` — contains `apply_model_selection` but NOT model parsing
- No existing `model_parsing.rs` module

## Proposed Extraction
Create `codelet/sessions/src/model_parsing.rs` with:
- `pub struct ModelParseResult` containing: `registry_provider: String`, `model_part: String`, `is_profile_model: bool`, `is_codex_model: bool`, `is_custom_model: bool`
- `pub fn parse_model_string(model: &str) -> Result<ModelParseResult, String>`

## Call Sites to Update
1. `create_session_with_id` — replace L630-669 with `parse_model_string(model)?`
2. `create_session_from_manifest` — replace L931-970 with `parse_model_string(model)?`
3. `create_isolated_session_with_id` — replace L1211-1246 with `parse_model_string(model)?`
