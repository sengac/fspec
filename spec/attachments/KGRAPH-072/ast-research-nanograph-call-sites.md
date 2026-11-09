# KGRAPH-072 AST research: nanograph call sites in codelet-graph (HEAD 7fb2449d5 + KGRAPH-073)

## `self.db.snapshot()`
```
rust/graph/src/database.rs:244:23:self.db.snapshot()
rust/graph/src/bundle.rs:158:23:self.db.snapshot()

```

## `Database::init($$$A)`
```
rust/graph/src/database.rs:37:18:Database::init(db_path, schema_source)

```

## `Database::open($$$A)`
```
rust/graph/src/database.rs:51:18:Database::open(db_path)

```

## `self.db.load($$$A)`
```
rust/graph/src/database.rs:153:9:self.db

```

## `self.db.load_with_mode($$$A)`
```
rust/graph/src/database.rs:166:9:self.db

```

## `self.db.run_json($$$A)`
```
rust/graph/src/database.rs:196:22:self

```

## `$DB.export_all_entities()`
```
rust/graph/src/ast_index.rs:212:26:db.export_all_entities()
rust/graph/src/ast_pipeline/incremental.rs:53:20:db.export_all_entities()
rust/graph/src/bundle.rs:33:24:self.export_all_entities()

```

## `$DB.stats()`
```
rust/graph/src/dispatch_helpers.rs:15:11:db.stats()

```

## Conclusion
- snapshot() (removed in 1.3.0) is used only by GraphDatabase::stats (database.rs) and bundle.rs export_all_entities; both move to store::metadata::DatabaseMetadata / store::export::build_export_rows_at_path.
- export_all_entities becomes async (build_export_rows_at_path is async); callers: bundle export, ast_index incremental, incremental::read_stored_mtimes and their tests.
- init/open/load/load_with_mode/run_json keep their signatures in 1.3.0.
