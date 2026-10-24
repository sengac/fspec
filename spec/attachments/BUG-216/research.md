# BUG-216: Streaming loop-detector false positives

## Scope and confidence

Investigated on 2026-10-21 at `7e60de8e`. Three detector defects remain in source. The first screenshot's internal abort is confirmed by logs and persisted response replay; there is no need to infer a network outage. No production changes, builds or Cargo tests were made during the audit.

## Runtime evidence

The two-hour audit window was 02:48:19–04:48:19 UTC (12:48:19–14:48:19 local, UTC+10). Three warning/abort pairs reported `LongSuffixMatch { m: 16 }`:

- `321a7519-b009-497d-98ee-5acc966a52a4` at 02:59:36 UTC.
- `f44d4706-edfc-4401-bdcf-4a7cf1425178` at 03:18:23 and 03:18:33 UTC.

For the first incident, warning and abort were approximately 2 ms apart. The session became Idle at 02:59:37.689, queued synthetic `Continue` at 02:59:37.691 (retry 1/10), and returned Running at 02:59:38.749.

A read-only replay of saved deltas identified a repeated reference to `rust/fspec-tui/src/app/dispatch_esc_cascade.rs:`. Delta-local splitting treated its subword fragments as a 16-word suffix. Concatenating the response before whitespace tokenization did not produce that long-suffix match. This was ordinary technical prose, not demonstrated repetition collapse.

## Three source defects

1. **Chunk boundaries become word boundaries.** `rust/agent-loop/src/stream_loop_detector.rs:156` applies `split_whitespace()` independently to each delta. It never retains an incomplete word across chunks. Detection therefore depends on arbitrary provider chunking.
2. **Configured long-match tolerance is ignored.** The configuration includes `min_long_match_repeats` (default 3), but the long-suffix branch at line 218 returns on one earlier occurrence.
3. **Latched evidence repeatedly escalates.** Later `feed()` calls return the same latched signal immediately. `rust/agent-loop/src/background_output.rs:210` sends each return through escalation as though it were a new trigger; the next chunk can turn warning into abort without additional repetition.

The abort path at `rust/agent-loop/src/background_output.rs:259` records a marker, stages a corrective note and calls `session.interrupt()`. Retry scheduling is at `rust/agent-loop/src/agent_loop.rs:1562`.

## Existing specifications and coverage

- `spec/features/streaming-loop-detection.feature:18` describes word-level evidence, a repeat threshold and fresh-trigger escalation. Its narrative and examples should be reconciled before choosing exact threshold semantics.
- `rust/agent-loop/tests/rig014_streaming_loop_detection.rs:222` feeds enough copies to detect repetition but does not prove earlier copies remain below threshold; premature latching can pass unnoticed.
- `rust/agent-loop/tests/rig015_loop_abort_behavioral.rs:200` tests behavioral cancellation with a stub provider, not the combined Codex notification sequence.

## Regression plan

- Property-test equivalent text under arbitrary subword chunk boundaries, including paths and punctuation.
- Cover two legitimate references to the same long pathname with distinct surrounding prose.
- Assert no long-suffix match before the configured repetition threshold and detection once it is met.
- Assert latched state alone cannot count as independent post-warning evidence.
- Retain bounded memory, per-channel isolation, reset behavior and genuine degenerate-loop detection.
- Exercise warning, internal abort, persistence and bounded retry together.

Do not fix this by globally disabling detection or merely raising thresholds. BUG-219 covers the misleading Codex banner; BUG-220 covers missing interruption provenance. A correct detector fix does not eliminate those independent defects.
