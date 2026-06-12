# AST Research — RPC-214 `create-story` Rust port

## TypeScript source (to port) — src/commands/create-story.ts
- `createStory(options)` (line 32):
  1. `checkFoundationExists(cwd, cmd)` — throws foundation-missing userMessage
     + `<system-reminder>` when spec/foundation.json absent.
  2. Empty-title guard -> "Title is required".
  3. `ensurePrefixesFile(cwd)`; unregistered prefix ->
     "Prefix '<p>' is not registered. Run 'fspec create-prefix <p> ...' first."
  4. `ensureWorkUnitsFile(cwd)`.
  5. `--parent` must exist -> "Parent story '<p>' does not exist"; nesting depth
     `calculateNestingDepth` < MAX_NESTING_DEPTH=3 -> "Maximum nesting depth (3) exceeded".
  6. `--epic` via `ensureEpicsFile` must exist -> "Epic '<e>' does not exist".
  7. `generateNextId` (line 176): high-water-mark = max(prefixCounters[prefix],
     max existing id suffix) + 1; id = `<PREFIX>-<NNN>` zero-padded width 3.
  8. New story literal order: id, title, type:'story', status:'backlog',
     createdAt, updatedAt, [description], [epic], [parent], and children:[] ONLY
     when no parent.
  9. Push id to states.backlog; append to parent.children; set
     prefixCounters[prefix]=nextNumber.
  10. When --epic: append id to epic.workUnits in spec/epics.json.
- `calculateNestingDepth` (line 206): recursive walk up `parent` chain, depth
  starts at 1.

## Rust reference shape (copy this) — codelet/fspec-core/src/commands/create_epic.rs
- `pub async fn run(args_json, project_root)` — parse args (serde camelCase),
  validate, read-or-init store, merge preserving insertion order via
  serde_json::Map, `write_json_atomic`, return rendered success block.
- `render_success` builds "✓ Created <thing>\n  Title: ...\n  [Description: ...]".
- `read_raw_<store>_object` re-reads raw JSON to preserve existing key order.

## Shared helpers needed (SUPERVISOR owns io/ensure.rs)
- `ensure_work_units_file` (exists), `ensure_prefixes_file` (exists),
  `ensure_epics_file` (NEEDS ADDING — write-capable load+init),
  `check_foundation_exists` (NEEDS ADDING — emit verbatim foundation-missing
  message + <system-reminder>).

## Key parity decisions
- prefixCounters lives in WorkUnitsData.extra (flatten).
- children:[] omitted for child stories (has parent).
- zero-padded width-3 id via format!("{}-{:03}", prefix, n).
- two writes: spec/work-units.json then spec/epics.json (when --epic).
