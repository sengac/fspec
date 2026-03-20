# AST Research: GraphSearch Query Dependencies

## Existing graph module functions

### extractors.rs
- `extract_from_file_operation()` — structural extractor
- `extract_from_fspec_command()` — structural extractor
- `BatchQueue::new/push/flush` — batching

### merge.rs
- `entities_to_jsonl()` — JSONL conversion
- `calculate_strength()` — strength formula
- `merge_entities()` — merge logic
- `read_index_state() / write_index_state()` — watermark

### llm_extraction.rs
- `filter_extractable_turns()` — turn filtering
- `build_extraction_prompt()` — prompt building
- `parse_and_validate_response()` — LLM response parsing

### mod.rs (database lifecycle)
- `ensure_graph_db()` — lazy init singleton
- `is_graph_initialized()` — check state
- `reset_graph_db()` — reset singleton
- `graph_db_stats()` — stats from catalog
- `graph_describe_schema()` — schema description
- `close_graph_db()` — cleanup

## Nanograph Query API
- `db.run(query_source, query_name, params)` → RunResult
- `db.run_json(query_source, query_name, params, mode)` → RunResult
- `ParamMap` = HashMap<String, Literal>
- `Literal` variants: String, I32, I64, F32, F64, Bool, DateTime, Null
- `RunResult::Query(QueryResult)` contains Arrow RecordBatch
- `QueryResult` → `.to_json()` for JSON output

## Query result format
QueryResult has methods to convert to JSON. The `run_json` method returns RunResult which can be converted.
