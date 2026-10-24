<!-- card: {"id":"CMPCT-052","type":"bug","priority":"P0","findings":"CB-01","dependsOn":["CMPCT-053","CMPCT-055","CMPCT-059"]} -->
# CMPCT-052: Commit submitted DAGs before the next inference

## Confirmation

**Confirmed lifecycle defect; observed compatible historical ordering.**
`rust/agent-loop/src/inject_summary_handler.rs:57` queues an unversioned summary
and returns budget/success before installation. Actual apply is at `:245`.
`rust/agent-loop/src/agent_loop.rs:1398` drains it after the provider runner returns.
The tool loop can therefore continue on the old context. Historical retry exhaustion
preceded pending application; this does not establish the historical build identity.

## Acceptance criteria

- A submission reports queued versus committed honestly; no subsequent ordinary
  physical inference may use the superseded generation after accepted handoff.
- Commit through the shared generation-aware durable owner before resuming;
  preserve uncovered suffixes and actual tool-call/result pairing.
- Failed commit retains the last committed generation and blocks; no synthetic
  Continue, fallback pin or completion. Returned budgets describe the full request.

## Regression scenarios

- Given a streaming tool loop, when InjectSummary submits a DAG and the agent would
  infer again, then the next captured wire request uses the committed generation.
- Given a failed persistence step after submission, when the loop reaches the
  handoff, then no next inference or completion occurs and old state remains intact.

## Coding notes

Introduce an explicit handoff/stop outcome rather than relying on outer-loop
timing. Coordinate with lock ownership, candidate versioning and durable commit
cards; do not duplicate application logic in CLI and tool entry points.
Start with `rust/patches/rig-core/src/agent/prompt_request/streaming.rs:791`
and `rust/cli/src/interactive/stream_loop.rs:1042` to inspect continuation behavior.
Use real dispatch plus a scripted provider, not just a handler unit test.

```mermaid
sequenceDiagram
    participant Loop as Tool loop
    participant Owner as Commit owner
    participant Wire as Provider
    Loop->>Owner: Submit candidate with generation
    Owner->>Owner: Validate and durably commit
    alt Commit succeeds
        Owner-->>Loop: Committed generation
        Loop->>Wire: Next request from committed memory
    else Commit fails
        Owner-->>Loop: Typed blocked outcome
        Note over Loop,Wire: No next inference
    end
```

Mandatory companion: `00-shared-contract.md`. Research: CB-01 in
`compaction-research/compaction-bugs/01-runtime-and-concurrency.md`.
