# TOOL-027 AST research

## fspec astgrep --pattern '$C.write_exec_stdin($$$A)' --lang rust --path rust/fspec-tui/src
rust/fspec-tui/src/app/dispatch_exec_stdin.rs:113:19 (backend.write_exec_stdin, the TUI reducer)

## fspec astgrep --pattern '$C.write_exec_stdin($$$A)' --lang rust --path rust/rpc/src
rust/rpc/src/mcp_dispatch/group_27.rs:75:9 (client.write_exec_stdin, MCP dispatch)

## Layers to mirror (grep, non-test): write_exec_stdin and the last appended method configure_session_mcp_servers
- core/src/session_manager_handle/handle_trait_3.rs:126 (write, default Ok), handle_trait_4.rs:186 (configure, default refuses). The new method goes in handle_trait_4.rs with a refusing default.
- sessions/src/handle_impl/handle_impl_4.rs:67 (write: loop_block_on plus global_store().get_stdin_tx, clears the request on success); handle_impl_5.rs:185 (configure).
- rpc/src/service_declaration/session_mcp.rs ("new methods are appended here") and service_implementation/session_mcp.rs.
- rpc/src/mcp_dispatch: catalog.rs:133, dispatch.rs:157, validate.rs:133, group_42.rs. rpc/tests/mcp_coverage.rs requires every trait method to have an MCP mapping.
- rpc/tests/fixtures/wire_order_frozen.txt: append the name. rpc/tests/rpc449_wire_variant_order.rs reads the fragments.
- rpc-server/tests/mcp003/lifecycle.rs asserts configure_session_mcp_servers is LAST. Appending breaks it, so its scenario becomes "keeps index 129", the same form get-default-model-rpc.feature uses.
- FspecBackend: fspec-tui/src/transport/backend/subscriptions.rs:167, embedded/subscriptions.rs:160, websocket/scheduling_loops.rs:178. Mocks: tests/common/mock/mock_backend_1.rs:97, common/model_backend.rs:74, common/continue_backend.rs:84, tests/prov112/transport.rs:46, tests/app_dispatch_history_rpc025.rs:118.

## unified_exec internals
- ProcessEntry (process_store.rs) has pub `stdin_tx: mpsc::Sender<Vec<u8>>`, `tty: bool` and `child: Child`. The pipe forwarder (spawning.rs:289) loops `while let Some(data) = stdin_rx.recv()`, so dropping every sender ends it and drops the child's stdin (EOF). The only long-lived sender is the store's. get_stdin_tx clones are per write.
- Pipe spawn uses `process_group(0)`, so the child pid is the process-group id (spawning.rs build_kill_handle comment).
- PTY stdin uses the same mpsc and is written to the PTY master. 0x03 and 0x04 are handled by the slave's line discipline (ISIG / ICANON VEOF).
- spawning.rs (593) and tool.rs (529) are over 300 lines and are NOT touched. The EOF swap and the SIGINT live in a new stdin_control.rs.
