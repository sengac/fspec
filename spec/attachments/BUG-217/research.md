# BUG-217: Lossy subordinate stream forwarding

## Scope and confidence

Investigated on 2026-10-21 at `7e60de8e`. Runtime loss counts and the absence of replay in the lag branch are confirmed. The audit did not establish that every lost chunk was user-visible text; chunks can also carry lifecycle and accounting information. No load test or live relay call was performed.

## Observed evidence

Between 02:56:28 and 04:04:01 UTC, the combined log contained 621 `SESS-015` relay-lag warnings. Their skipped-message counts totalled 4,052 across five subordinate subscriptions.

Source log: `~/.fspec/logs/fspec-combined.log.2026-10-21`. Count application warning records, not historical warning text embedded inside INFO tool-result payloads.

## Source chain

- `rust/sessions/src/background_session.rs:91` documents the bounded supervisor broadcast buffer; capacity is 256 chunks.
- `rust/napi/src/agent_manager_handler.rs:207` subscribes to subordinate output and forwards it to the root parent's relay.
- At line 302, `RecvError::Lagged(n)` logs the loss and resumes receiving. There is no replay of skipped chunks in that branch.
- Continuing reception only reaches newer retained chunks; it does not recover the messages Tokio already discarded for this receiver.

This is not merely noisy logging. A slower subscriber loses information. The exact downstream impact depends on which chunk types were skipped and whether another independent consumer persisted them.

## Reproduction outline

Use the real bounded broadcast subscription with a deliberately stalled forwarding consumer. Produce more chunks than its capacity, including text and a terminal/control event. Resume the consumer and verify either ordered complete replay or an explicit gap plus authoritative reconciliation. Do not rely on timing-only sleeps when a barrier can control the stall.

The reproduction is proposed, not executed during this investigation.

## Specification and test work

- Review `spec/features/subordinate-session-persistence.feature` for durable-source guarantees, and `spec/features/agentview-spawned-subordinate-session-registration.feature` for subscription lifecycle.
- Locate existing SESS-015 forwarding tests in `rust/napi/src/agent_manager_handler.rs` before adding an integration test.
- Test burst load, sustained lag, root-parent routing, closed relays, cancelled subscriptions and terminal-event delivery.
- If replay is selected, test sequence IDs, duplicate suppression, ordering and reconnect catch-up using actual persisted output.
- Define whether reasoning/text deltas may be coalesced, and which control events must never be dropped.

## Recommended direction and questions

Choose an explicit delivery contract: durable replay with cursors, bounded backpressure, or documented coalescing plus state reconciliation. Increasing the buffer may reduce frequency but cannot guarantee correctness for an arbitrarily slow receiver. Do not introduce unbounded queues as a default fix.

Investigate why the consumer falls behind, including serialization and relay backpressure, before optimizing. Preserve the distinction between a subordinate's intact durable transcript and loss on the parent-forwarding subscription.
