# AST Research — TUI-002 default thinking level persistence

## Goal
Persist the default thinking level and re-apply it to every new/resumed session
so an idle Rust ratatui session shows the yellow `[T:High]` badge (parity with TS).
Do NOT modify the badge renderer.

## Findings (AstGrep over Rust sources)

### Stub to extend (server)
`fn set_thinking_level_default(&self, session_id: &SessionId, level: ThinkingLevel) -> Result<(), String>`
- Location: `sessions/src/handle_impl.rs:844`
- Currently: in-memory only — `session.set_base_thinking_level(level as u8)`, errors
  if the session is unknown. Must ALSO persist (always), then apply in-memory when
  the session exists.

### Persistence template to mirror
`pub fn save_default_model_with_dir(data_dir: &Path, model: &str) -> Result<(), String>`
- Location: `sessions/src/default_model_persistence.rs:47`
- Pattern: path-injectable `_with_dir` save/load cores + global convenience wrappers
  using `codelet_common::get_data_dir()`. File under data dir, serde JSON.
- New module `default_thinking_level_persistence.rs` will mirror this exactly:
  `default-thinking-level.json` → `{ "level": u8 }`. Load validates 0..=3, returns
  `ThinkingLevel::Off` for missing/malformed/out-of-range.

### Session base thinking level API
`sessions/src/background_session.rs`:
- `set_base_thinking_level(&self, level: u8)` (line 1072) — clamps to 3.
- `get_base_thinking_level(&self) -> u8` (line 1064).
- `base_thinking_level: AtomicU8::new(0)` initialised to Off at construction (line 507).

### Session-creation paths to apply default (server-side, all transports)
`sessions/src/session_manager.rs`:
- `create_session_with_id` — `BackgroundSession::new(...)` at line 555, insert at 636.
- `create_isolated_session_with_id` — `BackgroundSession::new(...)` at line 828, insert at 907.
- Apply `session.set_base_thinking_level(load_default_thinking_level() as u8)` right
  after each `BackgroundSession::new`.

### Renderer (reference only — DO NOT MODIFY)
`fspec-tui/src/views/agent/header_build.rs::thinking_label` maps
Off→None, Low→"Low", Medium→"Med", High→"High"; `build_left_line` paints `[T:<label>]`
yellow. Already at parity. `get_thinking_level` RPC reads the base level (handle_impl.rs:891).

### Test injection pattern
`sessions/tests/prov119_default_model_persistence.rs`: uses `_with_dir` cores for pure
round-trip tests, and `codelet_common::set_data_directory(temp)` + `SessionManager::new()`
+ `handle.create_session(None)` for end-to-end. A `DATA_DIR_GUARD` Mutex serialises the
process-global data dir. Same approach for TUI-002.

## ThinkingLevel
`rpc-types/src/lib.rs:303` — `pub enum ThinkingLevel { Off, Low, Medium, High }`
(`#[default] Off`). `level as u8` → 0..3.

## Plan
1. New `sessions/src/default_thinking_level_persistence.rs` (save/load `_with_dir` + global wrappers).
2. Register `pub mod` in `sessions/src/lib.rs`.
3. `set_thinking_level_default` (handle_impl.rs): persist always, apply in-memory when session present.
4. Apply persisted default in both session-creation paths after `BackgroundSession::new`.
