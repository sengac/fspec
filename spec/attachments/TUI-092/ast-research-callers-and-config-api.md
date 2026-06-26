# TUI-092 AST Research — caller surface & CONFIG-008 API

## CONFIG-008 shared config API (codelet/common/src/fspec_config.rs)
AST pattern `pub fn $NAME(...) -> Result<Value, String>` confirms the load entry points:
- `load_config_with_dirs(data_dir: &Path, cwd: &Path) -> Result<Value, String>` (line 88)
- `load_config() -> Result<Value, String>` (line 96)

Also present (read manually): `write_config_with_dirs(scope: ConfigScope, config: &Value, data_dir: &Path, cwd: &Path) -> Result<(), String>`, `write_config(...)`, `enum ConfigScope { User, Project }`. Project scope deep-merges OVER user on load via `deep_merge`.

## Callers of default_thinking_level_persistence (codelet/sessions/src)
AST pattern `crate::default_thinking_level_persistence::$FN(...)` found exactly three call sites, all on the GLOBAL WRAPPERS (no `_with_dir(s)` calls in src):
- session_manager.rs:575 → `load_default_thinking_level()`
- session_manager.rs:855 → `load_default_thinking_level()`
- handle_impl.rs:853   → `save_default_thinking_level(level)`

### Conclusion
Keeping `save_default_thinking_level(level) -> Result<(), String>` and
`load_default_thinking_level() -> ThinkingLevel` signatures unchanged means the
three call sites need NO edits. The `_with_dir` single-dir cores are only used by
the integration test `tui002_default_thinking_level.rs`, which we update to the
new `_with_dirs` (data_dir + cwd) signatures.
