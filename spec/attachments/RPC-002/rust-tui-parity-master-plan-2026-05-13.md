# RPC-013..022 — Master plan: Bring the Rust TUI to parity with TS

> Series of cards landed on 2026-05-13 after reviewing screenshots and
> walking the codelet/fspec-tui codebase post-RPC-012. Each card is a
> child of **RPC-002** (parent epic) and follows the
> RPC-005..012 invariants:
>
> - All new functionality goes through the `FspecBackend` trait
>   (codelet/fspec-tui/src/transport/mod.rs).
> - Both `EmbeddedFspecBackend` and `WebSocketFspecBackend` implement
>   every new method.
> - The shared service impl lives in `codelet/rpc/src/lib.rs::FspecServiceImpl`.
> - Shared types live in `codelet/rpc-types/src/lib.rs` and gate `napi`
>   feature attributes for cross-binding reuse.
> - All TypeScript code remains operational — the new RPC methods are
>   additive; NAPI exports are additive; no TS file changes are required
>   unless explicitly noted.
> - File-size discipline from RPC-012 rule [10] is maintained — every
>   new module file kept under 300 LoC.

## Card map

### Board side (5 cards)

| ID | Title | Pts | Depends on | Touches RPC/NAPI? |
|---|---|---|---|---|
| RPC-013 | View-aware footer (Board vs Agent) | 1 | — | No |
| RPC-014 | Rich box-drawing Kanban grid + work-unit details strip | 8 | RPC-013 | `WorkUnitInfo.attachments` (rpc-types) |
| RPC-015 | BoardView header (Logo + CheckpointStatus + KeybindingShortcuts) | 5 | RPC-014 | New: `checkpoint_counts()` |
| RPC-016 | Per-column scroll viewport + indicators + keyboard nav | 8 | RPC-014 | `WorkUnitInfo.last_state_change_at` |
| RPC-017 | Priority reorder persistence | 5 | RPC-014 | New: `move_work_unit_up/down` |

### Agent side (5 cards)

| ID | Title | Pts | Depends on | Touches RPC/NAPI? |
|---|---|---|---|---|
| RPC-018 | SessionHeader + SessionFooter | 8 | — | New: `get_model_info`, `get_thinking_level`, `get_workspace_info` |
| RPC-019 | Multi-line input + VirtualList scrollback | 8 | — | No |
| RPC-020 | Slash command palette + @file search popup | 8 | RPC-019 | New: `search_files` |
| RPC-021 | Multi-session + Shift navigation + history | 13 | RPC-018, RPC-019, RPC-020 | New: `persistence_get/add/search_history`. **Should split during specifying.** |
| RPC-022 | Modal dialogs (ModelSelector, ThinkingLevel, RoleBanner) | 8 | RPC-018, RPC-020 | New: `list_providers`, `set_session_model`, `set_thinking_level`, `get/set_session_role` |

**Total**: 72 story points across 10 cards (≈45 hours estimated).

## Suggested execution order

1. **RPC-013** (1pt) — smallest, removes a visible regression immediately.
2. **RPC-014** (8pt) — biggest visible upgrade; foundation for RPC-015/016/017.
3. **RPC-015, 016, 017** in parallel after RPC-014 (any order).
4. **RPC-018** (8pt) — agent-side foundation; visible header/footer.
5. **RPC-019** (8pt) — input/scrollback widgets needed by 20+22.
6. **RPC-020** (8pt) — overlays.
7. **RPC-022** (8pt) — modal dialogs.
8. **RPC-021** (split + work each child) — last because it requires
   the others to be in place.

## RPC/NAPI boundary cheat sheet

### New shared types in `codelet/rpc-types/src/lib.rs`

```rust
// Extended (additive fields on existing struct)
pub struct WorkUnitInfo {
    /* existing... */
    pub attachments: Vec<String>,           // RPC-014
    pub last_state_change_at: Option<String>, // RPC-016 (ISO-8601)
}

// New
pub struct CheckpointCounts { pub manual: u32, pub auto: u32 }              // RPC-015
pub struct ModelInfo { /* RPC-018 */ }
pub enum ThinkingLevel { Off, Low, Medium, High }                            // RPC-018
pub struct WorkspaceInfo { pub cwd: String, pub git_branch: Option<String> } // RPC-018
pub struct HistoryMatch { /* RPC-021 */ }
pub struct ProviderInfo { /* RPC-022 */ }
pub struct ModelEntry { /* RPC-022 */ }
```

### New RPC methods in `codelet/rpc/src/lib.rs::FspecService`

```rust
async fn checkpoint_counts() -> CheckpointCounts;                       // RPC-015
async fn move_work_unit_up(id: String);                                 // RPC-017
async fn move_work_unit_down(id: String);                               // RPC-017
async fn get_model_info(session: SessionId) -> ModelInfo;               // RPC-018
async fn get_thinking_level(session: SessionId) -> ThinkingLevel;        // RPC-018
async fn get_workspace_info() -> WorkspaceInfo;                          // RPC-018
async fn search_files(prefix: String, limit: u32) -> Vec<String>;        // RPC-020
async fn persistence_get_history(session: SessionId, limit: u32) -> Vec<String>; // RPC-021
async fn persistence_add_history(session: SessionId, text: String);      // RPC-021
async fn persistence_search_history(query: String) -> Vec<HistoryMatch>; // RPC-021
async fn list_providers() -> Vec<ProviderInfo>;                          // RPC-022
async fn set_session_model(session: SessionId, provider: String, model: String); // RPC-022
async fn set_thinking_level(session: SessionId, level: ThinkingLevel);   // RPC-022
async fn get_session_role(session: SessionId) -> Option<String>;         // RPC-022
async fn set_session_role(session: SessionId, role: Option<String>);     // RPC-022
```

### Additive NAPI exports (additive only — TS keeps using its existing surface)

The following NEW NAPI exports converge with the existing TS code on the
SAME `codelet_core` / `codelet_git` helpers, so both UIs stay in sync:

```ts
napi.count_checkpoints(cwd: string): CheckpointCounts                    // RPC-015
napi.move_work_unit_up(cwd: string, id: string): void                    // RPC-017
napi.move_work_unit_down(cwd: string, id: string): void                  // RPC-017
napi.get_model_info(sessionId: string): ModelInfo                        // RPC-018
napi.get_workspace_info(cwd: string): WorkspaceInfo                      // RPC-018
napi.search_files(cwd: string, prefix: string, limit: number): string[]  // RPC-020
// RPC-021 history NAPI surface already exists — kept unchanged
// RPC-022 model/role NAPI surface already exists — kept unchanged
```

## Invariants every card must preserve

From RPC-009 / 011 / 012:

1. **Single-task mutation** — all store mutations happen inside
   `App::dispatch` on the App task. No `Mutex` / `RwLock` / atomics on
   store types.
2. **Host-supplied tokio runtime** — only `tokio::spawn`; never
   `Runtime::new` / `Builder`.
3. **Loopback-only WebSocket bind** — no LAN exposure.
4. **`codelet-napi` is NOT a dep** of `codelet-fspec-tui`.
5. **File-size discipline** — every new module file kept under 300 LoC.
6. **Source-shape regression** — `tests/source_shape_*` scans must pass
   for every new file.
7. **Cross-transport parity** — every new behavior tested against BOTH
   `EmbeddedFspecBackend` and `WebSocketFspecBackend` (mirror the
   RPC-009 pattern).

## TypeScript preservation guarantee

After ALL 10 cards land:
- `src/tui/components/*.tsx` — UNCHANGED (no diffs).
- `src/tui/store/*.ts` — UNCHANGED.
- `src/tui/hooks/*.ts` — UNCHANGED.
- `src/commands/*.ts` — UNCHANGED.
- Existing NAPI exports — UNCHANGED.

The TS Ink TUI continues to work exactly as it does today. The Rust
ratatui TUI catches up to feature parity.
