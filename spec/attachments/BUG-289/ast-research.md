# AST research: BUG-289
```
fspec/src/combined.rs:54:9:common::write_daemon_json(&djson, addr.port(), process::id(), &workspace, started_at)
fspec/src/daemon.rs:66:9:common::write_daemon_json(&djson, addr.port(), process::id(), &workspace, started_at)

fspec/src/combined.rs:85:5:common::remove_daemon_json(&djson)
fspec/src/daemon.rs:133:5:common::remove_daemon_json(&djson)

```
