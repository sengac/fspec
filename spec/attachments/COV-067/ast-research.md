# AST research: dynamic-mcp-connect test locations

- `grep -n '^// Scenario:' rust/tools/src/mcp/tests_0*.rs` lists the scenario banners: `tests_01.rs` holds stdio connect (90), route (156), spawn failure (197) and HTTP connect (239). The rest are in `tests_02.rs`, `tests_03.rs` and `tests_04.rs`.
- `fspec astgrep --pattern 'async fn $NAME() { $$$B }' --lang rust --path rust/tools/src/mcp` gives each test function's line span to use as `--test-lines`.
- The implementation targets are already named in the current mappings: `stdio_and_routing.rs`, `http_and_session_registry.rs`, `client_handler.rs` and `types.rs`. Check each range against the current code before relinking.
