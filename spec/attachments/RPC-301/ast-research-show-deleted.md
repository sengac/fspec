# AST Research — `show-deleted` (RPC-301)

## TypeScript Sources Of Truth
- `src/commands/show-deleted.ts` (109 LOC)
- `src/commands/show-deleted-help.ts` (46 LOC)

## Type Shapes (from `src/types/index.ts:4-26, 154-162`)

```ts
interface ItemWithId {
  id: number;          // auto-incrementing, never reused
  text: string;
  deleted: boolean;    // soft-delete flag
  createdAt: string;   // ISO 8601
  deletedAt?: string;  // ISO 8601, only when deleted=true
}

type RuleItem = ItemWithId;
type ExampleItem = ItemWithId;
type ArchitectureNoteItem = ItemWithId;
interface QuestionItem extends ItemWithId {
  selected: boolean;
  answered?: boolean;
  answer?: string;
}

// On WorkUnit:
rules?: RuleItem[];
examples?: ExampleItem[];
questions?: QuestionItem[];
architectureNotes?: ArchitectureNoteItem[];
```

## Dispatch Signature (TS)

```ts
export async function showDeleted(options: {
  workUnitId: string;
  cwd?: string;
}): Promise<{
  success: boolean;
  deletedItems: Array<{ id: number; text: string; deletedAt?: string }>;
  totalDeleted: number;
}>;
```

## Observed Behaviour (line-by-line)

1. **L28–29**: Resolves `cwd` from option or `process.cwd()`. Joins
   `spec/work-units.json`.
2. **L32**: Calls `ensureWorkUnitsFile(cwd)` — this **AUTO-CREATES**
   `spec/work-units.json` if missing (load-or-init semantics), unlike
   `list-prefixes` which reads-or-empty.
3. **L35–37**: Validates the requested work unit exists in
   `data.workUnits[workUnitId]`. Throws `Error("Work unit '<id>' does not
   exist")` if not present.
4. **L39**: Pulls the `WorkUnit` object.
5. **L42–56**: Iterates 4 arrays in this exact order — `rules`,
   `examples`, `questions`, `architectureNotes`. Within each array,
   preserves array order. Filters by `r.deleted === true`. Maps to
   `{ id, text, deletedAt }` (drops `createdAt`, `selected`, `answered`,
   `answer`).
6. **L58–63**: Concatenates the four filtered lists into a single flat
   `deletedItems` array. **Insertion order: rules first, then examples,
   then questions, then architectureNotes.**
7. **L65–69**: Returns `{ success: true, deletedItems, totalDeleted:
   deletedItems.length }`.

## CLI Rendering (TS L72–108)

- Argument: positional `<workUnitId>` (required).
- No flags (no `.option(...)` calls).
- Empty case (`totalDeleted === 0`):
  - Single line `No deleted items found` to stdout via `output.log`.
- Populated case:
  - `\nDeleted items in <workUnitId> (<N> total):` (chalk.bold).
  - For each item: `  [<id>] <text>` followed by ` (deleted: <iso>)`
    (chalk.dim) when `deletedAt` is set; nothing when it's missing.
  - Final empty line for spacing.
- Error path: `output.error(chalk.red('✗ Failed to show deleted
  items:'), error.message)` → `process.exit(1)`.

## Two-Front-Doors Decisions

1. **Dispatcher (LLM-facing)**
   - Args JSON: `{"workUnitId": "<string>"}` (camelCase, required).
   - Optional `{"format": "json"|"text"}` (NEW Rust-only, mirroring
     `list-prefixes`) for the structured-output path.
   - Default format: `text` — same exact rendering as the CLI bridge.
   - Returns `DispatchResult{success, data, error}`.

2. **CLI (shell-facing)**
   - Clap subcommand `show-deleted <workUnitId>`. NO flags (parity with
     TS Commander.js).
   - Exit 0 on success, exit 1 on `FspecCoreError`.
   - Stderr: `Error: <msg>` (same chalk-equivalent contract as RPC-253
     rule [14]).

## File Read Semantics — IMPORTANT

The TS implementation uses `ensureWorkUnitsFile`, NOT `readWorkUnitsOrEmpty`.
That means `show-deleted` AUTO-CREATES `spec/work-units.json` when it is
missing. The Rust port must reuse the existing
`io::ensure::ensure_work_units_file` helper (already public — see
`io/ensure.rs:40`). This is a deliberate parity decision distinct from
`list-prefixes` (which uses the read-only twin).

After auto-create, the empty `workUnits` map will not contain the
requested ID → the command then throws "Work unit '<id>' does not
exist".

## WorkUnit Field Access

The Rust `WorkUnit` struct (`codelet/fspec-core/src/types/work_unit.rs`)
does NOT model `rules`, `examples`, `questions`, `architectureNotes` —
they round-trip through `extra: serde_json::Map<String, Value>`. The
`show-deleted` command will deserialize these arrays inline from
`wu.extra` using a typed `DeletedItemRaw` struct with `#[serde(default)]`
fields. This keeps the shared `WorkUnit` type minimal and parallel-port-
safe.

## Output Shape (JSON format)

```json
{
  "success": true,
  "workUnitId": "AUTH-001",
  "deletedItems": [
    { "id": 0, "text": "Old rule", "deletedAt": "2025-01-31T12:00:00.000Z" },
    { "id": 3, "text": "Obsolete example" }
  ],
  "totalDeleted": 2
}
```

Note: `workUnitId` added to the structured payload for dispatcher
traceability (the TS function signature already carries it through, but
the return type drops it — we add it here for symmetry with other
single-target queries).

## Help Config

Direct port of `show-deleted-help.ts` into a `pub const CONFIG:
CommandHelpConfig` mirroring `list_prefixes.rs` configuration.

## File Layout (matches porting playbook)

- `codelet/fspec-core/src/commands/show_deleted.rs` — dispatcher impl.
- `codelet/fspec-core/src/help/configs/show_deleted.rs` — help config.
- `codelet/fspec-core/tests/show_deleted.rs` — dispatcher tests.
- `codelet/fspec/src/show_deleted.rs` — CLI bridge.
- `codelet/fspec/tests/cli_show_deleted.rs` — CLI shell tests.
- `codelet/fspec/tests/fixtures/help/show-deleted.txt` — TS help fixture.

## Shared-File Wiring Required (Supervisor)

The supervisor MUST wire AFTER worker implementation:
1. `codelet/fspec-core/src/commands/mod.rs` — `pub mod show_deleted;`
   already exists (stub).
2. `codelet/fspec-core/src/help/configs/mod.rs` — add
   `pub mod show_deleted;`.
3. `codelet/fspec-core/src/dispatch.rs` — move `"show-deleted"` from the
   `run_stub` arm to the `run_ported` arm.
4. `codelet/fspec-core/src/canonical.rs` — mark `show-deleted` as ported
   (if there's an `is_ported` whitelist — confirm by reading file).
5. `codelet/fspec/src/main.rs` — add clap `Mode::ShowDeleted` variant
   and dispatch arm.

Worker will NOT touch these — they are listed in HARD RULES.
