# KGRAPH-078 AST research: hand-written graphs in query tests, and the write record

## fspec astgrep --pattern 'db.load_jsonl($$$A)' --lang rust --path rust/graph/tests
```
rust/graph/tests/ast_extraction_pipeline_test/walk_project_directory_with.rs:86:5
rust/graph/tests/ast_query_interface_test.rs:42:5
rust/graph/tests/ast_export_import_test.rs:36:5
rust/graph/tests/bug626_learnings_context_multi_byte.rs:48:5
rust/graph/tests/learnings_graph_data_model_test/initialize_learnings_graph_database.rs:109:5
rust/graph/tests/learnings_graph_data_model_test/initialize_learnings_graph_database.rs:196:5
rust/graph/tests/learnings_graph_data_model_test/initialize_learnings_graph_database.rs:206:5
rust/graph/tests/learnings_graph_data_model_test/initialize_learnings_graph_database.rs:212:5
rust/graph/tests/learnings_graph_data_model_test/initialize_learnings_graph_database.rs:218:5
rust/graph/tests/ast_transitive_test.rs:43:5
rust/graph/tests/ast_transitive_test.rs:70:5
rust/graph/tests/ast_graph_data_model_test/initialize_ast_graph_database.rs:100:5
rust/graph/tests/ast_graph_data_model_test/initialize_ast_graph_database.rs:205:5
rust/graph/tests/ast_graph_data_model_test/initialize_ast_graph_database.rs:212:5
rust/graph/tests/ast_graph_data_model_test/initialize_ast_graph_database.rs:219:5
rust/graph/tests/ast_graph_data_model_test/reusable_graph_database_supports.rs:118:9
rust/graph/tests/ast_call_chain_test.rs:41:5
rust/graph/tests/ast_call_chain_test.rs:69:5
rust/graph/tests/learnings_query_interface_test.rs:39:5
rust/graph/tests/ast_search_filter_test.rs:59:5
rust/graph/tests/cross_session_learning_test.rs:40:5
rust/graph/tests/cross_session_learning_test.rs:85:5
rust/graph/tests/cross_session_learning_test.rs:152:5
rust/graph/tests/cross_session_learning_test.rs:261:5
rust/graph/tests/bug643_overwrite_replaces_whole_graph.rs:212:5
rust/graph/tests/kgraph072_nanograph_1_3.rs:349:5
rust/graph/tests/kgraph072_nanograph_1_3.rs:350:5
rust/graph/tests/kgraph072_nanograph_1_3.rs:358:5:db.load_jsonl(r#"{"edge":"Supersedes","from":"registry-pattern","to":"old-pattern","data":{"supersededAt":"2026-03-22T00:00:00
rust/graph/tests/ast_variable_tracking_test.rs:49:5
```

## fspec astgrep --pattern 'write_index_record($$$A)' --lang rust --path rust/graph/src
```
```

## fspec astgrep --pattern 'fn build_structured_chains($$$A) $R { $$$B }' --lang rust --path rust/graph/src
```
```

