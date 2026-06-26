# TUI-092 — Repoint default thinking-level persistence to shared `fspec-config.json`

## Depends on
- **CONFIG-008** — the shared Rust `fspec-config.json` module. TUI-092 consumes
  `load_config` / `write_config` from it.

## Problem (the discovery)

Rust module: `codelet/sessions/src/default_thinking_level_persistence.rs`
(TUI-002). It persists the chosen default thinking level to a **dedicated file**:

- File: `~/.fspec/default-thinking-level.json`
- Shape: `{ "level": <u8> }` (own struct `DefaultThinkingLevelFile`)
- Writer: whole-file overwrite.
- Reader: reads only the global file — **no project-scope merge**.

The TypeScript reference (`src/tui/config/defaultThinkingLevelConfig.ts`) instead:

- File: shared `~/.fspec/fspec-config.json`, deep-merged with `<cwd>/spec/fspec-config.json`.
- Key: nested **`tui.defaultThinkingLevel`**, integer `0..=3`.
- Writer: `loadConfig()` → spread existing → set `tui.defaultThinkingLevel` →
  `writeConfig('user', …)` — a **read-modify-write that preserves sibling keys**
  (e.g. the persisted default model / `lastUsedModel` live in the same file).
- Reader: `loadConfig()` validates `typeof number && 0..=3`, else returns null → `Off`.

### Three concrete discrepancies to fix
1. **Wrong storage location & format** — separate file `{level:u8}` instead of
   shared `fspec-config.json` under `tui.defaultThinkingLevel`. Breaks interop.
2. **No project-scope override** — Rust reads only the global file.
3. **No shared-config merge semantics** — whole-file overwrite vs read-modify-write
   preserving sibling keys.

## What is CORRECT today (parity — DO NOT change)

- Save trigger: `/thinking` dialog "D" → `set_thinking_level_default` →
  `handle_impl.rs:853` calls `save_default_thinking_level(level)`.
- Persist-always semantics (writes independent of session existence).
- Apply on **new + resumed** sessions: `session_manager.rs:575` and `:855` call
  `load_default_thinking_level()` and apply the **global default** (resume also
  resets to global default — this matches the TS reference; not a bug).
- No separate per-session persisted level (reference behaviour).
- Value encoding `0..=3`; invalid / missing / malformed → `ThinkingLevel::Off`
  (graceful degradation).

These behaviours must continue to hold after the repoint.

## Implementation plan

Rewrite the internals of
`codelet/sessions/src/default_thinking_level_persistence.rs` to delegate storage
to the CONFIG-008 shared config, **keeping the public function signatures
unchanged** so call sites in `session_manager.rs` and `handle_impl.rs` need no
edits:

```rust
// SAVE: read-modify-write preserving siblings
pub fn save_default_thinking_level_with_dirs(data_dir, cwd, level) -> Result<(), String> {
    let mut config = load_config_with_dirs(data_dir, cwd).unwrap_or(Value::Object(empty));
    // ensure config["tui"] is an object, set config["tui"]["defaultThinkingLevel"] = level as u8
    write_config_with_dirs(ConfigScope::User, &config, data_dir, cwd)
}

// LOAD: read tui.defaultThinkingLevel, validate 0..=3, else Off
pub fn load_default_thinking_level_with_dirs(data_dir, cwd) -> ThinkingLevel {
    match load_config_with_dirs(data_dir, cwd) {
        Ok(cfg) => cfg["tui"]["defaultThinkingLevel"].as_u64()
            .filter(|n| *n <= 3).map(|n| level_from_u8(n as u8))
            .unwrap_or(ThinkingLevel::Off),
        Err(_) => ThinkingLevel::Off,
    }
}
```

Notes:
- Keep `level_from_u8` mapping (`1=Low,2=Medium,3=High, else Off`).
- The existing global convenience wrappers `save_default_thinking_level(level)` /
  `load_default_thinking_level()` keep their signatures; internally they resolve
  `data_dir = get_data_dir()` and `cwd = std::env::current_dir()` and delegate to
  the `_with_dirs` cores.
- **SAVE writes to USER scope** (matches TS `writeConfig('user', …)`). Project
  scope is read-only for override purposes (matches reference).
- The dedicated `default-thinking-level.json` file and `DefaultThinkingLevelFile`
  struct are removed.

## Migration consideration (decide via Example Mapping question)

Existing users may have a `~/.fspec/default-thinking-level.json`. Options:
- (a) No migration — they re-select once (simplest; TS never had this file).
- (b) One-time read of the legacy file on load if `tui.defaultThinkingLevel` is
  absent, then ignore it thereafter.

Recommend (a) for simplicity unless the supervisor decides otherwise; capture as
a red card.

## Tests to update / add

Existing test: `codelet/sessions/tests/tui002_default_thinking_level.rs` uses
`*_with_dir` (single dir). It will need updating to the new `*_with_dirs`
(data_dir + cwd) signatures, and assertions must check the value lands under
`tui.defaultThinkingLevel` in `<data_dir>/fspec-config.json` and that **sibling
keys are preserved** across a save.

New scenarios (acceptance):
1. Saving High writes `tui.defaultThinkingLevel = 3` into `fspec-config.json`.
2. Loading reads `tui.defaultThinkingLevel` and maps to the right level.
3. Saving preserves a pre-existing sibling key (e.g. a `tui.lastUsedModel` /
   top-level key) in the same file — proves read-modify-write.
4. Project-scope `<cwd>/spec/fspec-config.json` overrides the user value on load.
5. Missing / malformed config or out-of-range value → `Off`.
6. Round trip via the global wrappers still applies on new + resumed sessions
   (existing session_manager behaviour unbroken).

## Coding standards

- Path-injectable `_with_dirs` cores; thin global wrappers.
- `Result<_, String>` for fallible save; load is infallible → `Off` on any error.
- No mocking fs — `tempfile` OS temp dirs, mirroring existing test style.
- Keep the persistence source file focused and under the 300-line guideline.
