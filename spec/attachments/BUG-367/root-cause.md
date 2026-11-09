# BUG-367 root cause

`rust/rpc/src/mcp_dispatch/catalog.rs` builds the MCP method catalog from each group module's `schema()`. Every one of them is `serde_json::to_value(schemars::schema_for!(Args)).unwrap_or_default()`, so a schema that fails to serialize silently becomes `Value::Null`. The resulting tool has no input schema and nothing reports why. `group_27.rs` (write_exec_stdin) also contains a no-op `let _ = &args;`. TOOL-027's new group_43 follows the same pattern so the catalog stays uniform. This card fixes all of them together.

Fix direction: make `methods()` fallible (or build the catalog once with an error naming the failing method) and remove the `unwrap_or_default`. `rpc/tests/mcp_coverage.rs` already asserts each schema is an object, so it covers the result.

Duplicate note (2026-11-03): BUG-413 (MCP dispatch schema() falls back to an empty schema on serialization failure) covers the same unwrap_or_default; merge the two when either is picked up and close one as the duplicate.
