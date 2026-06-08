# AST Research — `query-work-units` (RPC-263)

## Sources
- TS implementation: `src/commands/query-work-units.ts` (278 lines)
- TS help config: `src/commands/query-work-units-help.ts` (53 lines)
- Closest cousin: `src/commands/list-work-units.ts` (141 lines)
- Canonical Rust port: `codelet/fspec-core/src/commands/list_work_units.rs`
- Stub: `codelet/fspec-core/src/commands/query_work_units.rs`

## Key Observations vs `list-work-units`

`query-work-units` is the *richer*, *advanced* sibling of `list-work-units`. Differences:

| Feature                       | list-work-units                  | query-work-units                              |
|-------------------------------|----------------------------------|-----------------------------------------------|
| Filters                       | status, prefix, epic, type       | status, prefix, epic, type, **tag**, **hasQuestions**, **questionsFor**, **workUnitId+showCycleTime** |
| Sorting                       | none                             | **--sort + --order (asc/desc)**               |
| Output formats                | `text` / `json`                  | `text` / `json` / **`csv`** / `table`         |
| Output to file                | no                               | **`--output <file>`** (writes CSV)            |
| Auto-creates files            | yes (ensureWorkUnitsFile + ensurePrefixesFile) | **no** (plain readFile, throws on ENOENT)     |
| Error wrapping                | raw fspec_core error              | wraps error as `Failed to query work units: <msg>` |
| CLI exit on error             | exit code 1                       | exit code 1, prefixed with `✗ Query failed:`  |
| `--json` shorthand            | no                                | `options.json` short-circuits format to json  |
| JSON shape                    | `{ workUnits: [...] }`            | `{ workUnits: [...], format: "json", data: [{workUnitId, featureFilePath}, ...] }` |
| Table shape                   | n/a                               | `{ workUnits, format: "table", columns, rows }` |
| Cycle time mode               | n/a                               | If `workUnitId + showCycleTime`, returns `{ stateTimings, totalCycleTime }` (hours) |
| Tag filter                    | n/a                               | `wu.tags.includes(tag)`                       |
| hasQuestions                  | n/a                               | true/false on `(wu.questions?.length || 0)`    |
| questionsFor                  | n/a                               | normalises bare → `@bare`; matches `q.text.includes(mention)` (or string q) |

## CLI Surface (Commander.js — `registerQueryWorkUnitsCommand`)

```
fspec query-work-units [options]
  --status <status>            Filter by status
  --prefix <prefix>            Filter by prefix
  --epic <epic>                Filter by epic
  --type <type>                story | task | bug
  --tag <tag>                  Filter by tag (e.g., @cli)
  --format <format>            text | json | csv (default 'text')
```

**Note:** the CLI registration is *narrower* than the JS function. `--sort`, `--order`,
`--output`, `--hasQuestions`, `--questionsFor`, `--show-cycle-time`, `--work-unit-id`,
`--json` are *function-level only* (consumed by other commands / programmatic callers).
For the **clap CLI bridge** we mirror exactly what Commander exposes.

For the **dispatcher** entry point we accept the full superset of options so the
agent-loop tool-call protocol retains parity with the TS function signature.

## CLI Output Behavior

- TS CLI registration only prints output when `--format=json` (`output.log(JSON.stringify(result, null, 2))`).
- For any other format (text/csv/table) the registered Commander action **prints nothing** —
  the TS function returns the structure but the CLI doesn't render it. (This is a quirk
  of the TS code, not a feature.) We mirror this behaviour for byte-for-byte parity.

## Help Output

Per `query-work-units-help.ts`:
- Description: "Query work units with advanced filters and output formats"
- Usage: `fspec query-work-units [options]`
- Options listed: --status, --type, --tag, --epic, --prefix, --format
- Examples: two examples filtering by status/format and by tag combo
- Related: list-work-units, export-work-units, search-scenarios, compare-implementations

## File I/O Contract

- Reads `spec/work-units.json` via `fs/promises.readFile`.
- Does **NOT** auto-create the file. ENOENT → wrapped error `Failed to query work units: ENOENT...`.
- Does **NOT** read `spec/prefixes.json`.
- For CSV output: writes to `options.output` via `writeFile`.

## Filter Chain (Order Matters)

1. `status` (exact)
2. `epic` (exact)
3. `prefix` (`wu.id.startsWith(prefix + '-')`)
4. `type` (defaulting missing → 'story')
5. `tag` (`(wu.tags ?? []).includes(tag)`)
6. `hasQuestions` (true → questions.length > 0; false → length === 0)
7. `questionsFor` (text includes `@mention`)

## Sorting

- `sort` key looked up on the WorkUnit object.
- String compare via `localeCompare`, number via subtraction.
- `order='desc'` negates the comparison.
- Undefined values are treated as equal (skip sort).

## CSV Format

```
id,title,status,createdAt,updatedAt
AUTH-001,Login,backlog,...,...
```

- Commas in `title` are **stripped** (`replace(/,/g, '')`).
- Missing fields → empty string.
- Written via `fs.writeFile`.

## Cycle Time Mode

When `workUnitId && showCycleTime`:
- Look up WU by id; throw `Work unit '<id>' does not exist` if missing.
- For each adjacent pair in `stateHistory`, compute hours between timestamps via `Math.round(durationMs / 3600000)`.
- Build `stateTimings: { state: "N hour(s)" }` (singular when N === 1).
- Return `{ stateTimings, totalCycleTime: "M hour(s)" }`.

## Error Wrapping

```js
throw new Error(`Failed to query work units: ${error.message}`);
```

## Rust Port Strategy

- **Dispatcher entry point**: `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`
- **Args struct**: full superset of CLI + function-level options (camelCase serde).
- **Filter chain**: mirror TS order; reuse `WorkUnit` / `WorkUnitsData` from `crate::types::work_unit`.
- **No file auto-creation**: use `read_work_units_or_empty` would auto-tolerate ENOENT silently — but TS *throws* on ENOENT. We must surface ENOENT as an error. Strategy: call `read_work_units_strict` (need to confirm helper exists) OR inline the readFile + serde-error mapping with the `Failed to query work units:` prefix.
- **CSV output**: write to `options.output` via std::fs::write.
- **Cycle time mode**: requires reading `stateHistory: Vec<{ state, timestamp, reason? }>` from WorkUnit. Verify the type exposes this.
- **Tag filter**: requires `tags: Vec<String>` on WorkUnit.
- **hasQuestions / questionsFor**: requires `questions: Vec<QuestionItem>` on WorkUnit; QuestionItem has `text` field, with string-fallback handling.
- **CLI bridge**: clap subcommand mirroring the *Commander.js-exposed* flag set only (status, prefix, epic, type, tag, format). Suppresses text output to match TS quirk.

## Open Questions / Shared-File Requests

1. **WorkUnit type richness** — does `crate::types::work_unit::WorkUnit` currently expose `tags`, `questions`, `stateHistory`, `featureFile`, `createdAt`, `updatedAt`? If not, supervisor must extend the shared type. **Likely needs supervisor to widen WorkUnit.**
2. **read_work_units_strict** — does `crate::io::ensure` provide a non-creating reader that escalates ENOENT? If not, we can inline `std::fs::read_to_string` + `serde_json::from_str` and map errors with the TS-style prefix. (Inline is fine for this port — keeps the worker self-contained.)
3. **Dispatcher routing** — supervisor must wire `query-work-units` route in `canonical.rs`, `dispatch.rs`, `commands/mod.rs`, `help/configs/mod.rs`. Worker will NOT touch those.

## Risk / Scope

- Cycle-time and questionsFor branches are non-trivial — may require new WorkUnit fields.
- We will scope Phase A scenarios to the **CLI-exposed surface** (status / prefix / epic / type / tag / format) plus dispatcher-level coverage of CSV + JSON + sort + hasQuestions to cover the full TS function.
- Estimate: **8 points** (very complex — widest filter set + CSV + cycle-time + multi-format output).
