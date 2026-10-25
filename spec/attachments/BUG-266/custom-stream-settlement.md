# BUG-266: Rhai stop, trailing usage and stream settlement

## Finding and evidence

Source-confirmed shared plumbing issue; user-installed scripts were not audited.

- `rust/providers/src/custom/stream_convert.rs:131`: handle_stop records stop,
  then calls mark_done.
- `rust/providers/src/custom/stream.rs:163`: mark_done terminates parsing;
  line 211 suppresses following frames, including trailing usage-only events.
- `rust/providers/src/custom/stream_http.rs:99`: records stop but continues
  reading; [DONE] at line 89 is what immediately ends the HTTP stream.
- `rust/providers/src/custom/rig_streaming.rs:108`: Final follows stream exhaustion.

An OpenAI-compatible script can map finish_reason to stop, lose subsequent
usage, then wait on a still-open connection until shared stall handling rather
than settling and nudging. The issue is not an infinite-hang claim.

## Acceptance/test direction

Specify a coherent lifecycle for finish_reason, usage, [DONE], EOF and errors.
Test usage after stop, stop with a pending connection, explicit [DONE], missing
terminal evidence, and transport error after stop. Verify exact billed usage,
honest errors and prompt Final/continuation when the protocol permits settlement.
Do not discard late usage merely to close quickly or mask post-stop failure.

Existing `rust/providers/tests/bug227/downstream.rs:64` fixtures put usage before
stop. `rust/providers/tests/bug227/bridge.rs:76` intentionally expects a post-stop
transport failure to remain failure: reconcile that contract before changing it.
History: original behavior from `75a9ba7d`; bridge/error hardening in `2699693b`.
See [full audit](../CONT-011/fspec-auto-continue-investigation.md).
