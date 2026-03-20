# KGRAPH-003: GraphSearch Tool — Implementation Notes

## Tool Registration Pattern

Follows the same pattern as `SessionSearchTool` and `DeepSearchTool`:

1. **Tool definition** in `codelet/tools/src/graph_search/`
2. **Handler** registered per-session via `HashMap<Uuid, GraphSearchHandler>`
3. **NAPI binding** in `codelet/napi/src/graph_search_handler.rs`

## Tool Schema

```rust
pub struct GraphSearchTool {
    session_id: Uuid,
}

#[derive(Deserialize, JsonSchema)]
pub struct GraphSearchArgs {
    pub action_type: GraphSearchAction,

    // search
    pub query: Option<String>,
    pub category: Option<String>,
    pub limit: Option<usize>,

    // neighbors
    pub node_id: Option<String>,
    pub depth: Option<usize>,
    pub edge_types: Option<Vec<String>>,

    // path
    pub from: Option<String>,
    pub to: Option<String>,
    pub max_hops: Option<usize>,

    // related
    pub topic: Option<String>,
    pub min_strength: Option<f32>,

    // decisions
    pub domain: Option<String>,
    pub status: Option<String>,
    pub since: Option<String>,

    // history
    pub concept: Option<String>,

    // index
    pub scope: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GraphSearchAction {
    Search,
    Neighbors,
    Path,
    Related,
    Decisions,
    History,
    Stats,
    Index,
}
```

## NanographBridge

Thin wrapper that translates GraphSearch actions into nanograph queries:

```rust
pub struct NanographBridge;

impl NanographBridge {
    pub fn search(query: &str, category: Option<&str>, limit: usize)
        -> Result<String, String>;
    pub fn neighbors(node_id: &str, depth: usize, edge_types: Option<&[String]>)
        -> Result<String, String>;
    pub fn path(from: &str, to: &str, max_hops: usize)
        -> Result<String, String>;
    // ... one method per action
}
```

Each method:
1. Acquires the graph DB singleton (auto-inits if needed)
2. Builds the appropriate `.gq` query string with parameters
3. Calls `db.run_query()` (or the Rust equivalent)
4. Converts the Arrow result to JSON
5. Returns formatted string for the LLM

## Handler Registration

Same pattern as SessionSearch — registered when a session starts:

```rust
pub fn register_graph_search_handler(session_id: Uuid) {
    let handler = GraphSearchHandler::new();
    GRAPH_SEARCH_HANDLERS.lock().unwrap().insert(session_id, handler);
}
```

## Dependencies

- Depends on KGRAPH-002 (database lifecycle must exist first)
- Does NOT depend on the indexing pipeline — the tool works on whatever
  data is in the graph (empty graph returns empty results)
