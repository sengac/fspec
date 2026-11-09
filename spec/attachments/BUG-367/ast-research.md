# BUG-367 AST research

## fspec astgrep --pattern 'serde_json::to_value(schemars::schema_for!($A)).unwrap_or_default()' --lang rust --path rust/rpc/src/mcp_dispatch | wc -l
     132

Output lines, one or more per match: every group module's schema() uses it.
