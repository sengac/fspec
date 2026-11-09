# AST research for KGRAPH-076 (2026-11-08)

## pattern: `collect_file_mtimes($$$)` in rust/graph/src
```
rust/graph/src/ast_index.rs:70:30:collect_file_mtimes(&source_files, root)
rust/graph/src/ast_index.rs:94:18:collect_file_mtimes(&source_files, root)

```

## pattern: `read_stored_mtimes($$$)` in rust/graph/src
```
rust/graph/src/ast_index.rs:71:35:read_stored_mtimes(db)

```

## pattern: `stamp_file_mtimes($$$)` in rust/graph/src
```
rust/graph/src/ast_index.rs:95:5:stamp_file_mtimes(&mut entities, &mtimes)

```

## pattern: `partition_changed_files($$$)` in rust/graph/src
```
rust/graph/src/ast_index.rs:76:13:partition_changed_files(&current_mtimes, &stored_mtimes)

```

## pattern: `walk_source_files($$$)` in rust/graph/src
```
No matches found
```

## pattern: `export_all_entities()` in rust/graph/src
```
No matches found
```

## pattern: `extract_path($$$)` in rust/graph/src
```
No matches found
```

## pattern: `$A::walk_source_files($$$)` in rust/graph/src
```
rust/graph/src/ast_index.rs:69:28:super::ast_pipeline::walk_source_files(root, true)
rust/graph/src/ast_index.rs:93:24:super::ast_pipeline::walk_source_files(root, true)

```

## pattern: `$A.export_all_entities()` in rust/graph/src
```
rust/graph/src/ast_pipeline/incremental.rs:53:20:db.export_all_entities()
rust/graph/src/database.rs:182:26:self.export_all_entities()
rust/graph/src/bundle.rs:32:24:self.export_all_entities()

```

## pattern: `$A::extract_path($$$)` in rust/graph/src
```
rust/graph/src/ast_pipeline/facts/mod.rs:235:48:extract::extract_path(path, rel)

```

## pattern: `$A::extract_tree($$$)` in rust/graph/src
```
rust/graph/src/ast_pipeline/mod.rs:150:27:facts::extract_tree(root, &walked.sources, threads)

```

## pattern: `$A::index_tree($$$)` in rust/graph/src
```
rust/graph/src/ast_index.rs:91:9:super::ast_pipeline::index_tree(root, true, super::ast_pipeline::facts::index_threads())

```
