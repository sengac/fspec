# TEST-042 AST research: wall-clock waits in fspec's test code

fspec astgrep runs over rust/ (2026-11-06), counting matches per pattern (production and test code alike); the exact check in rust/test-helpers/src/wall_clock_waits.rs reports 846 offences in 372 files (by call: tokio::time::timeout 430, tokio::time::sleep 320, std::thread::sleep 72, recv_timeout 20, interval 2, timeout_at 1, sleep_until 1).

- `tokio::time::sleep($$$A)`: 293 matches
- `tokio::time::timeout($$$A)`: 282 matches
- `std::thread::sleep($$$A)`: 63 matches
- `timeout($D, $F)`: 95 matches
- `sleep($D)`: 13 matches
- `$R.recv_timeout($D)`: 22 matches

## Sample (first 20 of tokio::time::timeout)
```
rust/core/src/lifecycle_hooks/executor.rs:70:11:tokio::time::timeout(timeout_duration, child.wait())
rust/cli/src/interactive/recovery_stall.rs:124:22:tokio::time::timeout(timeout_duration, stalled_stream.next())
rust/cli/src/interactive/recovery_stall.rs:181:19:tokio::time::timeout(timeout_duration, active_stream.next())
rust/cli/src/interactive/recovery_stall.rs:224:19:tokio::time::timeout(timeout_duration, slow_stream.next())
rust/cli/src/interactive/recovery_stall.rs:270:19:tokio::time::timeout(timeout_duration, partial_then_stall.next())
rust/cli/src/interactive/recovery_stall.rs:310:13:tokio::time::timeout(std::time::Duration::from_millis(50), never_completing)
rust/cli/tests/cmpct079_pre_prompt_isolation.rs:69:18:tokio::time::timeout(
rust/cli/tests/cmpct058_selected_overflow.rs:61:18:tokio::time::timeout(
rust/cli/tests/truthful_compaction_routes_bug213.rs:49:24:tokio::time::timeout(Duration::from_secs(3), async {
rust/cli/tests/cmpct079_no_substitute_success.rs:81:18:tokio::time::timeout(
rust/cli/tests/prov160_reconnect_watchdog.rs:96:5:tokio::time::timeout(
rust/cli/tests/prov160_reconnect_watchdog.rs:101:5:tokio::time::timeout(
rust/cli/tests/prov160_reconnect_watchdog.rs:108:5:tokio::time::timeout(
rust/cli/tests/prov160_reconnect_watchdog.rs:119:19:tokio::time::timeout(Duration::from_secs(60), async {
rust/cli/tests/fixtures/terminal_error_history.rs:179:9:tokio::time::timeout(
rust/cli/tests/fixtures/committed_interruption.rs:135:9:tokio::time::timeout(
rust/cli/tests/fixtures/committed_interruption.rs:157:9:tokio::time::timeout(Duration::from_secs(10), async {
rust/cli/tests/cmpct052_inference_handoff.rs:38:18:tokio::time::timeout(
rust/cli/tests/cmpct052_inference_handoff.rs:209:18:tokio::time::timeout(
rust/cli/tests/bug221/support.rs:155:9:tokio::time::timeout(
```
