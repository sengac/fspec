# AST research: BUG-627 daemon spawns in the fspec tests

## fspec astgrep --pattern 'spawn_fspec_daemon($$$)' --lang rust --path rust
```
rust/fspec/tests/daemon_mode.rs:36:26:spawn_fspec_daemon(ws.path())
rust/fspec/tests/daemon_mode.rs:147:26:spawn_fspec_daemon(ws.path())
rust/fspec/tests/daemon_mode.rs:247:30:spawn_fspec_daemon(ws.path())
rust/fspec/tests/daemon_mode.rs:297:30:spawn_fspec_daemon(ws.path())
rust/fspec/tests/client_mode.rs:239:33:spawn_fspec_daemon(ws.path())
rust/fspec/tests/client_mode.rs:301:33:spawn_fspec_daemon(ws.path())
rust/fspec/tests/client_mode.rs:339:33:spawn_fspec_daemon(ws.path())
rust/fspec/tests/client_mode.rs:470:32:spawn_fspec_daemon(ws.path())
rust/fspec/tests/client_mode.rs:517:32:spawn_fspec_daemon(ws.path())
rust/fspec/tests/client_mode.rs:570:32:spawn_fspec_daemon(ws.path())
rust/fspec/tests/cargo_shape/fspec_version_prints_the.rs:199:40:spawn_fspec_daemon(w.path())
rust/fspec/tests/daemon_lifecycle_rpc011.rs:43:31:spawn_fspec_daemon(workspace)
rust/fspec/tests/daemon_lifecycle_rpc011.rs:127:35:spawn_fspec_daemon(workspace)
rust/fspec/tests/daemon_lifecycle_rpc011.rs:176:32:spawn_fspec_daemon(workspace)
rust/fspec/tests/cross_frontend_parity.rs:71:26:spawn_fspec_daemon(ws.path())
rust/fspec/tests/cross_frontend_parity.rs:151:26:spawn_fspec_daemon(ws.path())
rust/fspec/tests/cross_frontend_parity.rs:417:30:spawn_fspec_daemon(ws.path())
rust/fspec/tests/bug539_corrupt_board_daemon_startup.rs:35:26:spawn_fspec_daemon(workspace.path())

```

## fspec astgrep --pattern 'CommandBuilder::new($$$)' --lang rust --path rust
```
rust/tools/src/bridge_pty.rs:115:19:CommandBuilder::new(&shell)
rust/tools/src/unified_exec/spawning/pty.rs:58:23:CommandBuilder::new(pty_program)
rust/fspec/tests/scrollback_pty_rpc078.rs:137:19:CommandBuilder::new(fspec_bin())

```

## fspec astgrep --pattern 'claim_default_pointer($$$)' --lang rust --path rust
```
No matches found
```

## fspec astgrep --pattern 'write_daemon_entry($$$)' --lang rust --path rust
```
No matches found
```

## Daemon / combined / PTY spawns of the fspec binary (statement scan of rust/fspec/tests at HEAD, before the fix)
```
fspec/tests/cargo_shape/fspec_version_prints_the.rs:80 mode=daemon HOME=False XDG=False arg=fspec_bin(
fspec/tests/cargo_shape/fspec_version_prints_the.rs:126 mode=daemon HOME=False XDG=False arg=fspec_bin(
fspec/tests/cargo_shape/fspec_version_prints_the.rs:171 mode=combined? HOME=False XDG=False arg=fspec_bin(
fspec/tests/client_mode.rs:80 mode=daemon HOME=True XDG=True arg=fspec_bin(
fspec/tests/client_mode.rs:176 mode=daemon HOME=True XDG=True arg=fspec_bin(
fspec/tests/client_mode.rs:643 mode=daemon HOME=False XDG=False arg=fspec_bin(
fspec/tests/client_mode.rs:677 mode=daemon HOME=False XDG=False arg=fspec_bin(
fspec/tests/combined_smoke.rs:65 mode=combined? HOME=False XDG=False arg=fspec_bin(
fspec/tests/combined_smoke.rs:107 mode=combined? HOME=False XDG=False arg=fspec_bin(
fspec/tests/combined_smoke.rs:151 mode=combined? HOME=False XDG=False arg=fspec_bin(
fspec/tests/combined_smoke.rs:193 mode=combined? HOME=False XDG=False arg=fspec_bin(
fspec/tests/combined_smoke.rs:239 mode=combined? HOME=True XDG=True arg=fspec_bin(
fspec/tests/combined_smoke.rs:438 mode=combined? HOME=True XDG=True arg=fspec_bin(
fspec/tests/common/mod.rs:95 mode=daemon HOME=False XDG=False arg=fspec_bin(
fspec/tests/common/mod.rs:120 mode=combined? HOME=False XDG=False arg=fspec_bin(
fspec/tests/cross_frontend_parity.rs:705 mode=daemon HOME=False XDG=False arg=common::fspec_bin(
fspec/tests/daemon_lifecycle_rpc011.rs:209 mode=daemon HOME=False XDG=True arg=env!("CARGO_BIN_EXE_fspec"
fspec/tests/daemon_lifecycle_rpc011.rs:318 mode=daemon HOME=False XDG=True arg=env!("CARGO_BIN_EXE_fspec"
fspec/tests/daemon_lifecycle_rpc011.rs:397 mode=daemon HOME=False XDG=True arg=env!("CARGO_BIN_EXE_fspec"
fspec/tests/daemon_mode.rs:172 mode=daemon HOME=False XDG=False arg=fspec_bin(
fspec/tests/daemon_mode.rs:212 mode=daemon HOME=False XDG=False arg=fspec_bin(
fspec/tests/daemon_mode.rs:344 mode=daemon HOME=False XDG=False arg=fspec_bin(
fspec/tests/daemon_mode.rs:448 mode=daemon HOME=True XDG=True arg=fspec_bin(
fspec/tests/daemon_mode.rs:562 mode=daemon HOME=False XDG=False arg=fspec_bin(
fspec/tests/daemon_mode.rs:599 mode=daemon HOME=False XDG=False arg=fspec_bin(
fspec/tests/rpc442_daemon_registers_start_workspace.rs:17 mode=daemon HOME=True XDG=True arg=common::fspec_bin(
fspec/tests/rpc446_per_daemon_discovery.rs:156 mode=daemon HOME=True XDG=True arg=common::fspec_bin(
fspec/tests/rpc446_per_daemon_discovery.rs:199 mode=daemon HOME=True XDG=True arg=common::fspec_bin(
fspec/tests/rpc447_daemons_cli.rs:91 mode=daemon HOME=True XDG=True arg=common::fspec_bin(
fspec/tests/scrollback_pty_rpc078.rs:137 mode=pty HOME=False XDG=False arg=fspec_bin(
fspec/tests/stale_daemon_json_rpc011.rs:113 mode=daemon HOME=False XDG=True arg=env!("CARGO_BIN_EXE_fspec"
fspec/tests/status_subcommand_rpc011.rs:42 mode=daemon HOME=False XDG=True arg=env!("CARGO_BIN_EXE_fspec"
rpc-server/tests/common/mod.rs:81 mode=combined? HOME=False XDG=False arg=bin
```

## Findings

- Only the fspec binary writes discovery files: fspec/src/daemon.rs and fspec/src/combined.rs call common::claim_default_pointer and common::write_daemon_entry under $XDG_RUNTIME_DIR/fspec (else ~/.fspec). No other crate's tests start it.
- tests/common::spawn_fspec_daemon and spawn_fspec_combined passed neither HOME nor XDG_RUNTIME_DIR, so every caller (bug539, daemon_mode, client_mode, cross_frontend_parity, daemon_lifecycle_rpc011, cargo_shape) ran its daemon against the user's real runtime dir. bug539 seeds a tempfile workspace, as in the incident.
- Ad-hoc spawns with the same problem: daemon_mode, client_mode (two daemons in the reconnect test), cross_frontend_parity (dead-proxy daemon), cargo_shape/fspec_version_prints_the (cwd daemon, combined), combined_smoke (4 combined spawns), scrollback_pty_rpc078 (PTY combined mode).
- stale_daemon_json_rpc011, status_subcommand_rpc011 and daemon_lifecycle_rpc011 isolate XDG_RUNTIME_DIR but keep the user's HOME (~/.fspec logs and data).
- rpc442, rpc446, rpc447, rpc456, bug595 and two client_mode/daemon_mode tests already isolate both.
