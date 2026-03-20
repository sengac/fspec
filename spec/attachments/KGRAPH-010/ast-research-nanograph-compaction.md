# AST Research: Nanograph Compact/Migrate/Delete API

## Nanograph Database Methods (vendor/nanograph/crates/nanograph/src/store/database/persist.rs)

### Compact
- `db.compact(options?)` — Lance storage compaction
- `db.cleanup(options?)` — Remove old Lance fragments

### Migrate
- `db.migrate(new_schema_source, options?)` — Schema evolution
- Safety check: nanograph has built-in schema diff/safety levels

### Delete via Mutation Queries
- Mutations use `delete` keyword in nanograph query language
- `db.run_query(mutation_query, params)` for delete operations

### Schema IR Hash
- Schema compiled to `schema.ir.json` on disk
- Can compare hash of bundled schema vs on-disk for change detection
- Available via `db.schema_ir()` method

## Existing graph module (mod.rs)
- `ensure_graph_db()` checks `schema.ir.json` exists on open
- Schema migration point: between `db_path.exists()` check and `Database::open()`
