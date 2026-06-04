# AST Research — RPC-248 `list-prefixes` Rust Port

Generated: 2026-06-04

## TypeScript source under port

`src/commands/list-prefixes.ts` (134 LOC)

### Top-level entities

| Pattern                                            | AST hit                                     |
|----------------------------------------------------|---------------------------------------------|
| `interface Prefix { ... }`                         | `list-prefixes.ts:7-11`                     |
| `interface PrefixesData { ... }`                   | `list-prefixes.ts:13-15`                    |
| `interface WorkUnit { id, status?, [k]: unknown }` | `list-prefixes.ts:17-21`                    |
| `interface WorkUnitsData { workUnits, states }`    | `list-prefixes.ts:23-26`                    |
| `interface PrefixWithProgress { ... }`             | `list-prefixes.ts:28-34`                    |
| `export async function listPrefixes(options)`      | `list-prefixes.ts:36`                       |
| `export function registerListPrefixesCommand(...)` | `list-prefixes.ts:101`                      |

### Control flow of `listPrefixes`

1. `cwd = options.cwd || process.cwd()` — `list-prefixes.ts:39`
2. Read `spec/prefixes.json` → ENOENT → early return `{ prefixes: [] }` (`list-prefixes.ts:48-52`)
3. Read `spec/work-units.json` → any error swallowed via bare `catch {}` (`list-prefixes.ts:57-63`)
4. For each prefix in `Object.values(prefixesData.prefixes)`:
   - `totalWorkUnits` = count of work-units whose `id.startsWith(prefix.prefix + '-')`
   - `completedWorkUnits` = subset with `status === 'done'`
   - `completionPercentage = total > 0 ? round((completed/total)*100) : 0`
5. Push `{ prefix, description, totalWorkUnits, completedWorkUnits, completionPercentage }`.

### CLI surface (Commander.js)

```ts
program
  .command('list-prefixes')
  .description('List all prefixes')
  .action(async () => { ... });
```

**No options registered.** No `--format`, no filters. The CLI hardcodes text-render.

### CLI text rendering (`list-prefixes.ts:107-123`)

- Empty: `output.log('No prefixes found')` → exit 0
- Populated: `\nPrefixes (N)\n\n`
- Per prefix: `PREFIX\n  description\n` + conditional `  Work Units: completed/total (pct%)\n` (only when `totalWorkUnits > 0`) + blank line
- Catch path: `output.error('Error:', error.message)` → exit 1

---

## Rust scaffold under port

`codelet/fspec-core/src/commands/list_prefixes.rs` (current stub):

```rust
pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "list-prefixes",
        work_unit: "RPC-248",
    })
}
```

**Signature divergence:** The stub takes `(_args_json)` only, but RPC-253's ported pattern is `(args_json, project_root)`. This port must change the signature and update `dispatch.rs` accordingly.

### Shared infrastructure already present (will be reused)

| Module                                             | Purpose                                          |
|----------------------------------------------------|--------------------------------------------------|
| `codelet/fspec-core/src/io/project_root.rs`        | `find_or_create_spec_directory(cwd)` for read-paths |
| `codelet/fspec-core/src/io/locked_file.rs`         | `read_or_init_json`, `write_json_atomic`         |
| `codelet/fspec-core/src/io/ensure.rs`              | `ensure_work_units_file`, `ensure_prefixes_file` |
| `codelet/fspec-core/src/types/work_unit.rs`        | `WorkUnit`, `WorkUnitsData`, `PrefixesData`      |

### Gaps to fill (per architecture notes)

1. **New helpers in `io::ensure`:**
   - `read_prefixes_or_empty(cwd) -> Result<PrefixesData, FspecCoreError>` — ENOENT → `Ok(empty)`, parse error → propagate.
   - `read_work_units_or_empty(cwd) -> Result<WorkUnitsData, FspecCoreError>` — ENOENT OR parse error → `Ok(initial)`.
2. **New type `types::prefix::Prefix`** — `{ prefix, description, created_at, extra }` mirroring TS Prefix interface.
3. **Refactor `PrefixesData.prefixes`** from `serde_json::Map<String, Value>` to `IndexMap<String, Prefix>` for insertion-order parity (preserves the `IndexMap` pattern used by `WorkUnitsData.work_units`).
4. **Wire `list-prefixes` into dispatcher `run_ported` arm** and add `"list-prefixes"` to `canonical::PORTED_COMMANDS`.
5. **Add `Mode::ListPrefixes` to `codelet/fspec/src/main.rs`** (flag-less variant) and a new `codelet/fspec/src/list_prefixes.rs` bridge module (delegates to `fspec_core::commands::list_prefixes::run` with project_root = CWD).

### Existing crate dependencies sufficient

- `indexmap` already in `codelet/fspec-core/Cargo.toml` (used by `WorkUnitsData`).
- `serde_json` already used.
- `tempfile` already in dev-deps.
- No new external crates needed.

### `is_ported` predicate

Located at `codelet/fspec-core/src/canonical.rs:204`. The `PORTED_COMMANDS` array (line 199) currently lists only `"list-work-units"` — RPC-248 will append `"list-prefixes"`.

---

## Test surface delta

| Component                           | New tests                                        |
|-------------------------------------|--------------------------------------------------|
| `io::ensure` helpers                | ENOENT and malformed-JSON branches per helper     |
| `types::prefix::Prefix`             | round-trip + extra-field preservation             |
| `types::work_unit::PrefixesData`    | IndexMap insertion-order regression               |
| `commands::list_prefixes::run`      | 12 dispatcher scenarios (rust-port feature file)  |
| `codelet/fspec/list_prefixes`       | 6 CLI scenarios (cli-subcommand feature file)     |
| `canonical::is_ported`              | `"list-prefixes"` now true                        |
| `dispatch::dispatch_command`        | tokio-runtime parity (regression for RPC-327)     |

## Non-goals (out of scope for RPC-248)

- `--workspace` flag on the subcommand (RPC-253 deferred this; precedent preserved).
- `--format` / `--filter` flags (not present in TS CLI surface).
- Migrating other commands that consume `PrefixesData` — only `ensure_prefixes_file` callers are touched, and the breaking change (`Map → IndexMap<String, Prefix>`) is backward-compatible at the JSON wire format.
