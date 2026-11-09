# AST research: RPC-446
```
fspec/src/common/daemon_json.rs:37:1:pub fn write_daemon_json(

fspec/src/common/daemon_json.rs:175:1:fn pid_is_alive(pid: u32) -> bool {
fspec/src/common/daemon_json.rs:186:1:fn pid_is_alive(_pid: u32) -> bool {

fspec/src/daemon.rs:62:        common::write_daemon_json(&djson, addr.port(), process::id(), &workspace, started_at)
fspec/src/daemon.rs:117:    common::remove_daemon_json(&djson);
fspec/src/combined.rs:50:        common::write_daemon_json(&djson, addr.port(), process::id(), &workspace, started_at)
fspec/src/combined.rs:69:    common::remove_daemon_json(&djson);
```
