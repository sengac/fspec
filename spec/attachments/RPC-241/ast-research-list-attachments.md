# AST research — `list-attachments` Rust port (RPC-241)

## 1. TypeScript source-of-truth

File: `src/commands/list-attachments.ts` (77 LoC)

### Public surface

```ts
export interface ListAttachmentsOptions {
  workUnitId: string;
  cwd?: string;
}

export async function listAttachments(options: ListAttachmentsOptions): Promise<void>
export function registerListAttachmentsCommand(program: Command): void
```

### Commander.js registration (lines 62-77)

```
.command('list-attachments')
.description('List all attachments for a work unit')
.argument('<workUnitId>', 'Work unit ID')
.action(async (workUnitId: string) => { ... })
```

**Flags exposed:** NONE. There is a single positional argument `<workUnitId>` and **no `.option(...)` calls**. This mirrors `list-prefixes` (flag-less) but with one required positional.

### Behaviour breakdown (lines 14-60)

| Step | TS source | Behaviour |
|------|-----------|-----------|
| 1 | `cwd = options.cwd || process.cwd()` | Resolve project root. |
| 2 | `data = await ensureWorkUnitsFile(cwd)` | Auto-create `spec/work-units.json` if missing (load-or-init, NOT read-only). |
| 3 | `if (!data.workUnits[options.workUnitId])` | Validate the work unit exists; throw `Work unit '<id>' does not exist`. |
| 4 | `workUnit.attachments == null/empty` | Print `No attachments found for work unit <id>` (chalk.yellow) and **return success** (no error). |
| 5 | header | Print `\nAttachments for <id> (<N>):\n` (chalk.bold). |
| 6 | per-attachment | For each `attachment` (relative path string): `fullPath = join(cwd, attachment)`. |
| 7 | `stat(fullPath)` success | Print `  ✓ <attachment>`, `    Size: <KB> KB` (rounded to 2 dp, dividing bytes by 1024), `    Modified: <stats.mtime.toLocaleString()>\n`. |
| 8 | `stat` throws | Print `  ✗ <attachment>` and `    File not found on filesystem\n`. |
| 9 | error in action | catch → `output.error('Error:', message)` then `process.exit(1)`. |

### TS error contract

- Missing work unit: `Error: Work unit '<id>' does not exist` → exit 1, stderr.
- Other thrown errors (e.g. malformed work-units.json) bubble out and exit 1.

### Output format details

- Header: `\nAttachments for <ID> (<COUNT>):\n` — that's a leading newline, then the header line, then `:` + `\n` (chalk.bold wraps the entire string, including both newlines).
- Each attachment block:
  - Line 1: `  ✓ <relPath>` OR `  ✗ <relPath>`
  - Line 2 (✓): `    Size: <KB> KB`
  - Line 3 (✓): `    Modified: <localeString>\n` — the trailing `\n` is INSIDE the chalk arg, producing a blank line between attachments.
  - Line 2 (✗): `    File not found on filesystem\n` — same trailing `\n`.

> `output.log(...)` is just `console.log` semantics — `console.log` appends its own `\n`. The trailing `\n` in `"...\n"` therefore adds a SECOND newline, giving a blank line between attachments.

### Sentinel output (no attachments)

```
No attachments found for work unit <ID>
```

(chalk.yellow; followed by console.log's own `\n`.)

## 2. Differences vs `list-prefixes`

| Aspect | list-prefixes | list-attachments |
|--------|---------------|------------------|
| Flags | NONE | NONE |
| Positional args | NONE | `<workUnitId>` (required) |
| Reads work-units.json | YES (silently empty on any failure) | YES (auto-create + escalate parse errors via `ensureWorkUnitsFile`) |
| Reads prefixes.json | YES (escalate parse errors) | NO |
| Validates target exists | N/A | YES → error if `data.workUnits[id]` undefined |
| Filesystem `stat` calls | NO | YES (per attachment, swallow ENOENT into ✗ marker) |
| Empty result behaviour | `No prefixes found` sentinel | `No attachments found for work unit <id>` sentinel (different per-id text) |

## 3. Rust port design

### Shared infra reuse

- `crate::io::ensure::ensure_work_units_file` — **reuse**. This already auto-creates and escalates parse errors.
- `crate::types::work_unit::{WorkUnit, WorkUnitsData}` — needs `attachments: Option<Vec<String>>` field added to `WorkUnit`. Currently `WorkUnit` flattens unknowns into `extra`, so attachments round-trip already, but typed access requires either pulling from `extra` or a new typed field. Two options:
  - **Option A (preferred):** Add `#[serde(default, skip_serializing_if = "Option::is_none")] attachments: Option<Vec<String>>` to `WorkUnit`. Minimal, locally-scoped change.
  - **Option B:** Read attachments out of `wu.extra` via `extra.get("attachments")`. Avoids touching `WorkUnit` but is uglier.
- Need new helper `format_size_kb(bytes: u64) -> String` that matches JS `(n/1024).toFixed(2)` (round half-to-even? actually JS `toFixed` uses round-half-away-from-zero in V8 but the spec is implementation-defined). For positive non-zero values the difference vs round-half-to-even is rare; we'll mirror by computing `((bytes as f64) / 1024.0 * 100.0).round() / 100.0` and formatting `"{:.2}"`. Verified above: `1234/1024 → "1.21"`, `0/1024 → "0.00"`.
- Need new helper `format_mtime_locale(mtime: SystemTime) -> String` for the `Modified:` line. ⚠ JS `Date.toLocaleString()` is locale- and TZ-sensitive; bit-for-bit parity is impossible across processes. Acceptable compromise: match the TS *runtime behaviour on the test host* by producing a non-empty ISO-ish string. The TS feature does not pin the format, so the Rust port should:
  - Emit `<YYYY-MM-DD HH:MM:SS>` (UTC) for determinism.
  - Document the deviation as an acceptance-criteria note ("Modified line is informational; format is not bit-stable across Node and Rust").

### Args struct

```rust
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ListAttachmentsArgs {
    work_unit_id: Option<String>,  // required; missing → InvalidArgs
    #[serde(default)]
    format: Option<String>,        // "text" (default) | "json" (dispatcher-only)
}
```

### Function signature

```rust
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>
```

### Render

- Text format mirrors TS:
  - Empty: `No attachments found for work unit <id>\n`
  - Non-empty:
    ```
    \n
    Attachments for <id> (<N>):\n
    \n
      ✓ <path>\n
        Size: <KB> KB\n
        Modified: <ts>\n
    \n
      ✗ <missing>\n
        File not found on filesystem\n
    \n
    ```
- JSON format (dispatcher only, parity with list-prefixes/list-work-units convention):
  ```
  {
    "workUnitId": "...",
    "attachments": [
      { "path": "spec/attachments/X/foo.png", "exists": true,  "sizeKb": "1.21", "modified": "2026-06-01 12:34:56" },
      { "path": "spec/attachments/X/old.png", "exists": false }
    ]
  }
  ```

### CLI bridge (`codelet/fspec/src/list_attachments.rs`)

```rust
pub struct CliArgs {
    pub work_unit_id: String,
}

pub async fn run(args: CliArgs) -> Result<u8>
```

Marshalling: `{"workUnitId": "<id>"}` → `list_attachments::run`.

### Clap variant in `main.rs`

```rust
#[command(name = "list-attachments", about = "List all attachments for a work unit")]
ListAttachments {
    /// Work unit ID (e.g. AUTH-001).
    work_unit_id: String,
},
```

### Dispatcher routing

Move `list-attachments` arm from `run_stub` into `run_ported`, and add `"list-attachments"` to `PORTED_COMMANDS`.

## 4. Files this RPC will touch

ISOLATED (this worker):
- `codelet/fspec-core/src/commands/list_attachments.rs` — rewrite stub.
- `codelet/fspec-core/tests/list_attachments.rs` — NEW dispatcher integration test.
- `codelet/fspec/src/list_attachments.rs` — NEW CLI bridge module.
- `codelet/fspec/tests/cli_list_attachments.rs` — NEW CLI integration test.
- `spec/features/list-attachments-rust-port.feature` — NEW feature.
- `spec/features/list-attachments-cli-subcommand.feature` — NEW feature.
- `codelet/fspec-core/src/types/work_unit.rs` — **MUST CHANGE (typed `attachments` field on `WorkUnit`)**. This is a shared file; will request supervisor to either authorise the field addition OR I read attachments from `extra` to stay isolated.

SHARED (ask supervisor):
- `codelet/fspec-core/src/canonical.rs` — add `"list-attachments"` to `PORTED_COMMANDS`.
- `codelet/fspec-core/src/dispatch.rs` — add arm to `run_ported`, remove arm from `run_stub`.
- `codelet/fspec/src/main.rs` — register `mod list_attachments;`, add `Mode::ListAttachments { work_unit_id: String }` variant + match arm, extend long_about.
- `codelet/fspec/tests/cargo_shape.rs` — add `"list_attachments.rs"` to the locked file list (raises 8 → 9).

## 5. Risk / hotspots

1. **`toLocaleString` parity** — impossible to match Node's `Date.toLocaleString()` exactly across hosts. The Rust port should emit a deterministic UTC ISO-like string and document the divergence as a non-blocking deviation. The Gherkin acceptance criteria MUST NOT assert the literal Modified text — only assert the line PREFIX (`    Modified: `).
2. **`toFixed(2)` parity** — V8's `toFixed` for `0/1024` = `"0.00"`. Rust `"{:.2}"` on `0.0` also = `"0.00"`. For `1234/1024` (= 1.205078125): V8 = `"1.21"`, Rust `(1.205078125 * 100).round() / 100 = 1.21`. Edge case: ties (`x.xx5`) — V8 uses round-half-to-even via the underlying double, Rust `round()` uses round-half-away-from-zero. Rare in practice for file sizes; document as a known deviation if needed.
3. **`attachments` field on `WorkUnit`** — currently round-trips via `extra`. If we add a typed field, no other ported command reads it, so the only risk is serde write-back ordering. Since we never write back in `list-attachments`, this is a pure read-only addition with `#[serde(default, skip_serializing_if = "Option::is_none")]` so it's invisible when absent.
4. **Trailing-newline parity** — TS prints `\nAttachments for...:\n` via `output.log` which adds its own `\n` → effective `\n\nAttachments for...:\n\n` at start of populated output. The Rust render must produce exactly the same byte sequence to satisfy the substring-line scenarios.
