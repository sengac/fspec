# BOARD-024 final evidence-only handoff

## Status and authorization

Canonical status: **testing**, not implementing or Done. Production remains
paused pending the user's refactor/minimal-wiring decision. Shared dispatch
ownership is resolved, but grants no broader production approval. Original
history, documents and receipts are retained; cancelled reviewer follow-ups were
not implemented. No further review agent or task was started.

## Authoritative RED

Reused `/tmp/board024-red-mux-review.log`: all seven current BOARD-024 Rust source
files, the reused intent fixture and common Rust fixtures predate that completed
run. The source-newer check returned no paths. No test rerun was necessary or run
during this evidence-only finalization.

The final raw attachment `board024-final-red.log` is a byte-identical copy of
that receipt. SHA256 of both original and final copy:

`f32e3a0d903f842d82a72656c32ac3116682d41e368879ae9fc6a2bc25dc351f`

Original command, from `rust/`:

```bash
CARGO_NET_OFFLINE=true CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_CI_TEST_DEBUG=0 cargo test --profile ci-test -p codelet-fspec-tui --test board024_enter_chooser -j 2 -- --test-threads=1
```

```text
Finished `ci-test` profile [unoptimized] target(s) in 6.64s
test result: FAILED. 1 passed; 16 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
Cargo exit status: 101
```

The empty-board control passes. Fifteen failures establish missing chooser entry
through actual App Board/Mux key events; one establishes the missing footer hint.
Deeper canonical loading, race rejection, scrolling, attachment and corrected
post-confirmation Mux assertions are not reached. The WebSocket iteration also
remains unreached behind embedded chooser setup. No deeper causal RED, GREEN,
full quality clearance or completion is claimed. Existing fixture reuse emits
eight dead-code warnings; no lint suppression was added.

## Mapping and integrity verification

Supported show-coverage readback confirms all 17 actual scenarios. An additional
read-only integrity check verifies exact feature/coverage scenario-name equality,
one test mapping per scenario, zero implementation mappings and no placeholder.
Supported show-work-unit readback confirms testing and registered evidence.

- `board024-final-sources.sha256`: current BOARD-024 feature, coverage sidecar,
  root integration target and six Rust scenario/helper modules.
- `board024-final-evidence.sha256`: every other registered attachment at
  finalization, including this handoff, the final raw log and source manifest.
  The evidence manifest excludes itself to avoid an impossible self-referential
  digest. Paths are repository-relative; verify with `sha256sum -c` from repo root.

## Deferred implementation-phase test weaknesses

- Canonical fixture shapes still need the cancelled review's corrections:
  architectureNotes record objects, stateHistory state/timestamp/reason, legacy
  statusHistory from/to, and eventStorm items/level/nextItemId. Unusual shapes
  should be separate unknown/legacy cases, not representative current fields.
- Add omitted assumptions, blockedBy and relatesTo to canonical completeness
  fixtures/assertions before claiming all-applicable-fields behavior is proven.
- Once typed detail completions exist, assert captured response kind, card and
  generation explicitly; holding arbitrary queued actions is not sufficient proof.
- Reconcile selection-marker/full-path row assumptions with the actual renderer;
  exercise real viewer/browser failure separately from unavailable-viewer behavior.
- Execute and review every currently blocked deeper assertion after chooser entry
  exists; test-mapping coverage is not evidence that those assertions passed.

The only remaining approval is the six-file integration-only oversized-file
exception in `board024-narrow-integration-proposal.md`, or separately authorized
preparatory refactor design/scope. This handoff grants neither. Session work ends
after attachment, checksum and canonical readback verification.
