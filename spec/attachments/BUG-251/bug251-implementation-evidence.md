# BUG-251 implementation checkpoint — validation blocked

## Honest status

BUG-251 is **implementing**, not complete or validating. Moving to validating
would misrepresent the missing full GREEN and post-integration quality checks.
The coordinator must resolve the shared MessageStore prerequisite and resume
this worker. No commits, live-data edits, configuration rewrites, alternate
attachment store or parallel retry spool were introduced.

## ACDD evidence

- Discovery used AST search of the actual user persistence call and inspected
  input transport schemas, provider adapters, blob processing and replay.
- Rules, examples and architecture notes are recorded on BUG-251.
- Feature `faithful-multimodal-archives.feature` precedes executable tests.
- Testing state preceded test creation. Production followed executable RED.
- `/tmp/bug251-red-executable.log`: 2 pass, 4 fail, actual runtime assertions.
  Accepted incoming images were absent from originals; stored image/document
  blocks failed cold replay. Legacy Unicode and invalid-source baselines passed.
- Earlier compiler failures, missing output directories and Cargo lock waits
  are explicitly NOT behavioral RED. The successful RED build took 41 minutes.
- Six scenarios map exactly 1:1 to six tests, with every exact `@step` inside
  its matching function. Coverage reports 100% scenario/test/implementation
  mappings, not a claim of passing behavior or measured line coverage.

## Connected implementation

- NEW `rust/agent-loop/src/user_persistence.rs` builds original ordered text
  and base64 image blocks, delegating to BUG-246 `persist_user_content`.
- Exactly one `agent_loop.rs` user call/import changes; BUG-246's fail-closed
  recovery gate remains intact. No edits to its persist/background-output files.
- NEW `rust/sessions/src/replay_media.rs` decodes supported base64 images,
  external image URLs, base64 PDF and PDF URL sources into authentic rig media.
- `replay_content.rs` connects the media helper and typed compatibility errors.
  Existing incoming-image chunks retain display image data and MIME identity.
  Documents/URLs receive honest attachment labels, not fake media text in model
  context. External URL contents are not captured or claimed immutable.
- Existing blob store and BUG-246 writer own extraction, `_blobRefs`, recovery
  and identity. Stored document fixtures now call that real common writer;
  their former temporary duplicate envelope writer was removed.
- No document-send UI is invented. OpenAI Responses supports URL/PDF regression
  sources; current Anthropic rig URL-document limitations remain unchanged.
- Actual `maxImages` is a Read-tool result budget, not a history clipping policy.

## Tests and checks

- Real embedded tarpc and WebSocket accepted-image input routes.
- Bounded originals, exact content/order/type, oversized-page typed errors.
- Fresh child process cold restore and physical next Anthropic request.
- Physical OpenAI Responses PDF/URL request and original document metadata.
- Invalid MIME/base64/URL and oversized-image compatibility failures.
- Existing blob/message cleanup, reopen, missing-source typed originals failure.
- Legacy Unicode plus current text-only writer and persisted Read image budget.
- Assertions avoid dumping base64 payloads.
- Scoped check passed before later shared-core additions:
  `cargo check -p codelet-sessions --test bug251_multimodal_archives --profile release-slim -j 2`
  Log `/tmp/bug251-check.log`.
- Scoped Clippy is BLOCKED, not passed: `/tmp/bug251-clippy.log` reports E0599
  `conversation_commit.rs:20`: `MessageStore::store_with_id` does not exist yet.
  That helper belongs to BUG-246; MessageStore write ownership is BUG-247/248.
- Feature validation, tag validation, owned-file rustfmt and whitespace checks
  pass. New files remain below 300 lines; main integration test is 291 lines.

## Coordinator prerequisite and next commands

BUG-246's agreed stable-ID MessageStore API is documented in
`spec/attachments/BUG-246/coordination-contract.md`. Owner 247 was idle and was
successfully messaged by both 246 and 251, but no new assistant activity or
primitive appeared during this checkpoint. Parent may need to restart or
explicitly reassign that owner. Do not bypass retry ownership to unblock tests.

After the primitive lands, run only scoped commands, teeing distinct logs:

```sh
cargo test -p codelet-sessions --test bug251_multimodal_archives --profile release-slim -j 2 -- --nocapture
cargo check -p codelet-sessions --test bug251_multimodal_archives --profile release-slim -j 2
cargo clippy -p codelet-sessions --test bug251_multimodal_archives --profile release-slim -j 2
```

Resolve only BUG-251 regressions, refresh exact mappings if test line numbers
change, attach GREEN/quality evidence, then leave BUG-251 **validating** for the
parent's independent done audit. Shared architecture approval and ownership
boundaries remain in force. Last available disk space was about 19 GiB.
