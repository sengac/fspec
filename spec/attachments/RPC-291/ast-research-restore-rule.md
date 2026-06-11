# AST Research — `restore-rule` (RPC-291)

## TS sources

- `src/commands/restore-rule.ts` (143 LOC).
- `src/commands/restore-rule-help.ts` (59 LOC).

## Public dispatcher signature

```ts
interface RestoreRuleOptions {
  workUnitId: string;
  index?: number;       // single restore
  ids?: string;         // comma-separated bulk restore
  cwd?: string;
}
interface RestoreRuleResult {
  success: boolean;
  restoredRule: string;
  activeCount: number;
  restoredCount?: number;  // bulk only
  message?: string;        // idempotent single only
}
export async function restoreRule(options): Promise<RestoreRuleResult>
```

UNLIKE `restore-example`, `restore-rule` supports bulk restore via `ids`. The
TS code path for `ids` is at lines 53-82 and is exercised when callers pass
`{workUnitId, ids: "2,5,7"}` through the function-level API. The CLI surface
at lines 122-142 ONLY registers positional `<workUnitId> <index>` though —
`--ids` is NOT registered as a Commander.js option. Running
`node dist/index.js restore-rule FAKE-001 --ids 2,5` exits 1 with
`error: unknown option '--ids'`.

This means:
- **Dispatcher** must support both `index` and `ids`.
- **CLI** must support ONLY positional `<workUnitId> <index>`.
- The help fixture advertises `--ids` (captured from `node dist/index.js
  restore-rule --help`) but the CLI rejects it at parse time.

## Behaviour (line-by-line)

Prelude (lines 27-50):
1. `cwd = options.cwd ?? process.cwd()`.
2. `workUnitsFile = join(cwd, 'spec/work-units.json')`.
3. `data = await ensureWorkUnitsFile(cwd)` — auto-create + load.
4. Work-unit-exists gate: `Work unit '<id>' does not exist`.
5. Status gate (specifying): `Can only restore rules during discovery/specification phase. <id> is in '<state>' state.`
6. Rules-non-empty gate: `Work unit <id> has no rules`.

### Bulk path (`options.ids` truthy, lines 53-82)

1. `indices = ids.split(',').map(s => parseInt(s.trim(), 10))`.
2. For each index in order:
   - `rule = rules.find(r => r.id === index)`.
   - If not found → `Rule with ID <index> not found` (THROWS — bulk is
     **NOT atomic**, partial restores can land on disk before the throw…
     EXCEPT no disk write happens inside the loop, so practically it IS
     atomic w.r.t. disk: either the whole loop completes and the single
     write at line 73 fires, or a `throw` short-circuits before that
     write).
   - If `!rule.deleted` → `continue` (silently skip).
   - Else: `rule.deleted = false; delete rule.deletedAt; restoredRules.push(rule.text)`.
3. `workUnit.updatedAt = iso8601_now()`.
4. `fileManager.transaction(...)` — single atomic write.
5. Returns `{ success: true,
              restoredRule: restoredRules.join(', '),
              activeCount: count(r => !r.deleted),
              restoredCount: restoredRules.length }`.

   Note: when ALL ids are already active, `restoredRules` is `[]`,
   `restoredRule` becomes the empty string, `restoredCount: 0`, and the
   single atomic write still fires (updates `workUnit.updatedAt`).

### Single path (lines 84-119)

1. `rule = rules.find(r => r.id === options.index)`.
2. If not found → `Rule with ID <index> not found`.
3. **Idempotent path** — if `!rule.deleted`:
   ```
   return { success: true,
            restoredRule: rule.text,
            activeCount: count(r => !r.deleted),
            message: `Item ID ${index} already active` };
   ```
   Disk NOT mutated.
4. Otherwise: `rule.deleted = false; delete rule.deletedAt;`
5. `workUnit.updatedAt = iso8601_now()`.
6. `fileManager.transaction(...)`.
7. Returns `{ success: true, restoredRule, activeCount }`.

## CLI surface (`registerRestoreRuleCommand`)

```ts
program
  .command('restore-rule')
  .description('Restore a soft-deleted business rule by ID')
  .argument('<workUnitId>', 'Work unit ID')
  .argument('<index>', 'Rule ID (0-based)')
  .action(async (workUnitId, index) => {
    try {
      const result = await restoreRule({ workUnitId, index: parseInt(index, 10) });
      output.log(`✓ Restored rule: "${result.restoredRule}"`);
      if (result.message) output.log(`  ${result.message}`);
    } catch (error: any) {
      output.error('✗ Failed to restore rule:', error.message);
      process.exit(1);
    }
  });
```

- Always prints `✓ Restored rule: "<text>"` (NOTE: NO chalk.green wrapper
  here — `restore-example.ts` wraps with `chalk.green(...)`, but
  `restore-rule.ts` does NOT. Both reduce to the same bytes on non-TTY pipe
  capture, but it's a code-shape observation worth noting).
- Prints `  <message>` on a second line ONLY in the idempotent path.
- On error: stderr `✗ Failed to restore rule: <message>` + exit 1.

## Behavioural divergence note (Framing A)

The dispatcher front-door (via `RestoreRuleOptions`) supports `ids`, but
the CLI shell registration does not pass through `--ids`. The Rust port
mirrors this exactly:
- `commands::restore_rule::run` accepts BOTH `{workUnitId, index}` and
  `{workUnitId, ids: "..."}` JSON shapes — selecting bulk path when `ids`
  is present.
- `codelet/fspec/src/restore_rule.rs` bridge accepts ONLY
  `<workUnitId> <index>` positionals and forwards as JSON with `index`.

## TS `parseInt(index, 10)` parity

Same as `restore-example` and `remove-example`: non-numeric input maps to
the JSON string `"NaN"` which never matches an integer rule id, so the
canonical `Rule with ID NaN not found` error fires (unless the rules array
is empty, in which case the `has no rules` guard fires first).

For bulk path: `parseInt('2,abc,5'.split(',')[1].trim(), 10) → NaN`, so
the per-id `Rule with ID NaN not found` error fires from inside the loop
before any disk write. We must mirror this. The Rust bulk path will parse
each comma-separated token with `parse_ts_int_radix10`, treat `"NaN"` as
"won't match any id" → emit `Rule with ID NaN not found`.

## Shared infrastructure reuse

- `io::ensure::ensure_work_units_file`.
- `io::locked_file::write_json_atomic`.
- `io::time::iso8601_now`.
- `WorkUnit.extra["rules"]` and `extra["nextRuleId"]` (already used by
  `add_rule.rs`).

## CLI bridge plan

Same shape as `codelet/fspec/src/remove_example.rs`:
- `CliArgs { work_unit_id: String, index: String }`.
- Marshal to `{ "workUnitId": String, "index": Value }` with
  `parse_ts_int_radix10` semantics.
- The CLI does NOT forward `--ids` (not registered).
- On `Ok(rendered)`: `print!("{rendered}")`.
- On `Err`: `eprintln!("✗ Failed to restore rule: {}", render_core_error(&err))` + return `Ok(1)`.

## Dispatcher-only bulk surface

For `commands::restore_rule::run` the Rust impl accepts a 3rd JSON shape
selector:
```json
{"workUnitId":"AUTH-001","ids":"2,5,7"}
```
When `ids` is present we run the bulk path. When only `index` is present
we run the single path. When BOTH are present, the TS code checks `if
(options.ids)` first (line 53), so `ids` wins — Rust must mirror.

## Output rendering decisions

Single path (success):
```
✓ Restored rule: "<text>"
```

Single path (idempotent):
```
✓ Restored rule: "<text>"
  Item ID <n> already active
```

Bulk path (success / mixed / all-already-active):
```
✓ Restored rule: "<joined-texts>"
```
(Note: TS prints only `restoredRule` which is `restoredRules.join(', ')`
— when bulk restores 3 deleted rules whose texts are `"A"`, `"B"`, `"C"`,
output is `✓ Restored rule: "A, B, C"`. When all were already-active and
nothing was restored, `restoredRule` is `""` → `✓ Restored rule: ""`.
This is what TS does; we mirror it.)

All rendered strings end with a trailing newline. The CLI `print!`s as-is.
