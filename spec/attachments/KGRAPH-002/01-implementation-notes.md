# KGRAPH-002: Nanograph Database Lifecycle — Implementation Notes

## Integration Approach

Nanograph is a Rust crate — since the fspec NAPI layer is already Rust, we integrate it
as a **direct Cargo dependency**, not via the npm package. This gives us:
- Full Rust API access (not limited to the NAPI surface)
- Shared tokio runtime (nanograph uses tokio internally)
- No JS↔native boundary overhead for graph operations

## Cargo Dependency

Add to `codelet/napi/Cargo.toml`:

```toml
[dependencies]
# ... existing deps ...
nanograph = { git = "https://github.com/nanograph/nanograph", branch = "main" }
```

Or as a git submodule + path dependency if we need to pin/patch:

```toml
nanograph = { path = "../../vendor/nanograph/crates/nanograph" }
```

## Singleton Pattern

Follow **Pattern A** (same as MessageStore/SessionStore) — `lazy_static Mutex<Option<T>>`:

```rust
// New file: codelet/napi/src/graph/mod.rs
use lazy_static::lazy_static;
use std::sync::Mutex;
use nanograph::store::database::Database;

lazy_static! {
    static ref GRAPH_DB: Mutex<Option<Database>> = Mutex::new(None);
}
```

Key requirement: `Database` must be `Send` (it wraps `Arc<RwLock<...>>` internally,
so this should work). Verify during implementation.

## Directory Structure

The graph lives in the global `~/.fspec/` data directory alongside sessions and messages:

```
~/.fspec/
├── messages/                          # Existing message store
├── sessions/                          # Existing session manifests
├── blobs/                             # Existing blob store
└── graph/
    └── agent-memory.nano/             # Nanograph database
        ├── schema.pg
        ├── schema.ir.json
        ├── graph.manifest.json
        ├── _tx_catalog.jsonl
        ├── _cdc_log.jsonl
        ├── nodes/
        └── edges/
```

## Schema Bundling

The `agent-memory.pg` schema is compiled into the binary via `include_str!`:

```rust
const AGENT_MEMORY_SCHEMA: &str = include_str!("../schemas/agent-memory.pg");
```

Place the schema file at `codelet/napi/schemas/agent-memory.pg`.

## Init/Open Decision Tree

```
graph_db_ensure_open() →
  ├── GRAPH_DB already Some? → return Ok(())
  ├── ~/.fspec/graph/agent-memory.nano/ exists?
  │   ├── YES → Database::open(path) → store in GRAPH_DB
  │   └── NO  → mkdir -p → Database::init(path, AGENT_MEMORY_SCHEMA) → store
  └── On error → return Err (don't store None → next call retries)
```

## Reset on Data Directory Change

In `persistence/mod.rs::set_data_directory()`, add:

```rust
// Reset graph database when data directory changes
crate::graph::reset_graph_db();
```

## Public API (for other NAPI modules)

```rust
// codelet/napi/src/graph/mod.rs

/// Ensure graph DB is open, initializing if needed. Returns guard.
pub fn with_graph_db<F, R>(project_root: &Path, f: F) -> Result<R, String>
where F: FnOnce(&Database) -> Result<R, String>;

/// Reset the singleton (called on data directory change)
pub fn reset_graph_db();

/// Check if graph is initialized for this project
pub fn is_graph_initialized(project_root: &Path) -> bool;
```

## Process Exit Cleanup

Register a cleanup hook (nanograph Database drops cleanly via Drop trait,
but we should explicitly close to flush any pending writes):

```rust
// Called from TypeScript process exit handler
#[napi]
pub fn graph_db_close() -> napi::Result<()> {
    let mut db = GRAPH_DB.lock().map_err(|e| Error::from_reason(e.to_string()))?;
    *db = None; // Drop triggers Database cleanup
    Ok(())
}
```
