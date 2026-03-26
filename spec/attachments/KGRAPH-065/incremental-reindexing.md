# KGRAPH-065: Live File Watching and Incremental Re-indexing

## Problem

Every call to `ast_index` walks the entire codebase, re-extracts everything, and batch-loads with Overwrite mode. On large codebases this takes seconds to minutes. During active development, the graph becomes stale between re-indexes. Agents work with outdated call graphs and dead code information.

## CGC Reference Implementation

### Architecture — `core/watcher.py` (entire file, 203 lines)

CGC uses `watchdog` (a Python filesystem event library) with a 3-layer architecture:

1. **CodeWatcher** (lines 142–203) — manages the Observer thread, tracks watched paths
2. **RepositoryEventHandler** (lines 18–139) — per-repo event handler with debouncing
3. **Observer** — watchdog's filesystem event loop (background thread)

### Initial scan — `watcher.py` lines 50–68

```python
def _initial_scan(self):
    """Scans the entire repository, parses all files, builds initial graph."""
    supported_extensions = self.graph_builder.parsers.keys()
    all_files = [f for f in self.repo_path.rglob("*") 
                 if f.is_file() and f.suffix in supported_extensions]
    
    # 1. Pre-scan all files for global symbol map
    self.imports_map = self.graph_builder._pre_scan_for_imports(all_files)
    
    # 2. Parse all files in detail, cache parsed data
    for f in all_files:
        parsed_data = self.graph_builder.parse_file(self.repo_path, f)
        if "error" not in parsed_data:
            self.all_file_data.append(parsed_data)
    
    # 3. Create cross-file relationships
    self.graph_builder._create_all_function_calls(self.all_file_data, self.imports_map)
    self.graph_builder._create_all_inheritance_links(self.all_file_data, self.imports_map)
```

### Debouncing — `watcher.py` lines 70–82

```python
def _debounce(self, event_path, action):
    """Wait for quiet period before processing — prevents firing on every IDE save."""
    if event_path in self.timers:
        self.timers[event_path].cancel()
    timer = threading.Timer(self.debounce_interval, action)
    timer.start()
    self.timers[event_path] = timer
```

Default debounce interval: 2.0 seconds.

### Incremental update — `watcher.py` lines 84–119

```python
def _handle_modification(self, event_path_str):
    """Complete update cycle for a modified/created file."""
    # 1. Get all supported files
    all_files = [f for f in self.repo_path.rglob("*") if f.is_file() and ...]
    
    # 2. Re-scan ALL files for fresh global symbol map
    self.imports_map = self.graph_builder._pre_scan_for_imports(all_files)
    
    # 3. Update the SPECIFIC changed file in the graph
    self.graph_builder.update_file_in_graph(modified_path, self.repo_path, self.imports_map)
    
    # 4. Re-parse ALL files for complete in-memory representation
    self.all_file_data = []
    for f in all_files:
        parsed_data = self.graph_builder.parse_file(self.repo_path, f)
        ...
    
    # 5. CRITICAL: Re-link entire graph for calls and inheritance
    self.graph_builder._create_all_function_calls(self.all_file_data, self.imports_map)
    self.graph_builder._create_all_inheritance_links(self.all_file_data, self.imports_map)
```

**Note**: CGC's "incremental" update still re-scans all files for the symbol map and re-links the entire graph. Only the specific changed file's nodes are replaced. This is a known limitation — true incremental would track which files import the changed file and only re-link those.

### Event handlers — `watcher.py` lines 122–139

Handles: `on_created`, `on_modified`, `on_deleted`, `on_moved` — all debounced.

### MCP tools — `tool_definitions.py` lines 52–59

```python
"watch_directory": {
    "description": "Continuously monitor a directory for changes, auto-updating graph",
    "inputSchema": { "properties": { "path": {"type": "string"} } }
}
```

Also: `list_watched_paths`, `unwatch_directory`.

## What We Need to Implement

### Phase 1: Incremental Re-indexing (no watcher needed)

Before adding file watching, fix the fundamental issue: **ast_index always overwrites**.

1. **Track file modification times**: Store `mtime` per file in the graph
2. **On re-index**: Only re-extract files where `current_mtime > stored_mtime`
3. **Merge mode**: Use nanograph's Merge mode instead of Overwrite for unchanged files
4. **Delete stale**: Remove nodes for files that no longer exist

This alone makes re-indexing 10–100x faster for incremental changes.

### Phase 2: File Watching (optional, higher effort)

Add a filesystem watcher that triggers incremental re-index on file changes.

**In our Rust context, options:**
- `notify` crate (mature Rust filesystem watcher)
- Integrate with existing session lifecycle

```rust
use notify::{Watcher, RecursiveMode, watcher};
use std::sync::mpsc::channel;
use std::time::Duration;

let (tx, rx) = channel();
let mut watcher = watcher(tx, Duration::from_secs(2))?; // 2s debounce
watcher.watch(project_root, RecursiveMode::Recursive)?;

loop {
    match rx.recv() {
        Ok(event) => {
            // Re-extract only changed files
            // Merge into graph
        }
        Err(e) => break,
    }
}
```

### Cross-file re-linking problem

CGC's approach (re-link entire graph on every change) is expensive but correct. A smarter approach:

1. Track which files import the changed file (reverse dependency graph)
2. Only re-link the changed file + its importers
3. This requires maintaining an in-memory import dependency graph

### Our existing infrastructure

- `ast_pipeline::walk_and_extract()` already walks the filesystem — needs `mtime` filter
- `GraphDatabase::load_entities()` supports Merge mode — we just use Overwrite
- nanograph has `@key slug` for identity-based upsert — Merge mode matches on this

### Files to modify

| File | Change |
|------|--------|
| `codelet/napi/src/ast_pipeline/` | Add mtime tracking, incremental file filtering |
| `codelet/napi/src/graph/database.rs` | Use Merge mode for incremental updates |
| `codelet/napi/src/graph_search_handler.rs` | Support incremental vs full reindex flag |
| `codelet/tools/src/graph_search/types.rs` | Add `incremental` flag to AstIndex |

### Effort estimate

**High** — Phase 1 (incremental) is medium effort but requires careful testing. Phase 2 (file watching) adds a background thread/task, debouncing, and lifecycle management. The cross-file re-linking problem is the hardest part — a changed file's exports may affect any importer's call graph.
