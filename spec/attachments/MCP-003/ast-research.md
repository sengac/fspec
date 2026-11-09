# MCP-003 AST research

## fspec astgrep --pattern 'FspecServiceImpl::new($A)' --lang rust --path rust/rpc-server/src
rust/rpc-server/src/bounded_reads.rs:26:18:FspecServiceImpl::new(service)
rust/rpc-server/src/server.rs:217:24:FspecServiceImpl::new(Arc::clone(&service))


## fspec astgrep --pattern 'FspecServiceImpl::new($A)' --lang rust --path rust/rpc-embedded/src
rust/rpc-embedded/src/lib.rs:71:28:FspecServiceImpl::new(Arc::clone(&self.service))


## fspec astgrep --pattern '$S.authorizes($A)' --lang rust --path rust/rpc-server/src
rust/rpc-server/src/server.rs:152:17:service.authorizes(authorization)


## fspec astgrep --pattern 'init_mcp_session($A)' --lang rust --path rust/sessions/src
No matches found

## fspec astgrep --pattern 'cleanup_mcp_session($A)' --lang rust --path rust/sessions/src
No matches found

## fspec astgrep --pattern 'gather_mcp_tool_wrappers($A)' --lang rust --path rust/agent-loop/src
No matches found

## grep (call sites inside include!d / macro fragments that ast-grep does not parse)

- rust/sessions/src/session_creation_helper.rs:279 `codelet_tools::init_mcp_session(uuid)` — every session gets an MCP registry entry at creation.
- rust/sessions/src/session_manager/manager_impl_6.rs:79 `codelet_tools::cleanup_mcp_session(uuid)` — destroy cancels and removes it.
- rust/agent-loop/src/dispatch.rs:61, agent_loop/provider_openai.rs:19, provider_custom.rs:34/85 `codelet_tools::gather_mcp_tool_wrappers(session.id)` — the agent build reads the session's connections, so servers configured before a prompt are visible to it.

## Findings

- Three FspecServiceImpl construction sites: two in rpc-server after the RPC-447 handshake check (server.rs:217 normal RPC, bounded_reads.rs:26 the /fspec-read-v2 path) and one in-process (rpc-embedded lib.rs:71). Authentication state must be decided at those sites.
