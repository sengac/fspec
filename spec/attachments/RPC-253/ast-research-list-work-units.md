# AST Research — RPC-253 (Port `list-work-units` command to Rust)

**Date:** 2026-06-01
**Goal:** Map the TypeScript surface for the `list-work-units` command and its transitive shared utilities, before defining Rust port targets.

---

## 1. Target command — `src/commands/list-work-units.ts`

### Exports

| Symbol | Lines | Role |
|---|---|---|
| `listWorkUnits` | 29-74 | Pure logic — load store, apply filters, return summaries |
| `listWorkUnitsCommand` | 77-124 | CLI wrapper — format JSON/text, exit codes |
| `registerListWorkUnitsCommand` | 126-141 | Commander registration |

### Internal types

```typescript
interface ListWorkUnitsOptions { status?: string; prefix?: string; epic?: string; type?: WorkUnitType; cwd?: string; }
interface WorkUnitSummary { id: string; title: string; status: string; epic?: string; [k: string]: unknown; }
interface ListWorkUnitsResult { workUnits: WorkUnitSummary[]; }
```

### Filter chain (lines 41-69) — exact contract to mirror

```typescript
let workUnits = Object.values(workUnitsData.workUnits);          // line 41 — insertion order
if (options.status) workUnits = workUnits.filter(wu => wu.status === options.status);
if (options.prefix) workUnits = workUnits.filter(wu => wu.id.startsWith(`${options.prefix}-`));  // hyphen appended!
if (options.epic)   workUnits = workUnits.filter(wu => wu.epic === options.epic);
if (options.type)   workUnits = workUnits.filter(wu => (wu.type || 'story') === options.type);   // default story
const summaries = workUnits.map(wu => ({
  id: wu.id, title: wu.title, status: wu.status,
  ...(wu.epic && { epic: wu.epic }),                              // omit when falsy
}));
```

### CLI options (line 127-137)

| Flag | Description | Default |
|---|---|---|
| `-s, --status <s>` | filter by status | — |
| `-p, --prefix <p>` | filter by prefix | — |
| `-e, --epic <e>` | filter by epic | — |
| `-t, --type <t>` | story / task / bug | — |
| `--format <f>` | text or json | `text` |

### Output (lines 92-113)

**JSON:** `JSON.stringify({ workUnits: [...] }, null, 2)`

**Text:** `\nWork Units (N)\n\n{ID [status]\n  {title}\n  Epic: {epic}\n\n}…` — `No work units found` when empty.

### Error path (lines 116-123)

`output.error('Error:', err.message)` + `process.exit(1)`. SyntaxError on JSON parse is rewrapped by `ensureWorkUnitsFile` as `Failed to parse work-units.json: ...`.

---

## 2. Shared TS utilities to port — Rust target modules

### 2.1 `src/utils/ensure-files.ts:17-56` — `ensureWorkUnitsFile`

**Logic:**
1. `findOrCreateSpecDirectory(cwd)` → spec dir path.
2. Build default `WorkUnitsData` with `version=CURRENT_VERSION` (`"0.7.1"`), `meta.version="1.0.0"`, `meta.lastUpdated=ISO now`, empty `workUnits`, all 7 Kanban states empty.
3. `fileManager.readJSON(filePath, initialData)` — auto-creates file with default if ENOENT.
4. `ensureLatestVersion(cwd, data, CURRENT_VERSION)` — runs migrations.
5. On `SyntaxError`: rethrow `"Failed to parse work-units.json: <msg>. The file may be corrupted or contain invalid JSON."`

**Rust target:** `codelet/fspec-core/src/io/ensure.rs` → `ensure_work_units_file(project_root) -> Result<WorkUnitsData, FspecCoreError>`

### 2.2 `ensurePrefixesFile` (lines 64-73)

Same pattern, file `prefixes.json`, default `{ "prefixes": {} }`.

**Rust target:** same module → `ensure_prefixes_file(project_root) -> Result<PrefixesData, FspecCoreError>`

### 2.3 `src/utils/project-root-detection.ts:37-69` — `findOrCreateSpecDirectory`

**Logic (test-isolation-friendly):**
1. If `cwd/spec` already exists as dir → return it (test isolation).
2. Walk upward up to 10 levels checking for boundary markers (`.git`, `package.json`, `.gitignore`, `Cargo.toml`, `pyproject.toml`).
3. If existing `spec/` found in a project boundary → return that path.
4. Otherwise find project root via markers and create `spec/` there.
5. Fallback on any error: `cwd/spec` (mkdir -p).

**Rust target:** `codelet/fspec-core/src/io/project_root.rs` → `find_or_create_spec_directory(cwd: &Path) -> io::Result<PathBuf>`

### 2.4 `src/utils/file-manager.ts` — `LockedFileManager`

**Logic (essential subset for read-only `list-work-units`):**
- `readJSON(path, defaultData)`: if file missing, write `defaultData` and return it; else parse.
- 3-layer locking (proper-lockfile + in-process RW + atomic rename) — only the read+create path is needed for this port.

**Rust target:** `codelet/fspec-core/src/io/locked_file.rs` →
- `read_or_init_json<T: DeserializeOwned + Serialize>(path: &Path, default: &T) -> Result<T, FspecCoreError>`
- `write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), FspecCoreError>`

Locking strategy for Phase 1: use `fs2::FileExt::try_lock_exclusive` with brief retry — `proper-lockfile` parity can be added later when porting commands that genuinely concurrent-write. For `list-work-units` (read-mostly), an exclusive lock during the read+init window is sufficient.

### 2.5 Type definitions — `src/types/index.ts:131-195`

**Rust target:** `codelet/fspec-core/src/types/work_unit.rs`

Key structs (only fields required for `list-work-units`; the rest are `#[serde(flatten)] extra: serde_json::Map<String, serde_json::Value>` to round-trip transparently):

```rust
pub enum WorkUnitType { Story, Task, Bug }   // serde rename_all = "lowercase"

pub enum WorkUnitStatus { Backlog, Specifying, Testing, Implementing, Validating, Done, Blocked }

pub struct WorkUnit {
  pub id: String,
  pub title: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub r#type: Option<WorkUnitType>,
  pub status: WorkUnitStatus,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub epic: Option<String>,
  // … all other fields preserved via serde_json::Value catch-all
  #[serde(flatten)]
  pub extra: serde_json::Map<String, serde_json::Value>,
  pub created_at: String,
  pub updated_at: String,
}

pub struct WorkUnitsData {
  #[serde(default)] pub version: Option<String>,
  #[serde(default)] pub meta: Option<Meta>,
  pub work_units: IndexMap<String, WorkUnit>,   // preserves insertion order
  pub states: WorkUnitStates,
  #[serde(flatten)]
  pub extra: serde_json::Map<String, serde_json::Value>,
}

pub struct WorkUnitStates {
  pub backlog: Vec<String>, pub specifying: Vec<String>, pub testing: Vec<String>,
  pub implementing: Vec<String>, pub validating: Vec<String>,
  pub done: Vec<String>, pub blocked: Vec<String>,
}
```

`IndexMap<String, WorkUnit>` from the `indexmap` crate is the Rust equivalent of `Record<string, WorkUnit>` with `Object.values()` insertion-order semantics.

---

## 3. Constants

| TS | Value | Rust target |
|---|---|---|
| `CURRENT_VERSION` (`src/migrations/registry.ts`) | `"0.7.1"` | `pub const CURRENT_VERSION: &str = "0.7.1";` in `types::work_unit` |
| `BOUNDARY_MARKERS` | `[".git", "package.json", ".gitignore", "Cargo.toml", "pyproject.toml"]` | const slice in `io::project_root` |
| `MAX_SEARCH_DEPTH` | `10` | const u32 in `io::project_root` |

---

## 4. Existing Rust state

| File | Status |
|---|---|
| `codelet/fspec-core/src/commands/list_work_units.rs` | Stub returning `FspecCoreError::NotYetPorted` |
| `codelet/fspec-core/src/io/*` | Does NOT exist — must be created |
| `codelet/fspec-core/src/types/*` | Does NOT exist — must be created |

---

## 5. Crate dependencies to add to `codelet/fspec-core/Cargo.toml`

```toml
indexmap = { version = "2", features = ["serde"] }
fs2 = "0.4"                      # cross-platform file locking
chrono = { version = "0.4", default-features = false, features = ["clock", "serde"] }  # ISO timestamps for Meta.lastUpdated
```

Workspace-shared `serde`/`serde_json`/`thiserror`/`tokio`/`tempfile` already present.

---

## 6. Error variants to add to `FspecCoreError`

```rust
#[error("I/O error executing fspec command {command}: {source}")]
Io { command: &'static str, #[source] source: std::io::Error },

#[error("Failed to parse {file}: {reason}. The file may be corrupted or contain invalid JSON.")]
ParseJson { file: String, reason: String },
```

These are general-purpose and will be reused by every subsequent ported command.
