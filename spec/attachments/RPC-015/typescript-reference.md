# RPC-015 — BoardView header: FSPEC Logo + CheckpointStatus + KeybindingShortcuts

## TypeScript reference

### Header structure
`src/tui/components/UnifiedBoardLayout.tsx:360-380`

```tsx
{/* Header - 4 rows with Logo + CheckpointStatus/KeybindingShortcuts */}
<Box flexDirection="row" height={4}>
  <Box flexDirection="column" width={1}>
    <Text>│</Text><Text>│</Text><Text>│</Text><Text>│</Text>
  </Box>
  <Box flexGrow={1} flexDirection="row" paddingX={1}>
    <Logo />
    <Box flexGrow={1} flexDirection="column">
      <CheckpointStatus manualCount={checkpointCounts.manual} autoCount={checkpointCounts.auto} />
      <KeybindingShortcuts />
    </Box>
  </Box>
  <Box flexDirection="column" width={1}>
    <Text>│</Text><Text>│</Text><Text>│</Text><Text>│</Text>
  </Box>
</Box>
```

### Logo
`src/tui/components/Logo.tsx`

Renders the multi-line ASCII art `FSPEC` block (4 rows tall) — see the
TS screenshot at `~/Desktop/typescript-unified-board.png` top-left.

### CheckpointStatus
`src/tui/components/CheckpointStatus.tsx`

Renders `Checkpoints: {manual} Manual, {auto} Auto` on one row.

### KeybindingShortcuts
`src/tui/components/KeybindingShortcuts.tsx`

Renders the top-level navigation chord shortcuts:
`C Checkpoints ◆ F Changed Files ◆ D FOUNDATION.md ◆ / New Agent`

### Checkpoint count loading
`src/tui/store/fspecStore.ts:261-294`

```ts
loadCheckpointCounts: async () => {
  const cwd = get().cwd;
  const counts = await countCheckpoints(cwd);
  set(state => { state.checkpointCounts = counts; });
}
```

`countCheckpoints` lives in `src/utils/checkpoint.ts` (need to verify) and
returns `{ manual: number; auto: number }` by reading
`refs/fspec-checkpoints/*/auto-*` vs `refs/fspec-checkpoints/*/<name>`
from `.git/refs/`.

### NAPI surface
`codelet/napi/src/git.rs:53-416` already exposes:
- `get_checkpoint_file_diff`
- `create_ghost_checkpoint`
- `restore_ghost_checkpoint`
- `list_ghost_checkpoints`
- `delete_ghost_checkpoint`
- `get_checkpoint_diff_files`

The TS code currently iterates git refs in pure JS — the equivalent Rust
logic lives in `codelet_git::ghost_commit` and needs a new NAPI/RPC wrapper.

## Current Rust state

The Rust BoardView has NO header — only the column grid (see RPC-014). The
`BoardStore` has no `checkpoint_counts` field.

## Target Rust behavior

### New BoardStore fields

Add to `codelet/fspec-tui/src/store/board.rs`:
```rust
pub struct BoardStore {
    // existing fields...
    checkpoint_counts: CheckpointCounts,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CheckpointCounts {
    pub manual: u32,
    pub auto: u32,
}

impl BoardStore {
    pub fn checkpoint_counts(&self) -> CheckpointCounts { self.checkpoint_counts }
    pub fn set_checkpoint_counts(&mut self, counts: CheckpointCounts) {
        self.checkpoint_counts = counts;
    }
}
```

### New Action variant

```rust
pub enum Action {
    // existing...
    CheckpointCountsLoaded(CheckpointCounts),
}
```

### New FspecBackend method

`codelet/fspec-tui/src/transport/mod.rs`:
```rust
#[async_trait]
pub trait FspecBackend: Send + Sync {
    // existing methods...

    /// RPC-015: count manual + auto checkpoints across all work units.
    /// Mirrors the TS `countCheckpoints(cwd)` helper used by
    /// `useFspecStore.loadCheckpointCounts`.
    async fn checkpoint_counts(&self) -> Result<CheckpointCounts>;
}
```

### New RPC method

`codelet/rpc/src/lib.rs`:
```rust
#[tarpc::service]
pub trait FspecService {
    // existing methods...

    /// Return manual + auto checkpoint counts.
    async fn checkpoint_counts() -> CheckpointCounts;
}
```

### Shared type
`codelet/rpc-types/src/lib.rs`:
```rust
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointCounts {
    pub manual: u32,
    pub auto: u32,
}
```

### Service impl
`codelet/rpc/src/lib.rs` `FspecServiceImpl::checkpoint_counts` delegates to
`codelet_git::ghost_commit::count_checkpoints(cwd)` (new helper).

### NAPI surface preservation

Add a NAPI wrapper in `codelet/napi/src/git.rs`:
```rust
#[napi]
pub fn count_checkpoints(cwd: String) -> napi::Result<CheckpointCounts> {
    codelet_git::ghost_commit::count_checkpoints(&cwd)
        .map_err(to_napi_err)
}
```

This way the TS code can switch from its current pure-JS implementation to
the shared Rust helper at its own pace — but both paths use the same
`codelet_git::ghost_commit::count_checkpoints` function so they cannot drift.

### Three new widgets in `codelet/fspec-tui/src/views/board/`

1. `logo.rs` — paints the multi-line ASCII art `FSPEC` block. Mirror
   `src/tui/components/Logo.tsx` glyph-for-glyph.
2. `checkpoint_status.rs` — paints `Checkpoints: {manual} Manual, {auto} Auto`
   on one row.
3. `keybinding_shortcuts.rs` — paints `C Checkpoints ◆ F Changed Files ◆ D FOUNDATION.md ◆ / New Agent`.

### App-level wiring

`App::bootstrap` (`codelet/fspec-tui/src/app/bootstrap.rs`) fires off a
`backend.checkpoint_counts()` call alongside `list_work_units()` and emits
`Action::CheckpointCountsLoaded(counts)` when it returns.

`App::dispatch` handles `Action::CheckpointCountsLoaded` by calling
`board_store.set_checkpoint_counts(counts)`.

## RPC/NAPI boundary contract

```
TS UI → useFspecStore.loadCheckpointCounts() → countCheckpoints(cwd)  [TS, today]
                                          OR → napi.count_checkpoints(cwd)  [TS, after this card]

Rust TUI → FspecBackend::checkpoint_counts() → FspecService::checkpoint_counts() [tarpc]
                                            → codelet_git::ghost_commit::count_checkpoints(cwd) [shared impl]
```

Both the existing TS-pure path and the new TUI path eventually converge in
`codelet_git::ghost_commit::count_checkpoints`. The TS path keeps working
unchanged because the NAPI wrapper is purely additive.

## Existing TypeScript behavior preserved

- `src/tui/components/UnifiedBoardLayout.tsx` — UNCHANGED.
- `src/tui/components/Logo.tsx` — UNCHANGED.
- `src/tui/components/CheckpointStatus.tsx` — UNCHANGED.
- `src/tui/components/KeybindingShortcuts.tsx` — UNCHANGED.
- `src/tui/store/fspecStore.ts` — UNCHANGED (keeps using its current
  pure-JS `countCheckpoints` helper).
- `src/utils/checkpoint.ts` (or wherever `countCheckpoints` lives) — UNCHANGED.

The new NAPI export `count_checkpoints` is **additive** — nothing
deprecated.

## Acceptance criteria sketch

- A 4-row header strip is visible at the top of the BoardView, between the
  top border and the work-unit details strip from RPC-014.
- Left of the header: the `FSPEC` ASCII art logo block.
- Right of the header (row 1): `Checkpoints: {manual} Manual, {auto} Auto`
  with live counts from the new RPC method.
- Right of the header (row 2): `C Checkpoints ◆ F Changed Files ◆ D FOUNDATION.md ◆ / New Agent`.
- The counts update when `App::dispatch` receives `Action::CheckpointCountsLoaded`.
- A new `codelet_git::ghost_commit::count_checkpoints(&Path) -> Result<CheckpointCounts>`
  function exists and is used by BOTH the new NAPI export AND the new tarpc
  `FspecService::checkpoint_counts` implementation.
- The NAPI export `napi::count_checkpoints(cwd: String)` is exposed in
  `codelet/napi/src/git.rs` and re-exported through `codelet/napi/index.d.ts`.
- All existing RPC tests pass; new tests cover the new RPC method against
  both `EmbeddedFspecBackend` and `WebSocketFspecBackend` (RPC-009 cross-transport parity).
- The keybindings (C / F / D / /) are NOT yet wired to actions in this card
  — they are visible hints only. Wiring lands in subsequent cards
  (Checkpoint viewer, Changed Files viewer, FOUNDATION.md viewer, new
  Agent session — all RPC-002 children).
