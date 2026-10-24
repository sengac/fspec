# BUG-219: Misleading Codex cancellation notification

## Scope and confidence

Source-confirmed at `7e60de8e`, researched on 2026-10-21. The screenshot sequence is explained by an internal loop abort plus an unconditional reconnection notification. It does not establish a network outage. No new tests or live network calls were run.

## Screenshot timeline

Screenshot: `Screenshot 2026-10-21 at 1.00.35 pm.png` (Desktop; the original filename uses a narrow space before `pm`). Matching session: `321a7519-b009-497d-98ee-5acc966a52a4`.

- 02:59:36.439/441 UTC: loop detector warning and abort.
- 02:59:37.689: Running becomes Idle.
- 02:59:37.691: bounded loop-abort recovery queues synthetic `Continue`, retry 1/10.
- 02:59:38.749: the next turn starts with a corrective note.

Later output is a new recovery turn, not the cancelled stream mysteriously resuming. BUG-216 covers why legitimate technical prose triggered the abort.

## Exact source chain

1. `rust/providers/src/codex/reconnect.rs:25` renders `RecoveryStatus::Cancelled` as `Reconnection cancelled`.
2. `CodexRecoveryHandle::track` installs an unfinished guard on each polled Codex stream, not only recovery attempts.
3. `RecoveryGuard::finish` at line 136 checks `is_reconnecting()` before emitting recovered/failed.
4. `RecoveryGuard::drop` at line 150 has no equivalent check: every unfinished drop emits cancelled.
5. `rust/sessions/src/provider_status.rs:13` installs the observer that forwards these provider notifications.
6. `rust/cli/src/interactive/stream_loop.rs:768` drops an interrupted stream first, then emits Interrupted and Done. The guard's notification therefore precedes the interruption notice.

The defect was introduced with recovery commit `dc29ae80` and remains in the audited source.

## Existing tests and missing cases

- Review `spec/features/codex-reconnect-status.feature` and `spec/features/codex-offline-reconnection.feature`.
- `rust/providers/tests/prov160_codex_offline.rs:119` expects cancellation during initial setup/body waiting; distinguish ordinary initial streaming from actual offline/retry recovery before changing assertions.
- Review `rust/providers/tests/prov160_codex_reconnection.rs` and `rust/providers/tests/prov160_reconnect_stream_consistency.rs` for retry ownership and rollback invariants.
- Add a test that cancels a healthy, unfinished Codex stream and receives no reconnection-specific banner.
- Retain cancellation status for a genuinely active wait/retry, with exactly-once terminal notifications.
- Exercise internal loop abort through the TUI-facing output, plus user Escape and successful final/tool completion.

## Recommended direction

Track whether the cancelled operation was actually reconnecting before publishing recovery-specific status. Preserve resource cleanup and correct completion of active recovery. Do not suppress the real Interrupted event to hide the banner bug.

BUG-220 separately tracks reason-specific interruption events. Fixing notification truthfulness should not depend on changing the detector or the entire interrupt protocol.
