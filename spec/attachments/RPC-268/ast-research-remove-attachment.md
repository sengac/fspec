# remove-attachment AST Research (RPC-268)

## TypeScript source of truth

- **File**: `src/commands/remove-attachment.ts` (117 LOC)
- **Help**: `src/commands/remove-attachment-help.ts` (90 LOC)
- **Helpers used**:
  - `src/utils/ensure-files.ts::ensureWorkUnitsFile(cwd)` — auto-creates `spec/work-units.json`
  - `src/utils/file-manager.ts::fileManager.transaction(path, mutator)` — single atomic write
  - `fs/promises::unlink` — file deletion (optional, controlled by `--keep-file`)
  - `output.log` — stdout writer

## Behavioural surface (in TS execution order)

1. `cwd = options.cwd || process.cwd()`.
2. `workUnitsPath = join(cwd, 'spec', 'work-units.json')`.
3. `data = await ensureWorkUnitsFile(cwd)`.
4. If `!data.workUnits[workUnitId]` → throw `"Work unit '<id>' does not exist"`.
5. If `!workUnit.attachments || workUnit.attachments.length === 0` → throw `"Work unit '<workUnitId>' has no attachments to remove"`.
6. Locate by suffix: `attachmentIndex = workUnit.attachments.findIndex(p => p.endsWith(fileName))`.
7. If `attachmentIndex === -1` → throw `"Attachment '<fileName>' not found for work unit '<workUnitId>'"`.
8. `attachmentPath = workUnit.attachments[attachmentIndex]`; `fullPath = join(cwd, attachmentPath)`.
9. `workUnit.attachments.splice(attachmentIndex, 1)` — remove from array.
10. If `!keepFile`:
    - `try { await unlink(fullPath); output.log('✓ Attachment removed from work unit and file deleted'); }`
    - `catch { output.log('⚠ Attachment removed from work unit (file was already missing)'); }`
    - The `catch` does NOT distinguish ENOENT from other I/O errors — any failure becomes the "already missing" warning.
11. Else: `output.log('✓ Attachment removed from work unit (file kept)')`.
12. `output.log('  File: <attachmentPath>')` — always (after the status line).
13. `workUnit.updatedAt = new Date().toISOString()`.
14. `data.meta.lastUpdated = new Date().toISOString()` (if meta exists).
15. `fileManager.transaction(workUnitsPath, mutator)` — atomic write.
16. **NOTE**: The array splice happens BEFORE the file delete attempt. Even if the unlink fails (file missing), the tracking entry is removed AND the disk write proceeds. So `--keep-file` is the ONLY way to keep the disk file when the tracking entry is removed. There is no `--keep-tracking`-style flag.

## CLI surface (Commander.js)

```
fspec remove-attachment <workUnitId> <fileName> [options]
  --keep-file  Keep the file on disk (only remove from work unit tracking)
```

- 2 positional args (both required), 1 optional `--keep-file` flag (boolean).
- On error: `output.error('Error:', errorMessage); process.exit(1)`.

## Rust port architecture notes

### File layout (core)
- `codelet/fspec-core/src/commands/remove_attachment.rs` — `pub async fn run(args_json, project_root) -> Result<String, FspecCoreError>`.
- Reuses `io::ensure::ensure_work_units_file`, `io::locked_file::write_json_atomic`, `io::time::iso8601_now`.
- Reads `attachments` from `WorkUnit.extra["attachments"]` (Value::Array of strings), mutates via `as_array_mut()`.
- Returns the multi-line success output as a single String so the CLI bridge can `print!` it verbatim.

### Suffix matching
TS `path.endsWith(fileName)` is byte-suffix matching, not basename equality. e.g., a fileName of `"diagram.png"` matches both `"spec/attachments/AUTH-001/diagram.png"` AND `"foo/bar/X-diagram.png"` (yes, BUG-prone but it's the canonical TS behaviour). Mirror exactly with Rust `str::ends_with`.

### File deletion behaviour
- When `keepFile=false`: attempt `std::fs::remove_file(fullPath)`; on success the canonical text is "✓ Attachment removed from work unit and file deleted"; on ANY error (ENOENT, perms, etc.) text is "⚠ Attachment removed from work unit (file was already missing)".
- When `keepFile=true`: skip deletion; text is "✓ Attachment removed from work unit (file kept)".
- Array splice and JSON write always succeed regardless of file deletion outcome.

### Error messages (verbatim parity)
- `Work unit '<id>' does not exist`
- `Work unit '<workUnitId>' has no attachments to remove`
- `Attachment '<fileName>' not found for work unit '<workUnitId>'`

### Output (verbatim parity)
- Success body is a string with TWO lines: the status line + `"  File: <attachmentPath>\n"`.
- The relative path stored on disk is what's echoed — NOT the basename, NOT the absolute path.

### CLI bridge (codelet/fspec/src/remove_attachment.rs)
- clap variant: `RemoveAttachment { work_unit_id: String, file_name: String, keep_file: bool }`.
- Marshal JSON: `{workUnitId, fileName, keepFile}` (boolean).
- Print core's rendered string verbatim on success; print `Error: <reason>` on stderr on failure (mirrors `output.error('Error:', message)` not the ✗ Failed pattern).
- Exit 0 on success, 1 on error.

### Help fixture
- Capture `node dist/index.js remove-attachment --help` after `npm run build`.
- Help config mirrors TS quirks (typicalWorkflow comma-join, relatedCommands "fspec " doubling, commonErrors "Fix: undefined").
- TS help has 3 relatedCommands entries (vs list-attachments' 3 and add-attachment's 6).
