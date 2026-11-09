# TEST-042: wall-clock waits still in fspec's test code

Output of the TEST-042 check (rust/test-helpers/tests/test042_no_wall_clock_waits_in_tests.rs, the ignored workspace-scan scenario, run with --ignored) at the TEST-042 batch 9 working tree, 2026-11-07: **129 offences** (fspec 99, fspec-core 10, providers 9, tools 11).

The check reports wall-clock sleeps, fixed timeouts, timed-wait methods and (since the supervisor's 2026-11-07 scope addition) comparisons with a clock reading (`elapsed() comparison`, `std::time::Instant::now() comparison`, `tokio::time::Instant::now() comparison`). It resolves `use super::*` through the parent module file and `include!`d parts through the including file.

Clean crates: agent-loop, cli, common, core, fspec-tui, graph, rpc-embedded, rpc-server, sessions, test-helpers, tui.

Left:
- fspec (99): real daemon/client processes; settle sleeps after the port line can go (signal handlers are installed before the port is printed, rust/fspec/src/daemon.rs), negative 'still running' checks need a readiness signal (a log line or the first WebSocket frame). cross_frontend_parity has a source-shape test that requires tokio::time::timeout in the golden body and must change with it. Some files are held by the audit helper (cli_check, cli_audit_coverage, cli_bootstrap, cli_link_coverage, cli_reverse, cli_init, refac020/*, cargo_shape/*, src/*_tests.rs) until the audit cards are done.
- fspec-core (10): refac020_coverage_transactions (8, file-polling IPC, BUG-490 area, held) and dispatch_command_tokio_nesting (2 elapsed bounds).
- providers (9 clock comparisons): device-flow poll interval and backoff lower bounds (copilot_oauth_device_flow_test, device_protocol/legacy_poll, prov154_codex_recovery, prov160_codex_offline), an unreachable-server bound (openai_local_provider_test), a runtime-not-blocked bound (prov159_refresh_completion), a cancellation bound (prov160_codex_offline), and prov159_support/device_session's budget check. Each needs virtual time (paused clock where no real IO races the timers) or an injected clock.
- tools (11 clock comparisons): astgrep search-vs-parse ratio (bug478), rlcd001-004 bounded-wait claims, unified_exec yield-time lower bounds (the yield time is the feature itself), and bridge_relay's #[cfg(test)] 2 s read loop.

Weaker 'nothing happens' checks (a fixed number of scheduler rounds or a single drain instead of a time window): agent-loop await_idle_completion_races, bug241_metadata_presentation, cov057_esc_interrupt, rpc088_interrupt_cascade; cli prov161_physical_runtime; providers bug266_custom_stream_settlement, codex_oauth_server_test, prov151_codex_dispatch, prov154_codex_deadline, prov154_codex_recovery, prov160_codex_reconnection, prov161_scripted_dispatch; sessions paused_chunk_delivery_rpc409; tools bridge_test_fixtures; fspec-tui rpc434/rpc437 (closed-receiver subscribers), app_dispatch_reorder_rpc017, slash_clear_rpc046, slash_compact_rpc047, slash_debug_rpc055, debug_parity_rpc430, bug181_live_checkpoint_counts (degraded transport), and the settle/drain helpers of tui114/115/116/119/120/121/122/123, rig016, rpc445/448/455, board_open_attachment_rpc374 and wt009 (all moved to the current-thread runtime, where a yield runs every ready task, so the result does not depend on load).

Dropped wall-clock bounds whose fact is shown otherwise: core bug259 (WouldBlock is the deadline's own outcome), common bug477 reentrancy (a non-reentrant acquire would wait out the lock timeout and fail), sessions cov057 (returning at all), fspec-tui rpc025 (dispatch returns while the history latch is never released), rpc-server mcp006/mcp008 (the error names the timeout), tools bash abort (the interrupted result and the missing "completed" line). mcp008 no longer pins that the three silent servers time out in parallel (MCP-010's join_all); that needs a paused-clock test in the tools crate.

Found and fixed on the way: agent-loop bug250 ownership race (BUG-571, a lost 200 ms head start; now ordered by the recovery lease itself); fspec-tui rpc415 no longer waits for every subscriber to end (the workspace subscribers are bound to mock channels that disconnect_all does not drop) and proves "no live stream" by the new sender having no receiver. Known flake, carded by the supervisor: rpc-server ws_log_event (about 1 in 12 runs never sees "hello"; the log layer broadcasts every event, so forwarding feeds itself and the client channel lags).

## By call
```
     34 tokio::time::sleep
     24 std::time::Instant::now() comparison
     23 tokio::time::timeout
     23 std::thread::sleep
     20 elapsed() comparison
      5 tokio::time::Instant::now() comparison
```

## Every offence (file:line: call, paths relative to rust/)
```
fspec/tests/bug239_review_privacy.rs:73: tokio::time::sleep
fspec/tests/bug239_review_privacy.rs:127: tokio::time::sleep
fspec/tests/bug539_corrupt_board_daemon_startup.rs:53: tokio::time::timeout
fspec/tests/client_mcp_full.rs:226: tokio::time::timeout
fspec/tests/client_mode.rs:103: std::time::Instant::now() comparison
fspec/tests/client_mode.rs:104: std::thread::sleep
fspec/tests/client_mode.rs:140: tokio::time::sleep
fspec/tests/client_mode.rs:194: std::time::Instant::now() comparison
fspec/tests/client_mode.rs:195: std::thread::sleep
fspec/tests/client_mode.rs:223: tokio::time::sleep
fspec/tests/client_mode.rs:279: tokio::time::sleep
fspec/tests/client_mode.rs:321: tokio::time::sleep
fspec/tests/client_mode.rs:355: tokio::time::sleep
fspec/tests/client_mode.rs:485: tokio::time::sleep
fspec/tests/client_mode.rs:493: tokio::time::sleep
fspec/tests/client_mode.rs:527: tokio::time::sleep
fspec/tests/client_mode.rs:531: tokio::time::sleep
fspec/tests/client_mode.rs:546: std::time::Instant::now() comparison
fspec/tests/client_mode.rs:552: std::thread::sleep
fspec/tests/client_mode.rs:580: tokio::time::sleep
fspec/tests/client_mode.rs:584: tokio::time::sleep
fspec/tests/client_mode.rs:607: tokio::time::sleep
fspec/tests/client_mode.rs:671: tokio::time::sleep
fspec/tests/client_mode.rs:676: tokio::time::sleep
fspec/tests/client_mode.rs:731: tokio::time::sleep
fspec/tests/client_mode.rs:771: std::thread::sleep
fspec/tests/combined_smoke.rs:93: tokio::time::timeout
fspec/tests/combined_smoke.rs:127: tokio::time::sleep
fspec/tests/combined_smoke.rs:166: tokio::time::sleep
fspec/tests/combined_smoke.rs:277: std::time::Instant::now() comparison
fspec/tests/combined_smoke.rs:278: std::thread::sleep
fspec/tests/combined_smoke.rs:289: std::time::Instant::now() comparison
fspec/tests/combined_smoke.rs:294: std::thread::sleep
fspec/tests/combined_smoke.rs:308: tokio::time::timeout
fspec/tests/combined_smoke.rs:459: std::time::Instant::now() comparison
fspec/tests/combined_smoke.rs:460: std::thread::sleep
fspec/tests/combined_smoke.rs:510: std::time::Instant::now() comparison
fspec/tests/combined_smoke.rs:515: std::thread::sleep
fspec/tests/common/mod.rs:139: std::time::Instant::now() comparison
fspec/tests/cross_frontend_parity.rs:187: tokio::time::Instant::now() comparison
fspec/tests/cross_frontend_parity.rs:188: tokio::time::timeout
fspec/tests/cross_frontend_parity.rs:484: tokio::time::Instant::now() comparison
fspec/tests/cross_frontend_parity.rs:485: tokio::time::timeout
fspec/tests/cross_frontend_parity.rs:512: tokio::time::timeout
fspec/tests/cross_frontend_parity.rs:528: tokio::time::Instant::now() comparison
fspec/tests/cross_frontend_parity.rs:529: tokio::time::timeout
fspec/tests/cross_frontend_parity.rs:745: tokio::time::Instant::now() comparison
fspec/tests/cross_frontend_parity.rs:746: tokio::time::timeout
fspec/tests/daemon_lifecycle_rpc011.rs:56: tokio::time::sleep
fspec/tests/daemon_lifecycle_rpc011.rs:70: std::time::Instant::now() comparison
fspec/tests/daemon_lifecycle_rpc011.rs:71: tokio::time::timeout
fspec/tests/daemon_lifecycle_rpc011.rs:131: tokio::time::sleep
fspec/tests/daemon_lifecycle_rpc011.rs:141: tokio::time::sleep
fspec/tests/daemon_lifecycle_rpc011.rs:178: tokio::time::sleep
fspec/tests/daemon_lifecycle_rpc011.rs:224: tokio::time::sleep
fspec/tests/daemon_lifecycle_rpc011.rs:265: tokio::time::sleep
fspec/tests/daemon_lifecycle_rpc011.rs:289: tokio::time::sleep
fspec/tests/daemon_lifecycle_rpc011.rs:335: tokio::time::sleep
fspec/tests/daemon_lifecycle_rpc011.rs:420: tokio::time::sleep
fspec/tests/daemon_lifecycle_rpc011.rs:441: tokio::time::sleep
fspec/tests/daemon_lifecycle_rpc011.rs:482: std::time::Instant::now() comparison
fspec/tests/daemon_lifecycle_rpc011.rs:485: std::thread::sleep
fspec/tests/daemon_mode.rs:152: tokio::time::timeout
fspec/tests/daemon_mode.rs:199: tokio::time::timeout
fspec/tests/daemon_mode.rs:236: tokio::time::timeout
fspec/tests/daemon_mode.rs:269: std::time::Instant::now() comparison
fspec/tests/daemon_mode.rs:275: std::thread::sleep
fspec/tests/daemon_mode.rs:315: std::time::Instant::now() comparison
fspec/tests/daemon_mode.rs:321: std::thread::sleep
fspec/tests/daemon_mode.rs:375: std::time::Instant::now() comparison
fspec/tests/daemon_mode.rs:376: std::thread::sleep
fspec/tests/daemon_mode.rs:417: std::time::Instant::now() comparison
fspec/tests/daemon_mode.rs:422: std::thread::sleep
fspec/tests/daemon_mode.rs:473: std::time::Instant::now() comparison
fspec/tests/daemon_mode.rs:474: std::thread::sleep
fspec/tests/daemon_mode.rs:512: std::time::Instant::now() comparison
fspec/tests/daemon_mode.rs:517: std::thread::sleep
fspec/tests/rpc070_create_session_no_panic.rs:68: tokio::time::timeout
fspec/tests/rpc070_create_session_no_panic.rs:128: tokio::time::timeout
fspec/tests/rpc073_clear_history_no_panic.rs:68: tokio::time::timeout
fspec/tests/rpc073_clear_history_no_panic.rs:78: tokio::time::timeout
fspec/tests/rpc073_clear_history_no_panic.rs:122: tokio::time::timeout
fspec/tests/rpc073_clear_history_no_panic.rs:135: tokio::time::timeout
fspec/tests/rpc073_clear_history_no_panic.rs:261: tokio::time::timeout
fspec/tests/rpc073_clear_history_no_panic.rs:278: tokio::time::timeout
fspec/tests/rpc446_per_daemon_discovery.rs:136: std::time::Instant::now() comparison
fspec/tests/rpc446_per_daemon_discovery.rs:138: std::thread::sleep
fspec/tests/rpc446_per_daemon_discovery.rs:178: std::time::Instant::now() comparison
fspec/tests/rpc446_per_daemon_discovery.rs:179: std::thread::sleep
fspec/tests/rpc456_remote_daemon_over_tls.rs:61: tokio::time::timeout
fspec/tests/scrollback_pty_rpc078.rs:105: std::time::Instant::now() comparison
fspec/tests/scrollback_pty_rpc078.rs:110: std::thread::sleep
fspec/tests/scrollback_pty_rpc078.rs:205: std::thread::sleep
fspec/tests/stale_daemon_json_rpc011.rs:128: tokio::time::sleep
fspec/tests/status_subcommand_rpc011.rs:78: tokio::time::sleep
fspec/tests/status_subcommand_rpc011.rs:88: tokio::time::sleep
fspec/tests/status_subcommand_rpc011.rs:256: tokio::time::sleep
fspec/tests/support/bug239_logging.rs:108: tokio::time::sleep
fspec/tests/support/bug239_provider_failure.rs:52: tokio::time::timeout
fspec-core/tests/dispatch_command_tokio_nesting.rs:120: elapsed() comparison
fspec-core/tests/dispatch_command_tokio_nesting.rs:155: elapsed() comparison
fspec-core/tests/refac020_coverage_transactions/process.rs:51: std::time::Instant::now() comparison
fspec-core/tests/refac020_coverage_transactions/process.rs:54: std::thread::sleep
fspec-core/tests/refac020_coverage_transactions/process.rs:75: std::time::Instant::now() comparison
fspec-core/tests/refac020_coverage_transactions/process.rs:78: std::thread::sleep
fspec-core/tests/refac020_coverage_transactions/process.rs:92: std::time::Instant::now() comparison
fspec-core/tests/refac020_coverage_transactions/process.rs:95: std::thread::sleep
fspec-core/tests/refac020_coverage_transactions/worker.rs:18: std::time::Instant::now() comparison
fspec-core/tests/refac020_coverage_transactions/worker.rs:21: std::thread::sleep
providers/tests/copilot_oauth_device_flow_test.rs:299: elapsed() comparison
providers/tests/copilot_oauth_device_flow_test.rs:380: elapsed() comparison
providers/tests/device_protocol/legacy_poll.rs:65: elapsed() comparison
providers/tests/openai_local_provider_test.rs:302: elapsed() comparison
providers/tests/prov154_codex_recovery.rs:106: elapsed() comparison
providers/tests/prov159_refresh_completion.rs:114: elapsed() comparison
providers/tests/prov159_support/device_session.rs:41: tokio::time::Instant::now() comparison
providers/tests/prov160_codex_offline.rs:81: elapsed() comparison
providers/tests/prov160_codex_offline.rs:194: elapsed() comparison
tools/src/bridge_relay.rs:1662: std::time::Instant::now() comparison
tools/tests/bug478_astgrep_on_the_workspace_root.rs:108: elapsed() comparison
tools/tests/rlcd001_service_supervisor.rs:644: elapsed() comparison
tools/tests/rlcd001_service_supervisor.rs:744: elapsed() comparison
tools/tests/rlcd002_decision_tool.rs:490: elapsed() comparison
tools/tests/rlcd002_decision_tool.rs:544: elapsed() comparison
tools/tests/rlcd003_workflow_gate.rs:535: elapsed() comparison
tools/tests/rlcd004_security_layer.rs:656: elapsed() comparison
tools/tests/unified_exec_tool_test.rs:143: elapsed() comparison
tools/tests/unified_exec_tool_test.rs:148: elapsed() comparison
tools/tests/unified_exec_tool_test.rs:186: elapsed() comparison
```
