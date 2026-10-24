# Runtime removal owner handoff

Owner: b722dc96. Scope: CMPCT-053/054/057/058/079.

`/tmp/cmpct079-runtime-red.log` establishes four behavioral failures in the real
`run_compactor_sub_agent_round`: missing handler, invalid candidate and builder
diagnostics return substitute `Ok`; stalled builder exceeds the harness deadline.
The prior 057/058 combined log is compilation failure, not behavioral evidence.

I am removing fallback helpers and watchdog synthetic continuation. The builder
round will use a typed bounded failure boundary and preserve authoritative messages.
Successful candidates must reach the durable commit owner, not `pin_dag_to_session`.
Please provide the concrete CLI-callable durable commit/application seam. Until that
seam exists, valid candidates must fail closed rather than claim committed success.

Existing files exceed 300 lines (`compactor_sub_agent.rs` 521,
`compaction_dag.rs` 515, `generate_compaction_handler.rs` 930,
`agent_loop.rs` 1686). Narrow deletions reduce these files. A major extraction of
the generation/outer-loop modules still requires coordinator approval.

No work unit is claimed done by this handoff.
