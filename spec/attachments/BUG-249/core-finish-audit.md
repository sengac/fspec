# Core finish audit

Scope: BUG-247/248 durability/framing and BUG-249/250 core transcript lane only.
Existing shared changes are preserved; no commits or final BUG-249/250 transitions.

## New acceptance gaps under test

Added two scenarios to `transcript-journal-integrity.feature` before adding
`rust/core/tests/bug249_journal_admission.rs`: duplicate immutable record UUIDs
at another sequence/request must be rejected, and corrupt acknowledged JSON
must block further admission without changing existing evidence. The current
append path checks filenames but does not validate existing record bytes or
global record identity. Production is unchanged pending executable RED.

Added one scenario to `transcript-session-lifecycle.feature` and its real-session
regression before implementation: a failed private-lane deletion must preserve
the committed session for retry. Prior actual deletion RED already exists at
`/tmp/bug250-lifecycle-real-red.log`; the canonical hook remains absent from
`SessionStore::delete_owned` at audit start. Core ownership includes that narrow
hook; BUG-246 still owns pending spool and in-memory retirement. Parent message
attempts were rejected as busy and are not represented as delivered.

## Commands in progress

From `rust/`, using the existing target/profile and `-j 2`:

```sh
cargo test --profile release-slim -j 2 -p codelet-core --test bug247_message_durability --test bug247_stable_message_identity --test bug248_reference_evidence --test bug248_tail_recovery --test bug249_journal_integrity --test bug249_transcript_journal --test bug250_transcript_outcomes --test bug250_transcript_lifecycle --no-fail-fast
cargo test --profile release-slim -j 2 -p codelet-core --test bug249_journal_admission --no-fail-fast
cargo test --profile release-slim -j 2 -p codelet-core --test bug250_transcript_lifecycle
```

Logs are `/tmp/bug247-250-core-finish-tests.log`,
`/tmp/bug249-core-admission-red.log`, `/tmp/bug250-core-deletion-red.log`.
These initially wait behind an existing dependency rebuild; waiting is not GREEN.
A supplementary attempt to link only the real transcript source modules and
shared helper sources failed with incompatible LTO artifacts, so it provides no
behavioral evidence and does not authorize production changes.

Both changed feature files pass scoped format, syntax validation and tag checks.
New tests have scenario mappings. Final results and implementation mappings will
be appended after actual execution. BUG-247/248 remain validating until their
fresh scoped results and coverage/quality evidence are recorded.

## Executed admission RED and repair

After the shared Cargo build produced matching real core/helper libraries,
`python3 /tmp/bug249-core-admission-run.py core/tests/bug249_journal_admission.rs`
compiled the actual integration test against those full libraries (not the
failed source-module harness). `/tmp/bug249-core-admission-direct-red.log` records
**0 passed, 2 failed**: duplicate identity was acknowledged, and corrupt existing
history was acknowledged. These are actual filesystem assertion failures.

Only after that RED, `transcript/store.rs` shares its strict read/identity/sequence
validation with append under the same writer lease and rejects conflicting record
UUIDs before publication. No alternate store, fallback recovery, schema change,
or diagnostic payload is added. Existing history is read for validation, never
rewritten; storage amplification remains linear in admitted deltas. This extends
the existing per-admission filename scan with record validation, not a claim of
constant-time admission for arbitrarily long sessions.

Full core `cargo check` and strict core-lib Clippy are queued using the same
profile/config; logs are `/tmp/bug247-250-core-finish-check.log` and
`/tmp/bug247-250-core-finish-clippy.log`. Pending logs are not passing evidence.

## Executed deletion RED and owned GREEN

`python3 /tmp/bug249-core-admission-run.py core/tests/bug250_transcript_lifecycle.rs`
ran the actual test against the matching full core library before the deletion
repair. `/tmp/bug250-core-deletion-direct-red.log` records **1 passed, 2 failed**:
the transcript survived canonical deletion, and a blocked lane was ignored while
the manifest was removed. Only after this RED, `manifest_commit.rs` installs
private-lane deletion under the existing session ownership, before manifest
removal, deriving the data root from that store's own `sessions_dir`. A typed
`ManifestCommitError::Transcript` propagates failure without deleting the manifest.

The following supplementary full-source command now passes:

```sh
python3 /tmp/bug249-core-finish-run.py core/tests/bug249_journal_admission.rs core/tests/bug250_transcript_lifecycle.rs
```

`/tmp/bug249-250-core-finish-direct-green.log` records **2/2 admission tests and
3/3 lifecycle tests passed**. This compiles the entire actual core crate from
current sources with the exact dependency fingerprints produced by the existing
release-slim build, then links the tracked integration tests with the real shared
helper crate. It is not the failed earlier partial-module harness and contains no
mock store. The required canonical Cargo invocations remain queued separately;
their lock-wait logs are not claimed to have executed tests.

All four journal-integrity scenarios and all three session-lifecycle scenarios
have one test mapping each. Scoped coverage-path audits and rustfmt checks pass.
BUG-246 pending spool/memory retirement and higher-layer replay/projection remain
with their owners. Both parent and BUG-246 owner message attempts returned busy;
neither was delivered. BUG-249/250 remain validating for parent coordination.

## Full scoped source regression results

The supplementary full-source runner also executes all seven remaining owned
targets against current full core sources:

```sh
python3 /tmp/bug249-core-finish-run.py core/tests/bug247_message_durability.rs core/tests/bug247_stable_message_identity.rs core/tests/bug248_reference_evidence.rs core/tests/bug248_tail_recovery.rs core/tests/bug249_journal_integrity.rs core/tests/bug249_transcript_journal.rs core/tests/bug250_transcript_outcomes.rs
```

`/tmp/bug247-250-core-finish-direct-regression.log` records **28/28 passed**:
5 durability, 4 stable identity, 2 reference evidence, 7 tail recovery, 2 journal
integrity, 4 transcript journal, and 4 terminal outcomes. Together with the prior
five admission/lifecycle cases this is **33 distinct passing core tests**.
Unicode truncation/property recovery took 138.64 seconds. No power-loss claim is
made. The core source compiler passes as part of this runner.

`python3 /tmp/bug249-clippy-direct.py core codelet_core` independently checks the
entire current core crate with the matching release-slim dependency fingerprints
and `-D warnings`: exit **0**, explicitly captured in
`/tmp/bug247-250-core-finish-direct-clippy.log`. This is supplementary compiler
and Clippy evidence, not a claim that the queued Cargo commands completed.
Scoped coverage audits find all 20 BUG-247 paths and all 18 BUG-248 paths; all
four relevant feature files pass syntax/tag checks and changed Rust passes
rustfmt. Canonical repeat logs still contain only lock waits at this recording.

BUG-247 and BUG-248 are now **done** after serialized transitions backed by their
verified prior canonical results and fresh **9/9** full-source tests per card.
Their completion audits explicitly distinguish pending fresh Cargo repetitions
from actual executed evidence. BUG-249 and BUG-250 remain **validating**; this
worker does not claim their higher-layer caller, spool-retirement, archive or
replay integrations are complete. No HEAD commits were made.
