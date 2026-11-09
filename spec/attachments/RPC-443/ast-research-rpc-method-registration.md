# AST research: RPC method registration (RPC-443)

## `async fn get_work_unit_details(self, _ctx: Context, id: String) -> Result<String, String> { $$$B }`
```
No matches found
```

## `async fn get_workspace_info(self, _ctx: Context) -> WorkspaceInfo { $$$B }`
```
No matches found
```

MCP registration points: mcp_dispatch/{group_NN.rs, catalog.rs, dispatch.rs, validate.rs}; rpc/tests/mcp_coverage.rs enforces one mapping per trait method.
