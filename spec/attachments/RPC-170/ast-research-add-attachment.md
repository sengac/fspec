# add-attachment AST Research (RPC-170)

## TypeScript source of truth

- **File**: `src/commands/add-attachment.ts` (153 LOC)
- **Help**: `src/commands/add-attachment-help.ts` (106 LOC)
- **Helpers used**:
  - `src/utils/ensure-files.ts::ensureWorkUnitsFile(cwd)` — auto-creates `spec/work-units.json` with canonical empty structure
  - `src/utils/file-manager.ts::fileManager.transaction(path, mutator)` — single atomic write
  - `src/utils/normalize-path.ts::resolveFilePath(p)` — Unicode whitespace handling (BUG-130); two-phase: try exact → normalized → directory-scan for fuzzy whitespace match
  - `src/utils/attachment-mermaid-validation.ts::shouldValidateMermaid(p)` + `validateMermaidAttachment(p)` — gate by extension `.mmd`/`.mermaid`/`.md`
  - `src/utils/output.ts::output.log` — stdout writer (colourless when piped)

## Behavioural surface (in TS execution order)

1. `cwd = options.cwd || process.cwd()`.
2. `workUnitsPath = join(cwd, 'spec', 'work-units.json')`.
3. `data = await ensureWorkUnitsFile(cwd)` — auto-creates file on first run.
4. If `!data.workUnits[workUnitId]` → throw `"Work unit '<id>' does not exist"`.
5. `resolvedPath = await resolveFilePath(filePath)` — Unicode whitespace resolution.
6. `await access(resolvedPath)` — if fails, throw `"Source file '<filePath>' does not exist"` (uses the ORIGINAL user-supplied path in the error, NOT the resolved one).
7. If `shouldValidateMermaid(resolvedPath)`:
   - `validateMermaidAttachment(resolvedPath)` returns `{valid, error?}`.
   - On `!valid` → throw `"Failed to attach <basename>: <error>"`.
   - Any thrown error from the validator is unwrapped to its `.message` and rethrown.
8. `attachmentsDir = join(cwd, 'spec', 'attachments', workUnitId)`; `mkdir -p attachmentsDir`.
9. `fileName = basename(resolvedPath)`; `destPath = join(attachmentsDir, fileName)`; `copyFile(resolvedPath, destPath)`.
10. **BUG-055 dedup**: if `dirname(resolve(resolvedPath)) === resolve(cwd, 'spec', 'attachments')` (i.e., source is in spec/attachments/ root), `unlink(resolvedPath)` after successful copy.
11. `relativePath = relative(cwd, destPath)`.
12. `workUnit.attachments ??= []`.
13. If `workUnit.attachments.includes(relativePath)` → throw `"Attachment '<fileName>' already exists for work unit '<workUnitId>'"`.
14. `workUnit.attachments.push(relativePath)`.
15. `workUnit.updatedAt = new Date().toISOString()`.
16. `data.meta.lastUpdated = new Date().toISOString()` (if meta exists).
17. `fileManager.transaction(workUnitsPath, fileData => Object.assign(fileData, data))` — atomic write.
18. Output (3 lines):
    - `✓ Attachment added successfully`
    - `  File: <relativePath>`
    - If `description` provided: `  Description: <description>`

## CLI surface (Commander.js)

```
fspec add-attachment <workUnitId> <filePath> [options]
  -d, --description <text>  Optional description of the attachment
```

- 2 positional args (both required), 1 optional `-d/--description` flag.
- On error: `output.error('Error:', errorMessage); process.exit(1)`.

## Rust port architecture notes

### File layout (core)
- `codelet/fspec-core/src/commands/add_attachment.rs` — `pub async fn run(args_json, project_root) -> Result<String, FspecCoreError>`.
- Re-uses `io::ensure::ensure_work_units_file`, `io::locked_file::write_json_atomic`, `io::time::iso8601_now`.
- `attachments` field lives in `WorkUnit.extra` map (not yet promoted to typed field) — mirror the `list_attachments.rs` pattern: read from `extra.get("attachments")`, mutate via `extra.entry("attachments").or_insert(Value::Array(vec![]))`.
- **No mermaid validation in core**: this is a SCOPE SIMPLIFICATION. The TS implementation uses `jsdom` + dynamic `mermaid` import — an enormous transitive dependency tree (mermaid + jsdom + canvas + etc.) inappropriate for the lean `fspec-core` crate. The Rust port will SKIP mermaid validation; `.mmd`/`.mermaid`/`.md` files are copied unconditionally. This divergence is consistent with the `list-attachments` port which also omitted the JS Date.toLocaleString() time formatting. Tests assert the file is copied and added to the array; mermaid-syntax-error scenarios are NOT replicated.
- **No Unicode whitespace path resolution**: also a SCOPE SIMPLIFICATION. The TS `resolveFilePath` performs U+202F/U+00A0 fuzzy whitespace matching for macOS screenshot filenames (BUG-130). The Rust port uses the literal path — if the source file doesn't exist with the exact bytes provided, we surface the canonical "Source file '<filePath>' does not exist" error. macOS-specific Unicode normalisation can be added later if needed.

### Error messages (verbatim parity)
- `Work unit '<id>' does not exist`
- `Source file '<filePath>' does not exist` (uses ORIGINAL filePath, not resolved)
- `Attachment '<fileName>' already exists for work unit '<workUnitId>'`

### Output (verbatim parity)
- Success: `"✓ Attachment added successfully\n  File: <relativePath>\n"` plus optional `"  Description: <text>\n"` when description provided.

### CLI bridge (codelet/fspec/src/add_attachment.rs)
- clap variant: `AddAttachment { work_unit_id: String, file_path: String, description: Option<String> }`.
- Marshal JSON: `{workUnitId, filePath, description?}`.
- Stdout: print success text verbatim.
- Stderr on error: `Error: <reason>` (TS uses `output.error('Error:', message)`; mirror via `render_core_error`).
- Exit 0 on success, 1 on error.

### BUG-055 dedup behaviour
- Mirror this: if source is at `spec/attachments/<file>` (root, not subdir), copy then unlink the source. Reuses `std::path::Path::canonicalize` or simpler `std::path::Path::parent() == project_root.join("spec").join("attachments")` comparison.

### Help fixture
- Capture `node dist/index.js add-attachment --help` to `codelet/fspec/tests/fixtures/help/add-attachment.txt` after `npm run build`.
- Help config in `codelet/fspec-core/src/help/configs/add_attachment.rs` — mirror the TS quirks (typicalWorkflow comma-joined, relatedCommands "fspec " prefix doubling, commonErrors "Fix: undefined").
- TS help uses `solution:` keys, not `fix:` — yields "Fix: undefined" in output. Mirror with `fix: "undefined"`.
