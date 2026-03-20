# KGRAPH-009: DeepSearch Graph Integration — Notes

## Three Integration Points

### 1. GraphSearch as DeepSearch Sub-Agent Tool

When the graph database exists (`~/.fspec/graph/agent-memory.nano/`),
add `GraphSearchTool` to the DeepSearch sub-agent's tool set:

```rust
// deep_search_handler.rs — build_tools()
let mut tools = vec![
    read_tool, grep_tool, ast_grep_tool, glob_tool,
    ls_tool, bash_tool, session_search_tool,
];

if graph::is_graph_initialized() {
    tools.push(graph_search_tool);
}
```

This gives the sub-agent 8 tools instead of 7. The sub-agent can use
GraphSearch to find related concepts before diving into code files.

### 2. `update_graph` Flag

New optional parameter on DeepSearchArgs:

```rust
pub struct DeepSearchArgs {
    pub query: String,
    pub scope: Option<Vec<String>>,
    pub max_depth: Option<usize>,
    pub max_recursion_depth: Option<usize>,
    pub update_graph: Option<bool>,  // NEW
}
```

When `update_graph=true`, after the sub-agent finishes:
1. Collect all sessions the sub-agent accessed via SessionSearch
2. Queue those sessions for graph indexing
3. Run a lightweight extraction pass on the sub-agent's own output
   (its synthesized answer likely contains useful concept connections)

### 3. Graph Context in System Prompt

Before spawning the sub-agent, optionally query the graph for context
related to the query:

```rust
if graph::is_graph_initialized() {
    let related = graph::search_concepts(&query, 5)?;
    if !related.is_empty() {
        system_prompt.push_str(&format!(
            "\n\nKnowledge graph context — related concepts:\n{}",
            format_concepts(&related)
        ));
    }
}
```

This primes the sub-agent with relational context so it can make
smarter decisions about where to look.

## Backward Compatibility

All three integrations are **opt-in**:
- GraphSearch tool: only added if graph DB exists
- `update_graph`: defaults to false
- Graph context: only injected if graph DB exists and has data

DeepSearch works identically when no graph is present.
