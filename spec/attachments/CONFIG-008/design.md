# CONFIG-008 — Shared Rust `fspec-config.json` module

## Why

The Rust TUI port persists settings (default thinking level, default model) into
**dedicated single-purpose JSON files** under `~/.fspec`:

- `~/.fspec/default-thinking-level.json` → `{ "level": <u8> }`
- `~/.fspec/default-model.json` → `{ "model": <string> }`

The TypeScript reference we are porting from instead uses **one shared config
file** with read-modify-write semantics and a **two-scope deep merge**:

- User scope: `~/.fspec/fspec-config.json`
- Project scope: `<cwd>/spec/fspec-config.json` (overrides user on deep merge)

Reference: `src/utils/config.ts` (`loadConfig`, `writeConfig`, `getFspecUserDir`).

Because the Rust port uses isolated files, settings written by the TS build (or
by the `fspec` CLI) are **not read by Rust and vice-versa** — a port-fidelity /
interoperability break. This card builds the missing foundation: a shared Rust
config module that mirrors the TS behaviour. Consumers (CONFIG-008's dependent
TUI-092) repoint onto it.

## Reference behaviour to mirror (`src/utils/config.ts`)

### `loadConfig(cwd = process.cwd())`
1. Read user file `~/.fspec/fspec-config.json`.
2. Read project file `<cwd>/spec/fspec-config.json`.
3. Deep-merge: **project overrides user** (objects merged recursively; non-object
   values and arrays replaced wholesale).
4. Missing file → treated as empty `{}` (silent fallback).
5. Empty/whitespace-only file → empty `{}`.
6. **Invalid JSON → throw** an error `Invalid JSON in <path>: <msg>` (NOT silent).

### `writeConfig(scope, config, cwd)`
- `scope = 'user'` → `~/.fspec/fspec-config.json`.
- `scope = 'project'` → `<cwd>/spec/fspec-config.json`.
- Creates the parent directory (`mkdir -p`).
- Writes pretty JSON (2-space indent).

### Deep merge rule (`deepMerge`)
```
for each key in source:
  if source[key] is a non-null, non-array object:
      result[key] = deepMerge(target[key] || {}, source[key])
  else:
      result[key] = source[key]   // arrays and scalars replace
```

## Proposed Rust API

New module: `codelet/common/src/fspec_config.rs` (exported from
`codelet/common/src/lib.rs`). It belongs in `codelet-common` because both
`sessions` (persistence) and other crates may consume it, and `get_data_dir()`
already lives there.

Use `serde_json::Value` as the in-memory config representation (mirrors the
untyped JS object). Suggested signatures:

```rust
/// User-scope config path: <data_dir>/fspec-config.json
pub fn user_config_path(data_dir: &Path) -> PathBuf;

/// Project-scope config path: <cwd>/spec/fspec-config.json
pub fn project_config_path(cwd: &Path) -> PathBuf;

/// Load a single file. Missing/empty -> Ok(Value::Object(empty)).
/// Invalid JSON -> Err("Invalid JSON in <path>: <msg>").
fn load_config_file(path: &Path) -> Result<Value, String>;

/// Deep-merge `source` over `target` (project over user).
fn deep_merge(target: Value, source: Value) -> Value;

/// Load + deep-merge user then project scope.
/// `cwd` selects the project file. data_dir from get_data_dir().
pub fn load_config_with_dirs(data_dir: &Path, cwd: &Path) -> Result<Value, String>;

/// Convenience wrapper using get_data_dir() + std::env::current_dir().
pub fn load_config() -> Result<Value, String>;

pub enum ConfigScope { User, Project }

/// Write a whole config Value to the chosen scope. Creates parent dir.
pub fn write_config_with_dirs(scope: ConfigScope, config: &Value, data_dir: &Path, cwd: &Path) -> Result<(), String>;

pub fn write_config(scope: ConfigScope, config: &Value) -> Result<(), String>;
```

The `_with_dirs` cores are **path-injectable** so tests use OS temp dirs (no
global `set_data_directory`, no `$HOME` mutation) — matching the existing
`*_with_dir` pattern in `default_model_persistence.rs`.

## Acceptance scope (rules to capture in Example Mapping)

1. User-only config loads its keys when no project file exists.
2. Project config deep-merges over user config (project wins per key).
3. Nested objects merge recursively; sibling keys at each level are preserved.
4. Arrays and scalar values replace (not merge).
5. Missing files load as empty object (no error).
6. Empty / whitespace-only file loads as empty object.
7. Invalid JSON returns an error naming the offending path.
8. `write_config(User, ...)` writes `<data_dir>/fspec-config.json`, creating the dir.
9. `write_config(Project, ...)` writes `<cwd>/spec/fspec-config.json`, creating the dir.
10. Round-trip: write then load returns the written value (merged with the other scope).

## Constraints / coding standards

- No `unwrap()` in library paths that can fail at runtime — surface `Result<_, String>`.
- Path-injectable `_with_dirs` cores + thin global convenience wrappers.
- Tests: Rust integration tests under `codelet/common/tests/` using `tempfile`
  (OS temp dirs). No mocking of fs. Mirror the style of
  `codelet/sessions/tests/tui002_default_thinking_level.rs`.
- Keep the source file focused; module under the common crate.

## Out of scope

- Repointing thinking-level persistence (that is TUI-092).
- Migrating `default-model.json` (a possible later follow-up; note it has the
  identical divergence).
