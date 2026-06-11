# AST Research — `restore-example` (RPC-289)

## TS sources

- `src/commands/restore-example.ts` (111 LOC).
- `src/commands/restore-example-help.ts` (60 LOC).

## Public dispatcher signature

```ts
interface RestoreExampleOptions {
  workUnitId: string;
  index: number;
  cwd?: string;
}
interface RestoreExampleResult {
  success: boolean;
  restoredExample: string;
  activeCount: number;
  message?: string;
}
export async function restoreExample(options): Promise<RestoreExampleResult>
```

The dispatcher accepts ONLY `workUnitId` + `index`. **There is NO `ids` field**
in `RestoreExampleOptions` — unlike `restore-rule.ts` which DOES support bulk
restore via `ids`. The TS help (`restore-example-help.ts`) advertises an
`--ids <ids>` option, but the TS Commander.js registration at lines 88-110
does NOT register it. Running `node dist/index.js restore-example FAKE-001 --ids 2,5`
exits 1 with `error: unknown option '--ids'`.

This means the Rust port is single-restore only on both fronts. The help
fixture is captured byte-for-byte from `node dist/index.js restore-example
--help` so the `--ids` option text appears in the fixture (it is documentation,
not a working code path).

## Behaviour (line-by-line)

1. `cwd = options.cwd ?? process.cwd()`.
2. `workUnitsFile = join(cwd, 'spec/work-units.json')`.
3. `data = await ensureWorkUnitsFile(cwd)` — auto-creates the canonical empty
   structure on first run.
4. If `!data.workUnits[id]` → `throw new Error("Work unit '<id>' does not exist")`.
5. `workUnit = data.workUnits[id]`.
6. If `workUnit.status !== 'specifying'` →
   `Can only restore examples during discovery/specification phase. <id> is in '<state>' state.`
7. If `!workUnit.examples || examples.length === 0` →
   `Work unit <id> has no examples`.
8. `example = workUnit.examples.find(e => e.id === options.index)`.
9. If no example → `Example with ID <index> not found`.
10. **Idempotent path** — if `!example.deleted`:
    ```
    return { success: true,
             restoredExample: example.text,
             activeCount: count(e => !e.deleted),
             message: `Item ID ${index} already active` };
    ```
    Disk is NOT mutated.
11. Otherwise: `example.deleted = false; delete example.deletedAt;`
12. `restoredExample = example.text`.
13. `workUnit.updatedAt = new Date().toISOString()`.
14. `fileManager.transaction(workUnitsFile, async fileData => { Object.assign(fileData, data); })`.
15. Returns `{ success: true, restoredExample, activeCount }` (NO `message`).

## CLI surface (`registerRestoreExampleCommand`)

```ts
program
  .command('restore-example')
  .description('Restore a soft-deleted example by ID')
  .argument('<workUnitId>', 'Work unit ID')
  .argument('<index>', 'Example ID (0-based)')
  .action(async (workUnitId, index) => {
    try {
      const result = await restoreExample({ workUnitId, index: parseInt(index, 10) });
      output.log(chalk.green(`✓ Restored example: "${result.restoredExample}"`));
      if (result.message) output.log(`  ${result.message}`);
    } catch (error: any) {
      output.error('✗ Failed to restore example:', error.message);
      process.exit(1);
    }
  });
```

- Always prints `✓ Restored example: "<text>"`.
- Prints `  <message>` on a second line ONLY in the idempotent path.
- On error: stderr line `✗ Failed to restore example: <message>` + exit 1.

## TS `parseInt(index, 10)` parity

`parseInt('abc', 10) → NaN`; `find(e => e.id === NaN) → undefined`; thus
`Example with ID NaN not found` is the canonical error for non-numeric input
UNLESS the examples array is empty (then the `has no examples` guard fires
first). This matches the pattern already implemented in `remove-example`.
See `codelet/fspec/src/remove_example.rs::parse_ts_int_radix10` for the
canonical Rust mirror.

## Shared infrastructure reuse

- `io::ensure::ensure_work_units_file` — auto-create + load.
- `io::locked_file::write_json_atomic` — atomic write.
- `io::time::iso8601_now` — timestamp parity with `new Date().toISOString()`.
- `WorkUnit.extra["examples"]` — examples array lives in the `extra` map
  via `#[serde(flatten)]` (same as `remove-example`).

## CLI bridge plan

Same shape as `codelet/fspec/src/remove_example.rs`:
- `CliArgs { work_unit_id: String, index: String }` (raw string for TS
  `parseInt` parity).
- Marshal to `{ "workUnitId": String, "index": Value }` where Value is a
  JSON number or `"NaN"` string via the same `parse_ts_int_radix10` helper.
- On `Ok(rendered)`: `print!("{rendered}")` (the core already emits the
  full multi-line success block including the optional `Item ID N already
  active` second line).
- On `Err`: `eprintln!("✗ Failed to restore example: {}", render_core_error(&err))`
  + return `Ok(1)`.

## Output rendering decision

The core `run()` returns a String. We render:
```
✓ Restored example: "<text>"
```
plus, for the idempotent path:
```
✓ Restored example: "<text>"
  Item ID <n> already active
```
Both end with a trailing newline. The CLI bridge `print!`s the string as-is.

## Two-front-doors parity

Both fronts call `commands::restore_example::run(args_json, project_root)`.
The dispatcher passes JSON like `{"workUnitId":"AUTH-001","index":2}`. The
CLI bridge marshals positional args into the same shape (with `parseInt`
parity for `index`).
