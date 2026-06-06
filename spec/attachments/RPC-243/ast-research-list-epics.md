# AST Research — list-epics — RPC-243

## TypeScript source-of-truth

File: `src/commands/list-epics.ts` (146 lines)

### Interfaces (lines 7-37)

```ts
interface Epic {
  id: string;
  title?: string;          // ← OPTIONAL (line 9)
  description?: string;    // ← OPTIONAL (line 10)
  [key: string]: unknown;  // ← Forward-compat catch-all (line 11)
}

interface EpicsData {
  epics: Record<string, Epic>;
}

interface WorkUnit {
  id: string;
  status?: string;         // ← OPTIONAL
  epic?: string;           // ← OPTIONAL — used to associate WUs with epics
  [key: string]: unknown;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

interface EpicWithProgress {
  id: string;
  title?: string;          // ← OPTIONAL in output too
  description?: string;
  totalWorkUnits: number;
  completedWorkUnits: number;
  completionPercentage: number;
}
```

### Core function: `listEpics({ cwd? })` (lines 39-102)

```ts
const cwd = options.cwd || process.cwd();
const epicsFile = join(cwd, 'spec', 'epics.json');
const workUnitsFile = join(cwd, 'spec', 'work-units.json');

// 1. READ epics.json — ENOENT → return { epics: [] } (lines 47-57)
let epicsData: EpicsData;
try {
  const epicsContent = await readFile(epicsFile, 'utf-8');
  epicsData = JSON.parse(epicsContent);
} catch (error: unknown) {
  if (error instanceof Error && 'code' in error && error.code === 'ENOENT') {
    return { epics: [] };
  }
  throw error;   // ← ALL OTHER ERRORS (parse failures, permission errors) escalate
}

// 2. READ work-units.json — bare `catch {}` swallows ANY failure (lines 60-66)
let workUnitsData: WorkUnitsData | undefined;
try {
  const workUnitsContent = await readFile(workUnitsFile, 'utf-8');
  workUnitsData = JSON.parse(workUnitsContent);
} catch {
  // No work units file yet
}

// 3. Aggregate progress per epic (lines 69-99)
const epics: EpicWithProgress[] = [];
for (const epic of Object.values(epicsData.epics)) {   // ← insertion order
  let totalWorkUnits = 0;
  let completedWorkUnits = 0;

  if (workUnitsData) {
    for (const workUnit of Object.values(workUnitsData.workUnits)) {
      if (workUnit.epic === epic.id) {                 // ← EXACT epic-id match (NOT prefix-based)
        totalWorkUnits++;
        if (workUnit.status === 'done') {
          completedWorkUnits++;
        }
      }
    }
  }

  const completionPercentage =
    totalWorkUnits > 0
      ? Math.round((completedWorkUnits / totalWorkUnits) * 100)
      : 0;

  epics.push({
    id: epic.id,
    title: epic.title,
    description: epic.description,
    totalWorkUnits,
    completedWorkUnits,
    completionPercentage,
  });
}

return { epics };
```

### CLI wrapper: `listEpicsCommand()` (lines 104-139)

```ts
const result = await listEpics({});

if (result.epics.length === 0) {
  output.log('No epics found');
  process.exit(0);
}

output.log(`\nEpics (${result.epics.length})`);
output.log('');
for (const epic of result.epics) {
  output.log(epic.id);                          // ← always
  output.log(`  ${epic.title}`);                // ← always (even if undefined → "  undefined")
  if (epic.description) {                       // ← only when truthy
    output.log(`  ${epic.description}`);
  }
  if (epic.totalWorkUnits > 0) {                // ← only when total > 0
    output.log(
      `  Work Units: ${epic.completedWorkUnits}/${epic.totalWorkUnits} (${epic.completionPercentage}%)`
    );
  }
  output.log('');                               // ← trailing blank line per entry
}
process.exit(0);

// On error: chalk-red 'Error: <message>' to stderr, exit 1
```

### Commander.js registration (lines 141-146)

```ts
program
  .command('list-epics')
  .description('List all epics')
  .action(listEpicsCommand);
```

**ZERO `.option(...)` calls** — flag-less surface, same as `list-prefixes`.

## Key behavioural differences from list-prefixes

| Aspect | list-prefixes | list-epics |
|---|---|---|
| Source file | `spec/prefixes.json` | `spec/epics.json` |
| Work-unit association | `id.startsWith(prefix + '-')` | `workUnit.epic === epic.id` (exact match) |
| Filename for parse error | `prefixes.json` | `epics.json` |
| Empty sentinel | `No prefixes found` | `No epics found` |
| Header line | `Prefixes (N)` | `Epics (N)` |
| Per-entry layout | prefix → description → maybe Work Units | id → title → maybe description → maybe Work Units |
| `title` / `description` optionality | both required strings | both optional |
| Aggregation key | `prefix.prefix + '-'` prefix match | `workUnit.epic` exact-equality |

## On-disk shape — live data sample

```json
{
  "epics": {
    "coverage-tracking": {
      "id": "coverage-tracking",
      "title": "Coverage Tracking",
      "createdAt": "2025-10-13T09:49:22.109Z",
      "description": "...",
      "workUnits": ["COV-001", ...]
    },
    ...
  }
}
```

Note: the on-disk shape also carries `workUnits: string[]` (a denormalised list of WU IDs). The TypeScript implementation **IGNORES** this field — it always recomputes counts from `work-units.json`. We preserve `workUnits` (and any other field) via a `serde(flatten) extra` map for round-tripping.

## Shared infrastructure plan

NEW shared helpers in `codelet/fspec-core/src/io/ensure.rs`:

1. `read_epics_or_empty(cwd: &Path) -> Result<EpicsData, FspecCoreError>` — read-only twin of (forthcoming) `ensure_epics_file`. ENOENT → `Ok(EpicsData::initial())`. Parse error → escalate as `FspecCoreError::ParseJson { file: "epics.json", ... }`.
2. (Reuse) `read_work_units_or_empty(cwd)` from RPC-248 — bare-catch semantics already match the `list-epics` work-units read path.

NEW type module `codelet/fspec-core/src/types/epic.rs`:

```rust
pub struct Epic {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}
```

NEW container on `types/work_unit.rs` (extending the work_unit module rather than splitting):

```rust
pub struct EpicsData {
    #[serde(default)]
    pub epics: IndexMap<String, crate::types::epic::Epic>,
}

impl EpicsData {
    pub fn initial() -> Self { Self { epics: IndexMap::new() } }
}
```

(Putting `EpicsData` next to `PrefixesData` and `WorkUnitsData` matches the existing structure where the container types co-locate in `work_unit.rs` while the leaf `Epic` / `Prefix` records live in their own files.)

## CLI bridge expectations

- `codelet/fspec/src/list_epics.rs` — bridge module, empty `CliArgs` (no flags), delegates to `fspec_core::commands::list_epics::run`.
- `codelet/fspec/src/main.rs` — add `Mode::ListEpics` variant + match arm. Update long-about to mention `list-epics`.
- `codelet/fspec-core/src/canonical.rs` — add `"list-epics"` to `PORTED_COMMANDS` array.
- `codelet/fspec-core/src/dispatch.rs` — add `"list-epics" => commands::list_epics::run(...)` arm to `run_ported`, remove from stub match.
- `codelet/fspec/tests/cargo_shape.rs` — add `"list_epics.rs"` to the locked `src/` file list (grows from 8 → 9).

## Error message canonical substring

`Failed to parse epics.json` — symmetric with the existing `Failed to parse prefixes.json` and `Failed to parse work-units.json` strings used by `FspecCoreError::ParseJson`.

## Test data fixtures

Live `spec/epics.json` uses kebab-case epic IDs like `"coverage-tracking"`, `"test-coverage"`. Test fixtures should mirror that shape: `epic.id = "auth"`, `"dashboard"`, etc.
