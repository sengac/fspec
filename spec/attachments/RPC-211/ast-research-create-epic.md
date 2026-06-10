# RPC-211 — `create-epic` Rust port: AST/behaviour research

## TypeScript source
- Primary: `src/commands/create-epic.ts` (128 lines)
- Help: `src/commands/create-epic-help.ts`

## Behaviour observations from TS source

### Inputs (CLI surface, src/commands/create-epic.ts:115-127)
- Positional `<epicId>` (required) — must match regex `/^[a-z][a-z0-9]*(-[a-z0-9]+)*$/`
  - First char: lowercase letter
  - Subsequent: lowercase letters, digits, hyphen-separated groups
- Positional `<title>` (required)
- Option `-d, --description <description>` (optional)

### Inputs (programmatic surface, lines 21-26)
- `options.epicId: string`
- `options.title: string`
- `options.description?: string`
- `options.cwd?: string` (defaults to `process.cwd()`)

### Validation rules (line 31-35)
- `EPIC_ID_REGEX.test(epicId)` — invalid → throw `Error("Epic ID must be lowercase-with-hyphens format (e.g., epic-user-management)")`
- Note: TS wraps the throw in the outer try/catch which prepends `"Failed to create epic: "` to the error message (lines 74-78). So observable error becomes `"Failed to create epic: Epic ID must be lowercase-with-hyphens format (e.g., epic-user-management)"`.

### Side effects
1. `mkdir(spec/, { recursive: true })` (line 39) — auto-create spec dir
2. Read `spec/epics.json` if exists (lines 42-48). ENOENT → initialize empty `{ epics: {} }`. ANY other error caught the same way (bare catch).
3. Duplicate-check `data.epics[epicId]` (line 51-53) — exists → throw `"Epic ${epicId} already exists"` (wrapped → `"Failed to create epic: Epic ${id} already exists"`).
4. Construct new epic:
   ```js
   { id, title, createdAt: new Date().toISOString() }
   ```
   If description provided, add it AFTER `title` (line 62-64).
   Field order: `id`, `title`, [`description`], `createdAt`.
5. Insert into `data.epics[epicId]`.
6. Write via `fileManager.transaction(epicsFile, async fileData => { Object.assign(fileData, data); })` — atomic locked write.

### Output (CLI wrapper, lines 95-100)
On success:
```
✓ Created epic <id>
  Title: <title>
[  Description: <desc>    ← only when description present]
```
Exit code 0.

### Output (errors, lines 102-112)
`output.error('Error:', error.message)` to stderr, exit 1.
Error message format (per outer try/catch): `Failed to create epic: <inner message>`.

### Note on file behaviour
- The TS implementation does NOT route create-epic through `ensureEpicsFile` — it reads directly via `readFile` and falls back to `{ epics: {} }` on ANY read error. This means a malformed `spec/epics.json` is silently treated as empty and OVERWRITTEN, **destroying existing data**. We will preserve this exact behaviour for parity (matching the rule "TS bare catch swallows all errors").

## Rust port plan

### File layout
- `codelet/fspec-core/src/commands/create_epic.rs` — `async fn run(args_json, project_root) -> Result<String, FspecCoreError>`. Returns rendered text string. Errors → `FspecCoreError::InvalidArgs { command: "create-epic", reason: <msg> }` so dispatcher exposes `Failed to create epic: ...` text in the `error` field.
- `codelet/fspec-core/src/help/configs/create_epic.rs` — port `create-epic-help.ts`.
- `codelet/fspec/src/create_epic.rs` — clap bridge, marshals args to JSON, delegates.
- `codelet/fspec/tests/cli_create_epic.rs` — CLI E2E.
- `codelet/fspec/tests/fixtures/help/create-epic.txt` — help fixture.
- `codelet/fspec-core/tests/create_epic.rs` — dispatcher integration tests.

### Args struct
```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateEpicArgs {
    epic_id: String,
    title: String,
    #[serde(default)]
    description: Option<String>,
}
```

### Atomic write
- Use `io::locked_file::write_json_atomic` directly (already exists).
- Use IndexMap (via existing `EpicsData`) so field insertion preserves order.

### Field-order parity for epic record
Need to emit `id`, `title`, [`description`], `createdAt` IN THAT ORDER. The typed `Epic` struct has `id, title, description, extra`; the `description` field skipped when None. `createdAt` lives in `extra`. Inserting into the extra map AFTER construction yields correct serialization order via serde-flatten + IndexMap-style `serde_json::Map`. (Note: `Map` is BTreeMap by default — alphabetical. To force insertion order, build via `Value::Object` with `preserve_order` feature.)

**Decision**: I need to inspect whether `serde_json` is built with `preserve_order` in this workspace. If not, the cleanest approach is a dedicated `#[derive(Serialize)]` struct with explicit field declaration order (id, title, description, createdAt) — like `show_epic`'s `ShowEpicResult`. Will use that pattern.

For storing into `EpicsData.epics: IndexMap<String, Epic>`, the typed `Epic` struct serialization writes id, title, description, then extra. `createdAt` lives in extra (an `serde_json::Map`). When extra has only `createdAt`, serialization yields `id, title, description, createdAt` which matches TS field order ✓.

### Shared-file requests for supervisor
Need a new helper in `io/ensure.rs`: a write-counterpart for epics. Specifically:
- Option A: `write_epics_atomic(project_root, &EpicsData)` thin wrapper around `write_json_atomic`. Could be done in-module by importing locked_file directly — no shared file change needed.
- Option B: an `ensure_epics_file` helper that auto-creates. Not needed here — we use `read_epics_or_empty` (already exists) for reads, and call `write_json_atomic` directly on the path.

**Conclusion**: No shared-file changes needed for RPC-211. Use existing `read_epics_or_empty` + `write_json_atomic`.

### Text rendering
```
✓ Created epic <id>
  Title: <title>
  Description: <desc>    ← only when present
```
With trailing newline.

### Error scenarios
| Trigger | Rust error |
|--------|-----------|
| Missing `epicId` field in args | InvalidArgs "failed to parse args: ..." |
| epicId fails regex | InvalidArgs "Failed to create epic: Epic ID must be lowercase-with-hyphens format (e.g., epic-user-management)" |
| Epic already exists | InvalidArgs "Failed to create epic: Epic <id> already exists" |
| Atomic write I/O failure | InvalidArgs "Failed to create epic: <io msg>" — to keep parity with TS outer-catch wrapping |

### Dispatcher output
Success → return text. Mirrors TS `output.log` block.
JSON format? TS doesn't expose `--format` for create-epic. So no JSON output mode needed.

## Scenario inventory (preview)

### Dispatcher (`create-epic-rust-port.feature`)
1. Creates epic from minimal args (id+title) and writes spec/epics.json with id, title, createdAt
2. Creates epic with description and writes the description field
3. Preserves existing epics when adding a new one
4. Rejects invalid epicId format with regex error
5. Rejects duplicate epic id with already-exists error
6. Tolerates missing spec/ directory by creating it before writing
7. Field-order parity on disk: id, title, description, createdAt
8. Treats malformed spec/epics.json as empty store (TS bare-catch parity)
9. Returns args-parse error when epicId field is missing

### CLI (`create-epic-cli-subcommand.feature`)
1. clap exposes create-epic with --help showing arguments and -d flag
2. CLI creates epic and prints success block to stdout (exit 0)
3. CLI with -d prints Description line in success block
4. CLI exits 1 on invalid epicId, stderr contains "Error:" + canonical message
5. CLI exits 1 on duplicate, stderr contains "already exists"
6. `--help` byte-for-byte matches TS help fixture
