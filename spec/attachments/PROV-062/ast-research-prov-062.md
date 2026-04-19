# PROV-062 AST Research — Existing Integration Points

Conducted via AstGrep on codelet/providers/src/ on 2026-04-17.

## Relevant existing items to integrate with

- `codelet/providers/src/oauth/engine.rs:42` — `pub fn build_sandboxed_engine(modules: Vec<RhaiModule>) -> Engine` (PROV-060). ScriptLoader will reuse this factory so compiled scripts have access to the shared sandboxed modules (http, crypto, json, oauth, time, env).

- `codelet/providers/src/oauth/script_provider.rs:21` — `pub struct ScriptProviderConfig`. Custom provider's `ProviderConfig` is a superset — it adds `models`, `system_prompt`, `tool_style`, `api_style`, `headers`, `env_prefix`, `defaults`.

- `codelet/providers/src/manager.rs:22` — `pub enum ProviderType`. PROV-067 extends this with a `Custom(String)` variant; PROV-062 only adds the loader and does not yet wire into ProviderManager.

## File layout to be added

New module `codelet/providers/src/custom/` with:

- `mod.rs` — re-exports
- `config.rs` — `ProviderConfig`, `AuthConfig`, `ModelDef`, `Defaults`, `SystemPromptConfig`, `ToolStyle`, `ApiStyle`
- `discovery.rs` — `discover_provider_configs()`, `validate_provider_config()`, `get_fspec_home_base()`
- `script_loader.rs` — `ScriptLoader` with `Mutex<HashMap<PathBuf, (SystemTime, Arc<AST>)>>` cache
- `error.rs` — `CustomProviderError` (thiserror enum with variants: Io, Parse, MissingField, NameConflict, InvalidName, ScriptNotFound, MissingDefaultModel, RhaiParseError, MissingFunction)

Each file stays under 300 lines per project convention.

## Required-function validation

The 7 functions required on each compiled `rhai::AST`:

1. `build_request`
2. `build_headers`
3. `build_url`
4. `parse_response`
5. `parse_stream_chunk`
6. `build_stream_request`
7. `map_error`

Validation iterates `AST.iter_functions()` and fails if any are missing.
