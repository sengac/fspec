# AST Research — PROV-086 cred:: Rhai namespace

## Existing module builder pattern (building_blocks.rs)

AST pattern: `fn $NAME() -> RhaiModule { $$$BODY }` (zero-arg module builders)

Matches:
- `codelet/providers/src/oauth/building_blocks.rs:26` — `fn build_http_module() -> RhaiModule`
- `codelet/providers/src/oauth/building_blocks.rs:96` — `fn build_crypto_module() -> RhaiModule`
- `codelet/providers/src/oauth/building_blocks.rs:129` — `fn build_json_module() -> RhaiModule`
- `codelet/providers/src/oauth/building_blocks.rs:168` — `fn build_oauth_module() -> RhaiModule`

Conclusion: existing module builders take no arguments. `build_cred_module` MUST
accept a `provider_name: String` so each registered native fn can capture it by
clone.  Adding it to the `Vec<RhaiModule>` returned by `register_all_modules()`
would force callers to supply a provider — instead a new function
`register_all_modules_for_provider(provider_name: &str)` will be introduced that
returns the four existing modules plus `build_cred_module(provider_name.to_string())`.

## Engine factory (engine.rs)

AST pattern: `pub fn build_sandboxed_engine($$$ARGS) -> Engine { $$$BODY }`

Matches:
- `codelet/providers/src/oauth/engine.rs:51` — the sandboxed engine factory.

`build_default_engine()` lives at line 79 and calls `register_all_modules()`.
We will add a parallel `build_provider_engine(provider_name: &str) -> Engine`
that calls `register_all_modules_for_provider(provider_name)`.

## fspec_home helpers

AST pattern: `fn get_fspec_home() -> PathBuf { $$$BODY }`

Matches:
- `codelet/providers/src/claude_auth.rs:31` — honours `FSPEC_HOME`, falls back to
  `$HOME/.fspec/credentials`.
- `codelet/providers/src/copilot/auth.rs:154` — identical pattern.

Both are **private**.  PROV-086 introduces a single shared
`pub fn fspec_home() -> PathBuf` inside `oauth/building_blocks.rs` that mirrors
this logic so the `cred::` module has a well-defined credentials directory.

## Test harness for FSPEC_HOME

`codelet/providers/tests/fixtures/mod.rs:90` provides `setup_fspec_home()` which
returns `(TempDir, FspecHomeGuard)`.  The guard restores the previous env on
drop — this is the idiomatic way to isolate tests.  The new unit tests live
*inside* `src/` (so we use `std::env::set_var` directly in a `#[test]` guarded
by serialization through the test harness) OR we create an integration test
under `tests/` that uses `setup_fspec_home`.

PROV-086 plan: put unit-style tests for `cred::` in
`codelet/providers/src/oauth/cred_module_tests.rs` (compiled only under
`#[cfg(test)]`) and set `FSPEC_HOME` locally via `tempfile::TempDir` +
`std::env::set_var`.  Tests that mutate env vars are kept sequential by using
a lazily-initialised `Mutex` guard pattern local to this test file.

## Existing users of register_all_modules()

Grep matches:
- `src/oauth/engine.rs:80` — `build_default_engine()` uses it.
- `src/custom/script_loader.rs:61`
- `src/custom/custom_provider.rs:169`
- `tests/custom_config_and_loader_tests.rs:{307,329,360,392,423}`
- `tests/custom_system_prompts_test_helpers.rs:15`

None of these call sites have access to the provider name at the time they
build the engine, and they do not need the `cred::` module (they already worked
before). Keeping `register_all_modules()` unchanged preserves backward
compatibility for all of them.

## Security implications

- The `name` argument to `cred::read / write / delete / path` is validated
  *before* any `PathBuf` construction, so `..` / `/` / absolute paths cannot
  escape — the validator simply requires `name == provider_name`.
- `path()` joins `fspec_home()` with `format!("{name}.json")` only after
  validation, so the final path is always `<fspec_home>/<provider>.json`.
- Permissions are set to `0o600` on Unix using
  `std::os::unix::fs::PermissionsExt`.
