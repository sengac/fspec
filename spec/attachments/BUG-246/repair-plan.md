# BUG-246: Recover conversation persistence failures

## Priority and evidence

**P1 — source-demonstrated irreversible loss path; no overnight write failure was logged.**
The assistant buffer is drained before persistence. If saving fails, content is not restored
or retried. Tool-result saving then proceeds independently. A failed invocation write followed
by a successful result write can leave an orphan result. User-write failure also permits inference.

## Code to inspect

- `rust/agent-loop/src/background_output.rs:303`: takes accumulated content before attempting storage.
- `rust/agent-loop/src/background_output.rs:319`: logs assistant failure without retaining/retrying content.
- `rust/agent-loop/src/background_output.rs:413`: separately persists tool outcome and logs failure.
- `rust/agent-loop/src/agent_loop.rs:392`: user-save error does not block processing.
- `rust/core/src/persistence/manifest_updates.rs:69`: stores message before committing its manifest reference.
- `spec/features/agent-loop-persistence.feature`: explicitly describes nonblocking persistence policy.

## Required repair

1. Define acknowledged, pending, retryable, permanently failed, and promoted-but-unacknowledged
   persistence states. Do not equate emitted UI content with successful durable storage.
2. Retain content ownership until confirmed commit, using a bounded retry queue or durable recovery
   spool as appropriate. Reusing IDs must make retries idempotent across message and manifest writes.
3. Preserve causal order and call/result integrity if assistant or result persistence fails separately.
   A successful result must not make its invocation permanently unreachable.
4. Handle message-written/manifest-failed and manifest-promoted/acknowledgement-failed boundaries
   without duplicate messages, rollback, or re-execution of external tool side effects.
5. Specify behavior when storage remains unavailable: visible scoped error, explicit retry/recovery,
   and deliberate inference/side-effect policy. Change the existing nonblocking contract explicitly
   through ACDD; do not silently turn every transient write failure into a different product behavior.
6. Coordinate durable ordering with BUG-247, framing recovery with BUG-248, and streaming recovery
   with BUG-249. Avoid three independent retry/spool mechanisms.

## Regression scenarios

- Assistant append fails once, then recovers: all original content is saved once, in source order.
- Invocation persistence fails while result persistence would succeed; later reopen has a valid pair.
- Tool-result write fails after the tool caused a side effect; retry does not invoke the tool again.
- User-write failure has the newly specified visible behavior and preserves the input for recovery.
- Message write succeeds but manifest promotion fails; retry attaches/reconciles the same message identity.
- Promotion succeeds but acknowledgement fails; reopen/retry recognizes the committed record.
- Permanent failure and full recovery queue remain bounded, visible, and safe across shutdown.

Use isolated real stores and shared test helpers. Add a narrow storage fault-injection seam where
the real filesystem cannot deterministically establish ordering; do not mock the whole agent loop.
Existing source-order and manifest-count assertions are not sufficient.

## Prohibited shortcuts

- Do not merely add another log line, ignore failed records, or discard the buffer after retry exhaustion.
- Do not fabricate tool outcomes or automatically rerun side-effecting tools to recreate archive data.
- Do not disable lint/error propagation standards or treat an orphan JSONL append as a committed turn.

## Completion evidence

- [ ] Fault-injection tests cover every ownership/commit boundary and cold reopen.
- [ ] Each message and authentic tool result appears once with correct ordering.
- [ ] Permanent-failure policy, retry bounds, and user-visible recovery are specified.
- [ ] Feature-first red/green evidence and scoped quality checks are attached.

Related: BUG-247, BUG-248, BUG-249, PROV-050. Investigation:
`docs/investigations/2026-10-23-compaction-and-resume.md`.
