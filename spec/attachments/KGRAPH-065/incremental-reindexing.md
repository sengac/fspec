# KGRAPH-065: Incremental Re-indexing

## Problem

Every call to `ast_index` walks the entire codebase, re-extracts everything, and batch-loads with Overwrite mode. On large codebases this takes seconds to minutes. During active development, the graph becomes stale between re-indexes. Agents work with outdated call graphs and dead code information.

## CGC Reference Implementation

CGC's `core/watcher.py` (203 lines) uses `watchdog` for filesystem monitoring with:
- **RepositoryEventHandler**: per-repo event handler with 2s debounce
- **CodeWatcher**: manages Observer thread, tracks watched paths
- On change: re-scans ALL files for import map, updates ONLY changed file's nodes, then re-links entire graph

**Key insight from CGC**: Even "incremental" updates re-scan all files for the import/symbol map and re-link all edges. Only node extraction is skipped for unchanged files.

## Our Implementation: Phase 1 — Incremental Re-indexing (No Watcher)

### Architecture

**Strategy: mtime-filtered extraction + full overwrite-load**

1. **Store mtime on File nodes**: Populate the existing `lastModified: DateTime?` schema property via post-processing in `walk_and_extract` (no extractor signature changes)
2. **Compare mtimes**: Read stored File node mtimes from graph, compare with filesystem
3. **Selective extraction**: Only call `extract_file()` for changed/new files
4. **Reuse unchanged**: Export existing graph entities, keep those belonging to unchanged files
5. **Full overwrite**: Combine reused + fresh entities, deduplicate, overwrite-load

### Algorithm

```
incremental_index(project_root, db):
  // 1. Walk filesystem — always needed for full file list
  source_files = walk_source_files(project_root)
  current_mtimes = {rel_path → mtime for f in source_files}

  // 2. Read stored mtimes from graph (via all_files_with_mtime query)
  stored_mtimes = read_stored_mtimes(db)

  // 3. Partition files
  changed = files where mtime differs or file is new
  deleted = files in stored but not in current
  unchanged = files where mtime matches

  // 4. Decide strategy
  if stored_mtimes.is_empty() or changed.len() > 50% of total:
    fall_back_to_full_index()

  // 5. Extract only changed/new files
  fresh_entities = extract_files(changed, project_root, known_files)
  stamp_file_mtimes(fresh_entities, current_mtimes)

  // 6. Export unchanged from graph
  existing = db.export_all_entities()
  changed_slugs = slugs for changed + deleted files
  reused = existing.filter(|e| !belongs_to_changed_file(e, changed_slugs))

  // 7. Re-extract dependencies (always)
  dep_entities = extract_all_dependencies(project_root)

  // 8. Combine → deduplicate → overwrite
  all = reused + fresh_entities + dep_entities
  all = deduplicate_entities(all)  // prunes dangling edges
  db.load_entities_overwrite(all)
```

### Entity Ownership (slug-prefix based)

| Entity Type | Owned by file if... |
|-------------|---------------------|
| File node   | slug == file_slug |
| Function    | slug starts with `file_slug::` |
| Type        | slug starts with `file_slug::` |
| Variable    | slug starts with `file_slug::` |
| Dependency  | slug starts with `dep::` — always re-extracted |
| Edges       | from_slug starts with file_slug (or equals it for File→X edges) |

### Files to Modify

| File | Change |
|------|--------|
| `ast_pipeline/incremental.rs` | **NEW** — collect_file_mtimes, read_stored_mtimes, partition_changed_files, filter_reusable_entities |
| `ast_pipeline/mod.rs` | Extract walk_source_files() from walk_and_extract; add mtime stamping post-process; export incremental module |
| `ast_pipeline/helpers.rs` | No change (mtime stamped in post-process, not in build_file_node) |
| `ast_index.rs` | Incremental dispatch logic: mtime compare → selective extract → combine → overwrite |
| `schemas/ast-queries.gq` | Add all_files_with_mtime query returning slug + lastModified |
| `codelet/tools/src/graph_search/types.rs` | Add `incremental: Option<bool>` to AstIndex variant |
| `codelet/tools/src/graph_search/mod.rs` | Update tool definition docs for incremental param |
| `graph_search_handler.rs` | Pass incremental flag to dispatch_ast_index |
| `database.rs` | Make export_all_entities() pub (currently private) |

### Test Plan

6 scenarios covering:
1. Full index stores mtime on File nodes
2. Incremental with no changes → 0 re-extracted
3. Incremental with modified file → only that file re-extracted
4. Incremental with deleted file → entities removed
5. Incremental with new file → entities added
6. Incremental on empty graph → falls back to full
