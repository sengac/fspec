# RPC-313 — AST Research: `update-prefix`

**TS source:** `src/commands/update-prefix.ts` (93 LOC)
**Rust target:** `codelet/fspec-core/src/commands/update_prefix.rs` (currently `NotYetPorted` stub)

## Two-front-doors invariant

Both entry points eventually call `updatePrefix(...)`:

| Front door | TS site | Rust site |
|---|---|---|
| CLI (`fspec update-prefix AUTH -d "..."`) | `registerUpdatePrefixCommand` `src/commands/update-prefix.ts:75-92` | `codelet/fspec/src/update_prefix.rs` (NEW) → `update_prefix::run(args_json, project_root)` |
| LLM dispatcher | tools.ts → `dispatchCommand("update-prefix", args)` | `codelet/fspec-core/src/dispatch.rs::run_ported` → `update_prefix::run(args_json, project_root)` |

## Dispatcher arg surface (FULL)

The exported `updatePrefix` function (TS `src/commands/update-prefix.ts:24-29`) accepts:

```ts
{
  prefix: string;           // REQUIRED
  epicId?: string;          // optional — verify epic exists if provided
  description?: string;     // optional — replace existing description if provided
  cwd?: string;             // resolved to process.cwd() in TS
}
```

The function ALWAYS sets `updatedAt` to `new Date().toISOString()`, regardless of
whether any field actually changed. That side-effect is observable to subsequent
`list-prefixes` calls (TS does not surface `updatedAt`, but disk bytes change).

## CLI arg surface (SUBSET)

The Commander.js surface (TS `src/commands/update-prefix.ts:75-92`) is:

```
fspec update-prefix <prefix> [-d|--description <description>]
```

**Note: the CLI surface does NOT expose `--epic-id`.** Only `description` can be
updated from the shell. The dispatcher path (LLM tool call) CAN pass `epicId`.
This is a real surface asymmetry — RPC-213 (`create-prefix`) has the same shape
on both fronts.

## Behaviour — line-by-line trace

```
TS:30  cwd = options.cwd || process.cwd()
TS:34  data = ensurePrefixesFile(cwd)        // auto-creates spec/prefixes.json
TS:35  prefixesFile = join(cwd, 'spec', 'prefixes.json')
TS:38  if (!data.prefixes[options.prefix]) throw Error(`Prefix ${X} not found`)
TS:43  if (options.epicId) {
TS:44    epicsData = ensureEpicsFile(cwd)    // auto-creates spec/epics.json
TS:45    if (!epicsData.epics[options.epicId]) throw Error(`Epic ${Y} not found`)
TS:51  if (options.epicId !== undefined)    data.prefixes[X].epicId = options.epicId
TS:55  if (options.description !== undefined) data.prefixes[X].description = options.description
TS:59  data.prefixes[X].updatedAt = new Date().toISOString()    // ALWAYS
TS:62  fileManager.transaction(prefixesFile, fileData => Object.assign(fileData, data))
TS:66  return { success: true }
TS:67-71 catch arm: throw new Error(`Failed to update prefix: ${err.message}`)
```

CLI output on success (TS:87): `output.log('✓ Prefix ${prefix} updated successfully')`
CLI output on failure (TS:89): `output.error('✗ Failed to update prefix:', err.message)` + `process.exit(1)`

## On-disk Prefix shape

```ts
interface Prefix {
  prefix: string;
  description?: string;
  epicId?: string;          // <-- new vs RPC-213
  createdAt?: string;
  updatedAt?: string;       // <-- new vs RPC-213
}
```

The current Rust `Prefix` struct (`codelet/fspec-core/src/types/prefix.rs`) has
`prefix`, `description`, `created_at`, plus `#[serde(flatten)] extra` catch-all.
**It does NOT have native `epicId` or `updatedAt` fields.**

### Parity strategy (no shared-type change)

Mutate `epicId` and `updatedAt` through the `extra` map. Round-trip works because
existing `epicId`/`updatedAt` values on disk land in `extra` after deserialize,
and re-serialize emits them at the top level via `#[serde(flatten)]`. JSON byte
order: `prefix`, `description`, `createdAt`, then `extra` fields in insertion
order. This matches the TS object property order because:
- if the on-disk record already had `epicId`/`updatedAt`, they roundtrip in their
  existing position within `extra`
- if newly added, they append at the end of `extra` (matching JS property
  insertion order for newly-set keys)

This avoids touching the read-only shared `types/prefix.rs` file and keeps
RPC-313 fully within worker-owned files. If/when other commands need typed
access to `epicId`/`updatedAt`, a follow-up RPC can promote them out of `extra`.

## ensureEpicsFile parity gap

TS line 44 calls `ensureEpicsFile(cwd)` which AUTO-CREATES `spec/epics.json` when
missing. The Rust `io/ensure.rs` currently only exposes `read_epics_or_empty`
(RPC-243 read-only twin) — there is NO `ensure_epics_file` helper yet.

### Observable behaviour

| Scenario | TS behaviour | Rust behaviour |
|---|---|---|
| epicId passed, epics.json missing | auto-creates empty epics.json, then "Epic <X> not found" | returns empty epics, then "Epic <X> not found" |
| epicId passed, epic exists | success | success |
| epicId not passed | epics.json untouched | epics.json untouched |

**User-visible error message is identical.** The only difference is the
side-effect of auto-creating `spec/epics.json` on the "epic not found" failure
path. We accept this minor parity gap and document it. If supervisor wants
strict parity, they can add `ensure_epics_file` to `io/ensure.rs`.

## Error message parity

| TS line | TS message | Rust target |
|---|---|---|
| 39 | `Prefix ${prefix} not found` | wrapped → `Failed to update prefix: Prefix <X> not found` |
| 46 | `Epic ${epicId} not found` | wrapped → `Failed to update prefix: Epic <Y> not found` |
| 69 | `Failed to update prefix: ${err.message}` | top-level wrap on every error path |

Validation messages must satisfy substring assertions in tests.

## Atomic write

TS line 62: `fileManager.transaction(prefixesFile, async fileData => { Object.assign(fileData, data); })`.
Rust uses `io::locked_file::write_json_atomic` (write-temp + rename), which
matches the durability guarantee (RPC-213 established the pattern).

## Insertion order

`PrefixesData.prefixes` is an `IndexMap` in Rust (matches TS object literal
insertion order). Updating an existing key via `IndexMap::get_mut` (or
`insert` of existing key) preserves position. The update MUST be in-place;
we do NOT remove + re-insert (which would move the entry to the end).

## Result shape

TS returns `{ success: true }` — only a single boolean. The dispatcher JSON
output is therefore very small.

```rust
#[derive(Serialize)]
struct UpdatePrefixResult { success: bool }
```

The CLI bridge IGNORES the dispatcher payload and prints
`✓ Prefix <X> updated successfully` on success (single line, no payload).

## Files touched by this work unit (worker-owned)

- `codelet/fspec-core/src/commands/update_prefix.rs` (rewrite stub)
- `codelet/fspec-core/src/help/configs/update_prefix.rs` (NEW)
- `codelet/fspec/src/update_prefix.rs` (NEW CLI bridge)
- `codelet/fspec-core/tests/update_prefix.rs` (NEW dispatcher tests)
- `codelet/fspec/tests/cli_update_prefix.rs` (NEW CLI tests)
- `codelet/fspec/tests/fixtures/help/update-prefix.txt` (NEW help fixture)
- `spec/features/update-prefix-rust-port.feature` (NEW)
- `spec/features/update-prefix-cli-subcommand.feature` (NEW)

## Files NOT touched (supervisor wires)

- `canonical.rs`, `dispatch.rs`, `commands/mod.rs`, `help/configs/mod.rs`, `main.rs`

## TS reference excerpts

### update-prefix.ts:24-73 (`updatePrefix` core)
- Reads via `ensurePrefixesFile`
- Branches on `epicId` (verify), then conditionally sets fields
- Always sets `updatedAt`
- Atomic write via `fileManager.transaction`
- Wraps errors with `"Failed to update prefix: "` prefix

### update-prefix.ts:75-92 (`registerUpdatePrefixCommand`)
- positional `<prefix>` (required)
- option `-d, --description <description>` (optional)
- prints `✓ Prefix ${prefix} updated successfully` on success
- exits 1 on error
