# BUG-271: unreadable historical session manifests block all sessions

## Report and confidence

The reported Linux log at `2026-10-26T21:53:16.134054606+00:00`
identifies `/home/rquast/.fspec/sessions/b8c05474-d2ef-4bb9-83f6-899278219946.json`
and `EOF while parsing a value at line 1 column 0`. Session creation for
`openai:sglang/qwen3.8-27b` returns an empty session ID.

The failure path is source-confirmed. The remote manifest has not been inspected
and no runtime reproduction was performed during the initial investigation.
EOF at the start suggests empty or whitespace-only input; it does not establish
that an old schema is incompatible. Preserve the original file and obtain its
size, redacted structural information if nonempty, and old/new fspec versions.
Do not collect conversation content or credentials.

## Root cause

1. `rust/sessions/src/handle_impl.rs:139` invokes session creation.
2. `rust/sessions/src/session_manager.rs:839` persists the new manifest.
3. `rust/core/src/persistence/manifest_registry.rs:103` lazily initializes the
   global store before saving. Initialization at line 18 constructs a new store.
4. `rust/core/src/persistence/manifest_store.rs:31` calls `load_all`, which reads
   every user-global `sessions/*.json`. Lines 47–50 propagate the first failure.
5. One damaged historical file, even for another project, prevents store
   initialization. Later operations retry the same failing scan. The UUID in
   the error need not be the UUID of the newly requested session.

## Misleading UI failure

`rust/sessions/src/handle_impl.rs:142` turns all creation failures into an empty
ID. `rust/fspec-tui/src/app/session_creation.rs:31` interprets that as
`SessionCreationDeclined`. The dialog at
`rust/fspec-tui/src/app/dispatch_create_session_dialog.rs:217` says no default
model is set. A genuine storage problem therefore appears to require selecting
a model again. Diagnostics must also reach users with no open session.

## History and existing contract

`git show 2699693b -- rust/core/src/persistence/manifest.rs` changes discovery
from warning/continuing on incompatible JSON to propagating `read_manifest`
errors. Commit `f0eb0759` subsequently moves this loader to `manifest_store.rs`.
This confirms an availability behavior change, not why the remote file is empty.

`spec/features/durable-session-manifest-transactions.feature:51` and
`rust/core/tests/cmpct064_manifest_transactions.rs:248` explicitly require the
whole store to fail on one unreadable manifest. Revise this contract deliberately
to explicit per-session failure plus independent healthy-session availability.
Do not merely delete corruption checks. Preserve strict target-session replay,
revision conflicts, locks, atomic promotion, and lost-acknowledgement behavior.

Existing CMPCT-064 and PROV-101 coverage reports were reviewed. Coverage metadata
includes stale pre-split source anchors and is not evidence of a fresh test run.

## Compatibility and data safety

`rust/core/src/persistence/manifest_types.rs` already defaults revision, provider,
token usage and newer optional fields. Supported older files should remain
readable without discovery rewriting them. Empty/truncated JSON, missing required
fields and unsupported nested state are distinct from defaultable legacy fields.

`rust/core/src/persistence/message_cleanup.rs:50` and
`rust/core/src/persistence/messages/framing/references.rs:8` independently parse
all manifests to prove committed message references. They conservatively fail
on incomplete inventory. Preserve that safety boundary: unreadable is not absent.
Moving damaged files out of `sessions/*.json` without retaining unknown-reference
protection can permit accidental deletion. Do not auto-delete, replace with an
empty session, salvage a transcript, or mutate provider/recovery data.

## Proposed repair boundary

- Discover healthy sessions while retaining explicit path/identity/cause for
  unreadable entries; leave original files in place.
- Keep targeted load/resume/update failures strict and actionable.
- Surface actionable warnings without requiring a current session.
- Keep real directory access, write, ownership and promotion failures as errors;
  distinguish them from a genuinely missing default model.
- Preserve conservative cleanup/tail-repair behavior when references are unknown.
- Permit recovery on a subsequent cold load after an original is safely repaired.

## Acceptance and test-first handoff

The card records six rules and seven concrete examples covering empty files,
cross-project isolation, strict target-session failure, legacy defaults,
preservation under cleanup, real persistence errors, and repeated cold loads.

Write/update feature scenarios first, map integration tests 1:1, demonstrate
executable failures, then implement minimally. Use real temporary stores from
`rust/test-helpers`, with `serial_test` for process globals. Exercise the real
SessionManager/handle/TUI boundary, not just pure empty-ID mapping. Run scoped
`cargo test -p <crate> --test <target>` commands, never workspace tests. Keep each
new/modified file under 300 lines and request approval for major refactoring.

## Initial investigation validation

Only `spec/work-units.json` changed during card creation. `git diff --check`
passed. No production implementation, executable reproduction, or test run was
claimed. The user subsequently authorized starting work on BUG-271.
