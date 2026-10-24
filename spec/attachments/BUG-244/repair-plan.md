# BUG-244: Lossless, ordered, deduplicated resume delivery

## Priority and evidence

**P1 — source-confirmed delivery/recovery flaw; screenshot-specific broadcast loss is unproven.**
The target session contains 662 records reconstructing to 2,479 chunks: 1,817 content blocks
and 662 `Done` events. Its explicit first-1,000-chunk snapshot stops mid-text at manifest
position 62, timestamp 21:15:05 AEST, although saved history continues until 22:24:00.
That snapshot omits 1,479 chunks. Some may arrive through concurrent live restoration,
so do not describe 1,000 as an unconditional total-delivery cap.

## Root cause and code to inspect

- `rust/fspec-tui/src/app/dispatch_attach.rs:254`: fixed 1,000-chunk fetch; line 256 silently converts errors to an empty result.
- `rust/sessions/src/background_session.rs:1091`: `take(limit)` returns the oldest prefix, without pagination.
- `rust/sessions/src/session_restore.rs:109`: every restored chunk is also broadcast as live output.
- `rust/sessions/src/background_session.rs:94`: bounded broadcast capacity, including 256-chunk channels.
- `rust/rpc-server/src/server.rs:359` and `rust/fspec-tui/src/app/bootstrap.rs:173`: lag handling has no durable backfill.
- `rust/fspec-tui/src/app/dispatch_stream_chunks.rs:216`: normal recording appends without a replay boundary.
- `rust/fspec-tui/src/store/agent_view/chunk_tool_result.rs:55`: results need an existing matching tool card.

## Required repair

1. Specify stable session-generation and event-sequence identities plus an explicit snapshot boundary.
   Fetch bounded pages with continuation and completion state, rather than a fixed prefix.
2. Establish a race-free snapshot/live handoff. Events produced during hydration must be buffered or
   fetched after the snapshot cursor; both delivery paths must apply each identity once in source order.
3. Recover broadcast gaps using authoritative cursors. If recovery is impossible, show a clear error
   and retry state instead of implying that an incomplete transcript is complete.
4. Make initial hydration, reopened tabs, reconnect, and repeated manual resume idempotent.
   Avoid rebroadcasting restored history as new live agent activity to unrelated consumers.
5. Preserve tool-call/result pairing, message boundaries, original timestamps, and existing live content.
6. Implement one transport-neutral contract used by embedded and WebSocket paths; bound page memory
   and cancellation work. Obtain approval before a major RPC/replay refactor.

## Regression scenarios

- Cold resume greater than 1,000 and greater than 2,479 chunks: assert exact final content, order,
  tool cards/results and tail, not merely successful RPC return or message counts.
- Force more than 256 events of lag, then verify complete cursor-based backfill.
- Emit live output at every snapshot/page/handoff boundary; assert no omissions or duplicates.
- Resume an already-open session twice, reconnect, and reopen a tab; transcript stays stable.
- Fail a page, disconnect mid-page, cancel hydration, and change session focus; errors and retries
  remain scoped to the correct session/generation and do not corrupt existing display state.
- Exercise both transports and real backend/TUI plumbing with deterministic synchronization.

The current mocked resume test sends two text chunks and asserts the 1,000 limit.
`rpc422_session_persistence` backend success/count tests do not prove rendered completeness.

## Prohibited shortcuts and boundaries

- Do not merely increase 1,000, return the latest tail, or enlarge broadcast channels.
- Do not append a second copy of a snapshot to hide gaps, or silently swallow fetch errors.
- Full archival scope versus compacted model context is BUG-245; this card owns reliable delivery
  of the selected transcript source. Coordinate schemas rather than creating competing replay protocols.
- Existing eight-line collapsed tool views and marked visual trimming are not storage loss.

## Completion evidence

- [x] Long-session exact-once content/order tests pass with forced lag and concurrent live output.
- [x] Both transports and repeated resume share the same cursor contract.
- [x] Failure is visible and recoverable; memory/page limits and cancellation are tested.
- [x] New scenarios precede red tests and implementation; scoped quality checks pass.

Final evidence: `final-acdd-validation.md`.

Related: BUG-245, BUG-243, BUG-250. Investigation:
`docs/investigations/2026-10-23-compaction-and-resume.md`.
