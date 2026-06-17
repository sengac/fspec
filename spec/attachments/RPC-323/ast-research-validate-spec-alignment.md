# AST Research — validate-spec-alignment (RPC-323)

## TS source of truth
- `src/commands/validate-spec-alignment.ts` (104 LOC)
- `src/commands/validate-spec-alignment-help.ts`

## Signature (TS — the EXPORTED, TESTED function)
```ts
export async function validateSpecAlignment(options: {
  workUnitId: string;
  cwd?: string;
}): Promise<{ valid: boolean; warnings?: string[] }>
```

## CRITICAL: registration is BROKEN in TS
The `registerValidateSpecAlignmentCommand` action handler calls
`validateSpecAlignment({ fix: options.fix })` (no `workUnitId`!) and then reads
`result.aligned` / `result.issues` — neither exists on the returned shape. So the
CLI front-door is effectively dead/throws. The Rust port mirrors the **exported
function contract** (the real, tested behaviour), and the CLI bridge needs a
`workUnitId` arg to be useful. **ASK SUPERVISOR**: the help config advertises a
positional `[feature-files...]` + `--fix`, but the exported function takes a
`workUnitId`. Decision needed on the clap surface (see report).

## Exported-function algorithm (the real behaviour to port)
1. `workUnitsFile = join(cwd, 'spec', 'work-units.json')`.
2. `const content = await readFile(workUnitsFile, 'utf-8')` then `JSON.parse` —
   this is a DIRECT read (NOT ensureWorkUnitsFile). ENOENT/parse errors are caught
   below and re-thrown wrapped.
3. If `!data.workUnits[options.workUnitId]` → `throw new Error(`Work unit ${id} not found`)`.
4. Glob `spec/features/**/*.feature` (relative, non-absolute).
5. `workUnitTag = `@${workUnitId}``. For each feature file, read content, split into
   lines. For each line i: if `line.trim().includes(workUnitTag)` AND
   `i+1 < lines.length` AND `lines[i+1].trim().startsWith('Scenario:')` →
   `scenariosFound++`.
6. If `scenariosFound === 0` → `{ valid:false, warnings:[`No scenarios for ${workUnitId}`] }`.
7. Else → `{ valid:true }`.
8. ALL errors are caught and re-thrown as `Error(`Failed to validate spec alignment: ${msg}`)`.

## Rust port plan
- `commands/validate_spec_alignment.rs`: `Args { work_unit_id: Option<String>, fix: Option<bool> }`
  (camelCase `workUnitId`). If `work_unit_id` is None → InvalidArgs (required).
- Load work-units.json DIRECTLY (read_to_string + serde) — NOT ensure_*; ENOENT/parse
  wrap into the `Failed to validate spec alignment:` message (parity with TS catch).
  Use `serde_json::Value` raw read to avoid typed-loader required-field rejection
  (mirror validate_work_units approach) — only need `.get("workUnits").get(id)`.
- Glob features via `crate::io::feature_glob::glob_feature_files` — BUT that errors when
  spec/features missing; TS glob returns [] when dir missing. **Check semantics**: TS
  `glob(['spec/features/**/*.feature'])` returns empty array if dir absent (no throw).
  glob_feature_files returns DirectoryNotFound. Need a "soft" walk that returns [] when
  the directory is absent. **ASK SUPERVISOR** for `glob_feature_files_or_empty` OR handle
  the DirectoryNotFound by mapping to empty Vec locally in this command.
- Tag-scan logic: read each file, split lines, line.trim().contains(tag) && next.trim().starts_with("Scenario:").
- Result envelope `{ valid, warnings? }` returned as JSON string.
- CLI bridge renders: valid → `✓ ...`; invalid → list warnings to stderr + exit 1; not-found → error.

## Shared modules reused
- `crate::error::FspecCoreError`
- `crate::io::feature_glob` (with soft-empty handling — see ASK)
- direct std::fs read for work-units.json
