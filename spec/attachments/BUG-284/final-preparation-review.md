# BUG-284 final preparation checkpoint

**Status: Testing. No production repair, implementation approval or deployment.**

This follow-up incorporates `repair-independent-review.md` and
`repair-plan-audit.md`. It refines the proposed design and acceptance boundaries
in `admission-red-evidence.md`; it does not replace or erase earlier evidence.
The current final hash manifest is `admission-final-hashes.sha256`, not the
earlier pre-refinement `admission-red-hashes.sha256`.

## Current causal evidence and source identity

After the independent review, acceptance was strengthened in five previously
unmapped scenarios. No Rust source or existing mapped test changed. The
`red-final-review` run rechecked the current test/spec snapshot: **nine tests,
seven behavioral failures, two passing controls, exit 101**. Both the six-input
and two-input histories fail actual selected Codex lineage; hook and persistence
cancellation produce the other two exact selected-lineage failures. Runtime-lock
and construction-failure tests expose cold/runtime disagreement, and the killed
child exposes loss after acknowledgment but before dequeue.

Raw current receipt: `admission-final-receipts.tar.gz`, including
`receipts/red-final-review.{log,command,exit,started,finished}` and
`receipts/red-final-review-sources.sha256`. Earlier reviewed receipts remain
valid evidence of their exact earlier snapshots and are retained in the archive.
The final spec refinements did not change the mapped tests or weaken a guard.

Scoped compilation, strict Clippy and Rust formatting checks remain passing for
the identical Rust sources. The unchanged BUG-276/BUG-282 targets pass 5+3 tests.
Final native syntax/tag validation and coverage audit pass (9/9 references).
Independent exact-step/range verification passes: **14 scenarios, 9 test-mapped,
5 unmapped, 0 implementation mappings**. This is not complete acceptance or GREEN.

## Accepted design refinements before implementation

- Admitted input canceled or failed before dispatch must remain immutable and
  admitted, accompanied by truthful durable **model-visible** host disposition
  before subsequent inference. UI transcript events alone are insufficient.
  Bind the disposition to input UUID and execution-attempt identity, preserve
  exact order in runtime/replay/compaction, and fabricate no assistant success.
- An admitted input with uncertain post-crash dispatch is not renewed execution
  permission. Block automatic continuation until an explicit recovery decision
  and truthful model-visible disposition resolve what can actually be known.
  Local attempt start proves neither remote receipt nor remote completion.
- Receipt and recovery-spool originals/blobs must survive failed promotion and
  GC, not just normal manifest membership. Fork/merge cannot manufacture new
  execution rights or drop required cancellation context; unsupported boundaries
  must refuse safely. Identity and cleanup tests remain required.
- Input UUID, execution-attempt identity and genuine internal-continuation
  identity are distinct. Current in-process persistence retry idempotence is not
  an external wire idempotency contract. Identical text is never identity.

These requirements were added to the card's rules/notes and strengthened
unmapped Gherkin scenarios through a normal Testing -> Specifying -> Testing
review cycle. No tests or production implementation were written in that
specification refinement phase. The five unmapped scenarios remain explicit
work; no skeleton coverage was added.

## Exact unresolved coordination and approval boundaries

1. **Explicit human approval remains pending** for the focused mechanical
   decomposition in `cli-split-proposal.md`, targeting only
   `rust/cli/src/interactive/stream_loop.rs` and its proposed child modules.
   The attempted HITL authorization was denied, known not executed. Neither a
   retry nor an indirect replacement/bypass has been made. Mechanical extraction
   requires independent behavior-preservation verification before admission edits.
2. **The ingress acknowledgment bridge remains a design/scope constraint.**
   Current background submission methods are synchronous; a detached persistence
   task cannot provide acknowledged durability. The review identifies discarded
   ordinary-input errors in `rust/sessions/src/handle_impl.rs`,
   `rust/rpc/src/service_implementation/interaction_sessions.rs` and
   `rust/rpc/src/mcp_dispatch/group_15.rs`, plus absent caller submission identity
   in `rust/rpc-types/src/lib.rs`. These are concurrently owned interfaces and
   are **not** authorized for edits by the CLI approval request. Coordinate exact
   additional scope before promising caller-visible durable success or external
   retry deduplication. This report does not silently narrow the user's retention
   contract or claim those interfaces have been repaired.

## Test limitations retained explicitly

The current queue cancellation path, real durable originals, streamed checkpoint
and selected-compaction commit are causally composed. Fresh accepted history
still grows through direct test persistence plus `run_agent_stream`, not regular
queued fresh admission through the production provider factory. The seed/fresh
loopback request observers do not cover that factory: their counts alone cannot
prove zero possible canceled dispatches. Current assertions establish no canceled
runtime/observed loopback instruction, no stream Error on cancellation, and no
repeated isolated effect. Do not upgrade this to complete dispatch coverage.

The new historical `ancestor` mutation is user ancestry, not an established DAG;
genuine established-DAG rejection is preserved by the passing BUG-282 negative
control. The mapped combined setup/pre-compaction scenario tests construction
only; tighten its error assertion and add pre-compaction failure before claiming
full coverage. When the receipt API changes, replace the current archive helper
and message-count barrier with receipt-aware retrieval and deterministic Pending
barriers without weakening cancellation/recoverability assertions.

No full repaired admission, automatic-compaction composition, later crash-window
recovery, remote exactly-once guarantee or deployed/live verification is claimed.
All captured production hashes and protected BUG-283 hashes remain unchanged.
Live PID 922084 still hashes to
`d2c910442adbe130e69e921b151c4613b6fb445f87e05ecd3857f83c7064eb4a`.
No original failed history, provider invariant, concurrent production interface,
native session capacity, installed executable, commit or branch was changed.
