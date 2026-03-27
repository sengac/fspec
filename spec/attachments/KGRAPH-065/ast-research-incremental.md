# AST Research: Incremental Re-indexing Architecture

## Current ast_index Pipeline

`dispatch_ast_index(custom_path, reset)` has 5 steps:
1. Resolve project root (custom path or cwd)
2. Optional reset (delete on-disk db + clear in-memory singleton)
3. Get/create GraphDatabase via registry singleton
4. Extract: `walk_and_extract()` + `extract_all_dependencies()` + `deduplicate_entities()`
5. Batch load with `load_entities_overwrite()` (full replacement)

## walk_and_extract Two-Phase Design

**Phase 1**: Walk directory with `ignore::WalkBuilder`, collect source file paths, build `known_files` HashSet  
**Phase 2**: Call `extract_file()` per file (dispatches to 14 language extractors via extension matching), each wrapped in `catch_unwind`

Key: `known_files` set is needed for barrel-import resolution in TypeScript. Must be built from ALL source files, not just changed ones.

## Entity Deduplication

`deduplicate_entities()` handles:
- Node dedup: keyed by (node_type, slug), node with more properties wins
- Edge pruning: validates both endpoints exist AND match schema-expected types
- Dangling edges from renamed/removed targets are automatically pruned

## export_all_entities() (KGRAPH-069)

Private method on GraphDatabase, reads Arrow record batches directly:
- Phase 1: Export all nodes, build id→slug HashMap
- Phase 2: Export all edges, resolve numeric src/dst IDs to slugs via the map
- Supports: Utf8, LargeUtf8, Int32, UInt64, Boolean, Date64 types

**Current visibility: private** — needs to be made `pub` for incremental reuse.

## Schema: File Node

```
node File {
    slug: String @key
    path: String @unique
    language: String?
    lineCount: I32?
    lastModified: DateTime?  ← Already in schema, NOT populated
    isTest: Bool?
}
```

The `lastModified` field exists in schema but `build_file_node()` doesn't set it. The `all_files()` query doesn't return it.

## Incremental Strategy: mtime-filtered extraction + full overwrite

1. Walk filesystem → collect file paths + mtimes (always needed for `known_files`)
2. Read stored mtimes from graph via new query
3. Partition: changed/new/deleted/unchanged
4. Extract only changed/new files
5. Export unchanged entities from graph (reuse via `export_all_entities()`)
6. Combine, deduplicate (prunes dangling edges), overwrite-load

Key insight: extraction is O(file_size × ast-grep cost), export is O(entity_count × Arrow read). Skipping extraction for unchanged files is the main performance win.
