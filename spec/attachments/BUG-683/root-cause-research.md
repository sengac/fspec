# BUG-683 root cause: a resumed Codex session ran a turn before its saved state was restored

Investigated read-only on 2026-11-09 from the user's session
`b279075a-5d4a-4b37-8a11-2ea345fe56aa` (codex/gpt-6-astra): the daemon log
`~/.fspec/logs/private/fspec-combined.log.2026-11-08`, the session manifest
`~/.fspec/sessions/<id>.json`, its provider state `~/.fspec/provider-state/<id>/`,
its message records and its transcript. Nothing was written.

## 1. What is on disk

- Manifest revision 1858, 748 messages. `compaction_commit` identity generation **1**
  (operation 89984bf1, revision 1079, source generation 0, `publication_pending: false`);
  the payload is the Codex generation-1 boundary checkpoint (`items: []`).
- `provider_state` points at state root `96e49b18...json`. It holds two Codex checkpoints:
  generation 0 (prefix 434, revision 1075, 611 items) and generation **1**
  (prefix 741, revision 1846, 1112 items). Both are valid; nothing in the store is corrupt.
- Messages 742 to 747 were written by the failed turns on 2026-11-08 at 11:10 and 11:25 UTC:
  two supervisor requests, two short assistant replies, the second supervisor request and an
  input-disposition note. None of them has a provider checkpoint.

The only compaction was at 00:28 to 00:30 UTC (generation 0 to 1). The session then worked
normally at generation 1 for five hours (the generation-1 checkpoint was last saved at 05:25).
So compaction did not cause the failure, and generation 1 was never treated as a rollback.

## 2. How the counters are persisted

- `CodexHistory` (rust/providers/src/codex/history.rs) keeps the live `generation` and the
  last checkpoint in memory. A fresh `CodexSession` starts at generation 0 with no checkpoint.
- Each completed request calls `commit_for_generation(request, output, generation)`, which
  hands the checkpoint (carrying that generation) to the writer bound by
  `Session::bind_provider_persistence` (rust/cli/src/session/provider.rs).
- The writer calls `persist_provider_state` (rust/core/src/persistence/provider_state.rs),
  which loads the saved state root and calls `State::upsert`
  (rust/core/src/persistence/provider_state/types.rs). `upsert` refuses any checkpoint
  whose generation is lower than a saved checkpoint of the same provider:
  `incompatible provider state: generation rollback`. That refusal is correct: it keeps a
  stale runtime from overwriting newer saved state.
- The "generation 1, request 5" in the ContextSnapshot is not the live provider generation.
  `ContextProjection::emit` (rust/cli/src/interactive/context_projection.rs) reads the
  generation from the manifest's `compaction_commit` when the request context has none, and
  `request` is a process-wide counter. So the snapshot showed the saved generation 1
  while the live Codex history was at generation 0.

## 3. The sequence (log lines 1080 to 1205, and 533 to 561 of the excerpt)

1. 11:10:06 the daemon had restarted; input was refused with "session is not live".
2. 11:10:17 `resume_session` began. `create_session_from_manifest` built a new
   `BackgroundSession` with a fresh `CodexSession` (generation 0, no checkpoint), set
   `manifest_restore`, spawned the agent loop and inserted the session in the live map
   (11:10:18.57).
3. 11:10:18.60 "restoring messages and token state" began: archival transcript, then
   `get_session_message_envelopes`. On this session that took about 51 seconds
   (envelopes logged at 11:11:09).
4. 11:10:27 the client's resume RPC hit its 10-second deadline (`DeadlineExceeded`). The
   restore kept running on the server.
5. 11:10:28.66 the supervisor's message arrived. `send_input` only checks that the session
   is in the live map (`live_session`, BUG-328), so it was admitted. Nothing checks
   `manifest_restore`.
6. The agent loop ran the turn on the unrestored session: the request carried only the
   system prompt and the new input (17,892 tokens, 2 % fill), the model answered, and
   `commit_for_generation(.., 0)` asked the store to save a generation-0 checkpoint. The
   store holds generation 1, so `upsert` refused it: "generation rollback".
7. `block_terminal_request_failure` (rust/cli/src/interactive/request_failure.rs) then set
   `compaction_inference_block` to "selected request requires resolution: {error:#}", so
   the next queued input failed with "summary submission blocked ordinary inference: ...".
   `{error:#}` prints every layer of the chain, and the two `RequestError` wrappers repeat
   the same text, so the cause appears three times.
8. 11:24:53 to 11:25:20 the same thing happened again after the user closed and resumed the
   session: resume RPC deadline at 11:25:03, input admitted at 11:25:11, rollback at
   11:25:20. The late restore at 11:25:48 then ran against a manifest the failed turn had
   already changed, and no "session resume complete" line follows (most likely the replay
   revision check refused the restore). The live session was left with a blocked
   generation-0 history.

## 4. Root cause

A session resumed from its manifest is put in the live map, with its agent loop running,
before its saved history and provider state are restored, and input is admitted while
`manifest_restore` is still set. On a large session the restore outlasts the client's resume
deadline, so the user's next message runs on an empty generation-0 Codex history. Its
checkpoint write is then correctly refused as a rollback below the saved generation 1, and
the session stays blocked.

Secondary defects:
- The resolution block repeats the same cause three times and does not say what to do.
- The ContextSnapshot reports the saved generation while the live provider history is at
  another generation. Not changed by this card.

## 5. Fix (this card)

- `BackgroundSession` refuses input while its manifest restore is pending. This covers user
  input, steers and queued input, `/loop` firings, supervisor messages and manual compaction.
  The refusal says the session is still restoring its saved history and to send the input
  after the resume completes, or to resume again if the resume failed. Nothing is echoed,
  queued or written to the manifest, so the restore that is still running is not disturbed.
- A terminal provider-state failure names its cause once, and for a provider-state
  (checkpoint) failure says to close the session and resume it to reload its saved state.

## 6. Recovery of the stuck session

The stored state is consistent: committed compaction generation 1, saved Codex checkpoint
generation 1 at prefix 741, and messages 742 to 747 after it. A fresh resume installs the
committed generation 1 and the saved generation-1 checkpoint. Later turns replay messages
742 to 747 as ordinary history after the checkpoint. No hand-editing is needed.

Checked on 2026-11-09 against a copy of the session's manifest, provider state, message store,
blobs and transcript (the original was left untouched; its SHA-256 was the same before and
after). The copy was restored the way `resume_session` restores it:
`preflight_compaction_restore`, `restore_compaction_boundary` and `bind_provider_persistence`
on a Codex gpt-6-astra session. The result: 710 replay envelopes, live Codex generation 1,
the saved generation-1 checkpoint with 1112 items, model gpt-6-astra, and no inference block.
Loading the envelopes and restoring took 3.4 s on the copy, so most of the 51 s the daemon took
was spent elsewhere. The likely place is `restore_archival_transcript` over about 4,950
transcript events, or the daemon being under load; this was not measured.

After the reinstall: resume the session and wait for the resume to finish (about a minute on this
session) before sending input. With the fix, input sent earlier is refused with that
instruction instead of breaking the session. No new product recovery path is needed:
closing and resuming the session is the recovery path.
