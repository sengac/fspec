# AST Research: Singleton Pattern & NAPI Bindings for KGRAPH-002

## Research Date: 2026-03-19

## 1. Existing Singleton Pattern (persistence/mod.rs)

```rust
// codelet/napi/src/persistence/mod.rs lines 42-48
lazy_static::lazy_static! {
    static ref MESSAGE_STORE: Mutex<Option<MessageStore>> = Mutex::new(None);
    static ref SESSION_STORE: Mutex<Option<SessionStore>> = Mutex::new(None);
    static ref BLOB_STORE: Mutex<Option<BlobStore>> = Mutex::new(None);
    static ref HISTORY_STORE: Mutex<Option<HistoryStore>> = Mutex::new(None);
}
```

All stores are reset to `None` in `set_data_directory()` (line 57-78).

## 2. Data Directory Management (codelet/common/src/data_dir.rs)

```rust
static DATA_DIRECTORY: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn set_data_directory(dir: PathBuf) -> Result<(), String>;
pub fn get_data_dir() -> Result<PathBuf, String>;
```

Graph module will use `get_data_dir()` to derive `{data_dir}/graph/agent-memory.nano/`.

## 3. Nanograph Database API (from /tmp/nanograph analysis)

```rust
// nanograph::store::database::Database
impl Database {
    pub async fn init(db_path: &Path, schema_source: &str) -> Result<Self>;
    pub async fn open(db_path: &Path) -> Result<Self>;
    pub async fn open_in_memory(schema_source: &str) -> Result<Self>;
    pub fn catalog(&self) -> &Catalog;
    pub fn schema_ir(&self) -> &SchemaIR;
    pub fn path(&self) -> &Path;
    pub fn is_in_memory(&self) -> bool;
}
```

Database wraps `Arc<DatabaseShared>` so it's `Clone + Send + Sync`.

## 4. Key Integration Points

- **Cargo.toml**: Add `nanograph = { git = "https://github.com/nanograph/nanograph" }` with feature gate
- **New module**: `codelet/napi/src/graph/mod.rs` — GRAPH_DB singleton, init/open/reset
- **Schema file**: `codelet/napi/schemas/agent-memory.pg` — bundled via `include_str!()`
- **persistence/mod.rs**: Add `crate::graph::reset_graph_db()` call in `set_data_directory()`
- **lib.rs**: Register NAPI bindings for graph_db_close()

## 5. NAPI Binding Pattern (from lib.rs)

```
mod persistence;
mod graph;  // <-- new
```

NAPI functions use `#[napi]` attribute macro, async functions wrap tokio runtime.
