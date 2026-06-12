# AST Research — `export-work-units` (RPC-229)

TS source of truth: `src/commands/export-work-units.ts` (+ `export-work-units-help.ts`)
Rust stub: `codelet/fspec-core/src/commands/export_work_units.rs` (NotYetPorted, RPC-229)

## 1. TS surface inventory

### Exported function `exportWorkUnits(options)` — lines 21-48

```ts
exportWorkUnits(options: { format: string; output: string; cwd?: string })
  : Promise<{ success: boolean }>
```

Behaviour:
1. `cwd = options.cwd || process.cwd()`.
2. `workUnitsFile = join(cwd, 'spec', 'work-units.json')`.
3. `readFile(workUnitsFile, 'utf-8')` then `JSON.parse` → `WorkUnitsData`.
   - **No `ensureWorkUnitsFile`** — direct `readFile`. ENOENT / parse errors caught + rewrapped.
4. `workUnits = Object.values(data.workUnits)` — array, insertion order.
5. If `options.format === 'json'`:
   - `writeFile(options.output, JSON.stringify(workUnits, null, 2))` — **2-space indent**.
     Writes to the RAW output path (NOT joined with cwd — `options.output` verbatim).
6. else → `throw new Error(\`Unsupported format: ${options.format}\`)`.
   - **CSV is NOT implemented** despite the `--description`/help saying "json or csv".
7. Return `{ success: true }`.
8. catch → `throw new Error(\`Failed to export work units: ${error.message}\`)`.

### CLI registration `registerExportWorkUnitsCommand(program)` — lines 50-81

```ts
program
  .command('export-work-units')
  .description('Export work units to JSON or CSV')
  .argument('<format>', 'Output format: json or csv')
  .argument('<output>', 'Output file path')
  .option('--status <status>', 'Filter by status')
  .action(async (format, outputPath, options) => {
    const result = await exportWorkUnits({ format, output: outputPath, status: options.status });
    output.log(chalk.green(`✓ Exported ${result.count} work units to ${result.outputFile}`));
  });
// catch → output.error(chalk.red('✗ Failed to export work units:'), message); process.exit(1)
```

## 2. TS BUGS / quirks ("Framing A")

a) **`result.count` and `result.outputFile` are `undefined`** — the function
   returns only `{ success: true }`. So the success line is literally:
   `✓ Exported undefined work units to undefined`.
   - The `(result as Record<string, unknown>).count` cast does not invent values.

b) **`--status` flag accepted by Commander but the function IGNORES it.** No
   filtering occurs. The help-doc lists `--epic` too but the CLI registration
   only wires `--status`. (Function takes neither.)

c) **CSV unsupported** — `format=csv` → throw `Unsupported format: csv` →
   `Failed to export work units: Unsupported format: csv` → exit 1.

### Framing A decision (per command-port.md §10)
- **Dispatcher / function contract is canon**: `exportWorkUnits({format, output, cwd})`
  reads work-units, and for `format=='json'` writes `Object.values(workUnits)` as
  2-space pretty JSON to `output`, returns `{success:true}`. For anything else
  throws `Unsupported format: <fmt>`. NO status/epic filtering (function ignores them).
- **Rust core fn** implements THIS. Returns `{ "success": true }` JSON string on success.
- **CLI shell** mirrors broken TS: success log `✓ Exported undefined work units to undefined`
  (count/outputFile undefined). Marshals `format`, `output`, `status` to core; core ignores status.
- Help-doc (`export-work-units-help.ts`) is canon for `--help` output ONLY.

## 3. Help config (`export-work-units-help.ts`)

```
name: 'export-work-units'
description: 'Export work units to JSON or CSV format'
usage: 'fspec export-work-units <format> <output> [options]'
arguments:
  format (required): 'Output format: json or csv'
  output (required): 'Output file path'
options:
  --status <status> : Filter by status
  --epic <epic>     : Filter by epic
examples:
  fspec export-work-units json work-units.json
    desc: Export to JSON
    output: ✓ Exported 42 work units to work-units.json
relatedCommands: ['list-work-units', 'query-work-units']
```
(Note: help lists `--epic` AND `--status`; CLI only registers `--status`.
Help example output `42 ... work-units.json` differs from runtime `undefined ... undefined`.
Help-doc is canon for `--help` text rendering ONLY.)

## 4. Dispatcher contract (function canon) — Rust core fn

`pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`

Args (camelCase):
```rust
struct Args {
    format: Option<String>,   // "json" | other
    output: Option<String>,   // raw output file path
    status: Option<String>,   // accepted, IGNORED (parity)
    epic: Option<String>,     // accepted, IGNORED (help lists it)
}
```

Logic:
1. Read `spec/work-units.json` DIRECTLY (no auto-create). ENOENT → wrap
   `Failed to export work units: <io msg>`. Parse error → wrap similarly.
   (Reuse the direct-read + `format_io_error` pattern from query_work_units.rs.)
2. `units = data.work_units.values()` (insertion order, full WorkUnit objects).
3. If `format == Some("json")`: serialize `units` with **2-space pretty JSON**
   (`serde_json::to_string_pretty`), write to `output` path verbatim
   (`std::fs::write`). The serialized WorkUnit array must preserve per-unit
   on-disk field order (WorkUnit's manual Serialize already does this).
4. else → wrap `Failed to export work units: Unsupported format: <fmt>`.
5. Return JSON string `{ "success": true }`.

Edge: if `output` is None but format==json → TS would `writeFile(undefined, …)`
→ throws → wrapped. Decide exact message in PHASE B (likely just require output
non-empty; the CLI always supplies it as a required positional).

## 5. Shared infra reused (NO new shared files needed)
- `crate::types::work_unit::{WorkUnitsData, WorkUnit}` — full units round-trip via manual Serialize.
- Direct `std::fs::read_to_string` (no-auto-create) + `std::fs::write` for output file.
- No `write_json_atomic` (TS uses plain `writeFile` to an arbitrary external path).

## 6. Files to produce (6 artifacts)
1. `codelet/fspec-core/src/commands/export_work_units.rs` (rewrite stub)
2. `codelet/fspec/src/export_work_units.rs` (CLI bridge)
3. `codelet/fspec-core/src/help/configs/export_work_units.rs` (help config)
4. `codelet/fspec/tests/fixtures/help/export-work-units.txt` (help fixture)
5. `codelet/fspec-core/tests/export_work_units.rs` (dispatcher test)
6. `codelet/fspec/tests/cli_export_work_units.rs` (CLI test)

Shared-file changes (SUPERVISOR): canonical.rs PORTED_COMMANDS, dispatch.rs arm,
commands/mod.rs, help/configs/mod.rs, main.rs Mode (2 positional + --status) + intercept + bridge mod.

## 7. Estimate: 2 (simple) — ~80 LOC TS, read + serialize + write, reuses all infra, no filtering.
