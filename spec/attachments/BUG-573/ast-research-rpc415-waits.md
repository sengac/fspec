# AST research BUG-573: reconnect_resubscribe_rpc415 waits
```
rust/fspec-tui/tests/reconnect_resubscribe_rpc415.rs:112:5:codelet_test_helpers::wait_until("the list_work_units refetch", || {
rust/fspec-tui/tests/reconnect_resubscribe_rpc415.rs:253:5:codelet_test_helpers::wait_until("the work_units subscriber to bind", || {
```
Findings: the test's waits are wait_until on mock-recorded facts (list_work_units refetch, work_units receiver binding) under deadlock guards since b0629fc06; no wait on subscriber task exits remains. Stress: 260/260 runs pass under load.
