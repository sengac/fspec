# KGRAPH-077 AST research: query call sites in the code graph query layer

## fspec astgrep --pattern 'db.query_with_source($$$ARGS)' --lang rust --path rust/graph/src
```
rust/graph/src/ast_hierarchy.rs:99:21
rust/graph/src/ast_hierarchy.rs:138:31
rust/graph/src/ast_hierarchy.rs:179:11
rust/graph/src/ast_hierarchy.rs:192:11
rust/graph/src/ast_dead_code.rs:67:15
rust/graph/src/ast_dead_code.rs:228:37
rust/graph/src/ast_dead_code.rs:242:38
rust/graph/src/ast_call_chain/mod.rs:108:44
rust/graph/src/ast_call_chain/snapshot.rs:31:42
rust/graph/src/ast_call_chain/snapshot.rs:44:40
rust/graph/src/learnings_dispatch.rs:51:15
rust/graph/src/learnings_dispatch.rs:106:11
rust/graph/src/learnings_dispatch.rs:171:11
rust/graph/src/learnings_dispatch.rs:196:50
rust/graph/src/ast_complexity.rs:54:22
rust/graph/src/ast_complexity.rs:121:22
rust/graph/src/ast_complexity.rs:215:47
rust/graph/src/learnings_context.rs:74:11
rust/graph/src/learnings_context.rs:89:11
rust/graph/src/learnings_context.rs:111:11
rust/graph/src/ast_dispatch.rs:129:38
rust/graph/src/ast_dispatch.rs:224:15
rust/graph/src/ast_dispatch.rs:303:15
```

## fspec astgrep --pattern 'build_adjacency_list($$$ARGS)' --lang rust --path rust/graph/src
```
rust/graph/src/ast_call_chain/mod.rs:63:15
rust/graph/src/ast_transitive.rs:52:23
rust/graph/src/ast_transitive.rs:99:23
```

## fspec astgrep --pattern 'GraphSnapshot::load($$$ARGS)' --lang rust --path rust/graph/src
```
rust/graph/src/ast_transitive.rs:42:20
rust/graph/src/ast_transitive.rs:89:20
```

## fspec astgrep --pattern 'resolve_file_path($$$ARGS)' --lang rust --path rust/graph/src
```
rust/graph/src/ast_complexity.rs:91:16
rust/graph/src/ast_complexity.rs:154:20
```

