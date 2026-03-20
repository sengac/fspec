# Nanograph Crates & API Surface

## Rust Crate Architecture (4 crates)

### 1. `nanograph` (core library) — `crates/nanograph/`
The heart of the system. All domain logic. v1.0.0, MIT, edition 2024.

| Module | Role |
|--------|------|
| `schema/` | `schema.pest` grammar + parser → `SchemaFile` AST (NodeDecl, EdgeDecl, PropDecl, Annotation) |
| `query/` | `query.pest` grammar + parser → `QueryFile` AST; `typecheck.rs` validates against catalog with 21 type rules |
| `catalog/` | `Catalog` (runtime type registry with `NodeType`/`EdgeType`/`arrow_schema`) + `schema_ir.rs` |
| `ir/` | `lower.rs` — lowers typed AST into flat `QueryIR` (pipeline of `IROp`: NodeScan, Expand, Filter, AntiJoin) or `MutationIR` |
| `plan/planner.rs` | Converts IR to DataFusion `ExecutionPlan` trees |
| `plan/physical.rs` | Custom `ExpandExec` (CSR/CSC traversal), `CrossJoinExec`, `AntiJoinExec`, mutation execution |
| `plan/node_scan.rs` | Custom `NodeScanExec` with Lance filter pushdown |
| `store/database.rs` | `Database` struct — Lance-backed persistence, init/open/open_in_memory, load modes, delete, compact/cleanup/doctor |
| `store/csr.rs` | CSR/CSC adjacency structure — core graph index for traversal |
| `store/loader/` | Load orchestration: JSONL parsing, @unique enforcement, @key merge, embedding materialization |
| `store/migration.rs` | Schema evolution engine (diff, safety levels) |
| `store/txlog.rs` | Transaction catalog + CDC log |
| `embedding.rs` | OpenAI embedding client with retry logic and mock mode |

### 2. `nanograph-cli` — `crates/nanograph-cli/`
Thin clap wrapper. Commands: `init`, `load`, `check`, `run`, `delete`, `embed`, `migrate`, `describe`, `export`, `version`, `compact`, `cleanup`, `doctor`, `cdc-materialize`, `changes`, `schema-diff`

### 3. `nanograph-ffi` (C ABI) — `crates/nanograph-ffi/`
C-compatible FFI for Swift/native clients. Produces `cdylib`/`staticlib`. Exports:
- `nanograph_db_init/open/open_in_memory/close/destroy`
- `nanograph_db_load/load_file/run/run_arrow/check/describe`
- `nanograph_db_compact/cleanup/doctor`
- Thread-local error model

### 4. `nanograph-ts` (Node.js SDK) — `crates/nanograph-ts/`
**npm package `nanograph-db`** via napi-rs. This is the integration path for fspec.

```typescript
// Factory methods
const db = await Database.init(dbPath, schemaSource);
const db = await Database.open(dbPath);
const db = await Database.openInMemory(schemaSource);

// Data operations
await db.load(jsonlString, 'overwrite' | 'append' | 'merge');
await db.loadFile(filePath, mode);

// Queries
const results = await db.run(querySource, queryName, params?); // → JSON array
const buffer = await db.runArrow(querySource, queryName, params?); // → Arrow IPC Buffer

// Schema
const schema = await db.check(querySource); // typecheck queries
const info = await db.describe(); // schema introspection

// Maintenance
await db.compact(options?);
await db.cleanup(options?);
await db.doctor();
await db.close();
```

## Key Rust Library API

```rust
// Schema compilation
pub use catalog::build_catalog;            // SchemaFile → Catalog
pub use catalog::schema_ir;                // SchemaIR, build_schema_ir

// Query pipeline
pub use ir::ParamMap;                      // HashMap<String, Literal>
pub use ir::lower::{lower_query, lower_mutation_query};
pub use plan::planner::execute_query;

// Results
pub use result::{MutationResult, QueryResult, RunResult};

// Database
Database::init(path, schema_source)
Database::open(path)
Database::open_in_memory(schema_source)
db.load_with_mode(data, mode)
db.prepare_read_query(query) → PreparedQuery → .execute(params) → QueryResult
db.run_query(query, params) → RunResult
```

## Dependencies

Arrow 57, DataFusion 52, Lance 3.0, Pest 2 (parser), tokio, serde, napi-rs (for TS), reqwest (for embeddings), ahash, tempfile.
