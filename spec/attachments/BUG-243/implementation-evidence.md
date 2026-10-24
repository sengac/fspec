# BUG-243 implementation evidence

## Ownership and integration

Architecture authorization is recorded in
`spec/attachments/BUG-242/architecture-approval.md`. Shared-path coordination is
recorded in `implementation-coordination.md` and the BUG-243 architecture notes.
The archival contract was delivered to the BUG-244/245 owner before implementation.

The real CLI provider binder calls `migrate_provider_state` and
`persist_provider_state`. Core owns version-1 content-addressed checkpoint roots
and the optional manifest reference. Core replay selects verified native state
independently from conversation metadata. No MessageStore, conversation-commit,
manifest-commit, sessions rendering, or TUI source is edited by BUG-243.

Each root retains the latest checkpoint per provider generation and observed
reset provenance. A publication verifies committed state before removing owned
unreferenced roots. The current and displaced/pending roots plus a staging file
bound checkpoint retention; immutable source messages and retained generations
are not deleted. Native reasoning, protocol, model, generation, item IDs, call
IDs, and actual outcomes remain native checkpoint data.

Referenced state is authoritative. Missing roots, incompatible content identity,
generation, protocol, or source-prefix anchors fail explicitly. Stale or malformed
native metadata on genuine dialogue cannot override or invalidate the separate
checkpoint. Exact internal checkpoint positions expose bounded version-1 state
identity markers rather than fabricated dialogue.

`get_session_archival_page` is exported from `codelet_core::persistence`.
Each page contains at most one UTF-8-safe canonical-envelope fragment and stays
within its complete serialized byte budget, 1024..61440. Cursors pin a session,
revision, exclusive end, source-content prefix hash, position, and partial-record
identity. Appended suffixes do not enter a snapshot; changes under an existing
message ID invalidate it. Original structured fields and UUIDs survive
reassembly. Pre-envelope text has an explicit display projection; checkpoint
records expose only non-conversational identity markers. Model-context replay
still contains the committed summary and uncovered dialogue suffix.

## Feature-first executable RED

The specifications and integration tests precede production changes.
Nine scenarios are factored into six feature files, each linked to one test
source file. Included test modules execute through `bug243_provider_state`.
The captured-prefix cold-promotion regression also verifies a complete selected
provider/model identifier, conversational-only suffix replay, explicit checkpoint
markers, and unchanged stored originals.

1. `/tmp/bug243-red-confirmed.log` records the initial shared-core compilation
   blocker: `conversation_commit.rs` called missing `MessageStore::store_with_id`.
   This is not claimed as behavioral RED.
2. After the core owner supplied that prerequisite,
   `/tmp/bug243-red-runtime.log` records the executable checkpoint target:

   `cargo test --profile release-slim -j2 -p codelet-cli --test bug243_provider_state -- --nocapture`

   Result: **0 passed, 8 failed**. Failures expose internal checkpoints in
   originals, absent separate state references, conversation amplification,
   incorrect original-metadata authority, and absent checkpoint durability gates.
3. `/tmp/bug243-integrated-red.log` records the executable archival scenario
   against the initially bounded whole-record API. It fails with
   `OversizedRecord` at the 4096-byte budget. UTF-8 fragmentation is implemented
   after this failure, without increasing the response limit.

## GREEN

`/tmp/bug243-green.log`:

`cargo test --profile release-slim -j2 -p codelet-cli --test bug243_provider_state --test bug243_archival_pages --no-fail-fast -- --nocapture`

Result: **9 passed, 0 failed** (8 checkpoint scenarios and 1 archival scenario).

Coverage includes the real binder and SessionSearch originals tool; coordinate
and arbitrary-Unicode state round trips; cold native physical requests before
and after migration; interrupted staging; canonical-content and valid-CAS
wrong-prefix failures; missing referenced roots; genuine original typed errors;
stale/malformed native aliases; retained-generation/storage bounds; and all
seven ordered publication boundaries with explicit fault/retry behavior.

Archival coverage reassembles large UTF-8 dialogue, preserves original unknown
fields and identity, round-trips serialized cursors, excludes checkpoint payloads,
keeps reduced inference context, pins appended-session snapshots, validates page
budgets and cursor scope/version, and rejects changed or missing originals.

Durability evidence proves ordering and observed syscall/error behavior against
isolated real stores. It does not establish physical power-loss guarantees.

## Quality and handoff

All six feature files pass formatting, Gherkin validation, and tag validation:
`/tmp/bug243-spec-validation.log`.

`cargo check --profile release-slim -j2 -p codelet-core -p codelet-cli` passes:
`/tmp/bug243-check.log`.

Formatting checks and coverage audits pass for the six focused features plus the
captured-prefix regression: `/tmp/bug243-format.log` and
`/tmp/bug243-coverage-audit.log`.

The initial scoped core regression compilation is blocked by concurrent
agent-loop source changes, recorded in `/tmp/bug243-core-regressions.log`:
undefined `session`, `id`, and `call_id`, and misplaced/missing `journal` fields
in `background_output.rs` and `persist.rs`. BUG-243 does not edit those files.

The current final focused rerun and subsequent core regression retry are queued
on the shared release-slim artifact lock. Logs are
`/tmp/bug243-final-green.log` and `/tmp/bug243-core-regressions-final.log`.
The retry includes the complete provider/model ownership comparison used at a
committed generation. Scoped library Clippy is queued separately at
`/tmp/bug243-clippy.log`; it is not yet claimed as passing.

BUG-243 is **validating**, not done. No separate target directory, commit, or done
transition is created. Remaining shared-build results require coordinator review.
