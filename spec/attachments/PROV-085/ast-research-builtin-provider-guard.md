# AST Research: BUILTIN_PROVIDER_NAMES guard & shadowing touchpoints

Work unit: PROV-085

## Target symbols found

1. `const BUILTIN_PROVIDER_NAMES: &[&str]` — `codelet/providers/src/custom/config.rs:17`
   - Consumed at `config.rs:310` inside `ProviderConfig::validate`, emitting `CustomProviderError::NameConflict`.
   - Remove the constant AND the guard block; `NameConflict` variant stays in `error.rs` for API stability.

2. `fn custom_provider_registered(slug: &str) -> bool` — `codelet/providers/src/manager.rs:111`
   - Currently just walks `discover_provider_configs()` and checks name match.
   - Will be used by `ProviderType::from_str` / `map_provider_id_to_type` to short-circuit built-in slugs into `ProviderType::Custom` when a shadowing config is registered.

3. `impl FromStr for ProviderType :: fn from_str(...)` — `manager.rs:43`
   - Built-in slugs ("claude", "openai", "codex", "gemini", "zai", "github-copilot", "copilot") currently match first, before the `other => custom_provider_registered` fallthrough. We must reorder so the custom registry is consulted before the hardcoded match, unless `FSPEC_DISABLE_SCRIPT_SHADOWING` is set.

4. `fn map_provider_id_to_type` — `manager.rs:503`
   - Mirrors `from_str` for models.dev provider IDs (`anthropic`, `openai`, `google`, `zai`, `codex`, `github-copilot`, `copilot`). Same reordering required.

## Tests touching the guard

- `codelet/providers/tests/custom_config_and_loader_tests.rs:97-123` — `reject_provider_name_that_collides_with_a_builtin_provider`. Must be rewritten to assert loading SUCCEEDS (scenario: "Load a custom provider config named 'claude' without NameConflict").

## Shadowing decision

- Default: custom shadows built-in.
- `FSPEC_DISABLE_SCRIPT_SHADOWING=1` → skip shadow lookup, falling through to the hardcoded match arm. Checked inside a single helper (`should_shadow_builtins()`) consumed by both `FromStr` and `map_provider_id_to_type`.

## No other call sites

No other consumers reference `BUILTIN_PROVIDER_NAMES` or rely on `NameConflict` outside `custom_config_and_loader_tests.rs`.
