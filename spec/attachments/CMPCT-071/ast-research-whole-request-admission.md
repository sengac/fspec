# CMPCT-071 discovery

2026-10-17 successor session 663f0f7e resumed old agent 2df10a3b. Historical
session previews show CMPCT-071 remained specifying while CMPCT-082 acquired
the first failing tests; no earlier CMPCT-071 implementation was claimed.

AST query executed with AstGrep, Rust, scope
`rust/cli/src/compactor_sub_agent.rs`:

```text
pub fn pin_dag_to_session($$$PARAMS) { $$$BODY }
```

Result:

```text
rust/cli/src/compactor_sub_agent.rs:119:1:
pub fn pin_dag_to_session(session: &mut Session, dag_content: &str) {
```

The current primitive resets reminders, appends a wrapped DAG and recalculates
the token tracker. It has neither a usable-capacity argument nor a request-fit
veto. The calling round reports completion after this mutation. Existing tests
in `rust/cli/tests/cmpct044_compactor_pin_test.rs` establish that this is a real
runtime route, not merely a parser helper.

Acceptance repair requires the complete selected provider request before any
mutation or durable promotion. The pin primitive cannot derive tool schemas,
media accounting or verified suffix identity from DAG text. Integration must
therefore use the shared generation-commit interface and physical-send admission
contract, not a local DAG-only estimate. Owners of CMPCT-070/072, CMPCT-082 and
generation commits were contacted; busy-session responses did not queue messages.

Prerequisites CMPCT-065, CMPCT-070 and CMPCT-082 remain lifecycle blockers.
No shared production files are changed without coordination.
