# AST Research — CONFIG-008 persistence pattern

## Goal
Mirror the path-injectable `*_with_dir` core + thin global wrapper pattern, and
reuse `codelet_common::get_data_dir()` for the shared fspec-config.json module.

## Pattern: path-injectable cores + global wrappers (Result<_, String>)
Searched `codelet/sessions/src/default_model_persistence.rs` for:
`pub fn $NAME($$$ARGS) -> Result<$RET, String> { $$$BODY }`

Matches:
- `save_default_model_with_dir(data_dir: &Path, model: &str) -> Result<(), String>` (line 47) — path-injectable core
- `save_default_model(model: &str) -> Result<(), String>` (line 80) — thin global wrapper calling `codelet_common::get_data_dir()?`

Conclusion: CONFIG-008 mirrors this exactly with `load_config_with_dirs(data_dir, cwd)` /
`write_config_with_dirs(scope, config, data_dir, cwd)` cores and `load_config()` / `write_config(scope, config)`
wrappers that resolve `get_data_dir()` + `std::env::current_dir()`.

## Data dir source of truth
Searched `codelet/common/src/data_dir.rs` for:
`pub fn get_data_dir() -> Result<PathBuf, String> { $$$BODY }`

Match:
- `get_data_dir() -> Result<PathBuf, String>` (line 37) — lives in codelet-common.

Conclusion: new module belongs in `codelet-common` (same crate as `get_data_dir`); global
wrappers call it. serde_json is already a dependency; tempfile is in dev-deps for tests.
