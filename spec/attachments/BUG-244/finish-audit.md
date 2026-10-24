# BUG-244 / BUG-245 finish audit

Follow-up repairs and executable evidence are tracked in
`remaining-repair-evidence.md`; the historical pending items below are not the
current implementation status.

Owner: c38f004d-db70-4e9f-b035-906879bae41b. Shared dirty changes preserved.

Read AGENTS.md, repair plans, existing implementation/coordination evidence,
features, coverage, prior work and scoped tests before modifications.

Confirmed remaining gaps:

- BUG-245 archive inspector was absent. Existing executable RED is
  `/tmp/bug244-245-tui-pre-inspector.log`, missing `/archive` trim notice.
  Inspector implementation now uses the same bounded page/assembler contract,
  scoped nonce/cancellation, complete turn-inspector payload and explicit errors.
- BUG-245 compaction metadata/boundary was absent. Added a scenario and regression;
  `/tmp/bug245-compaction-marker-red.log` is queued. No marker production changes
  before executable RED.
- Further read-only audit identifies history inspection accepting a replacement
  generation, historical rendering clearing current provider-waiting state, and
  periodic polling retaining completed task handles. Feature-first regressions
  are queued in `/tmp/bug244-245-audit-red.log`; no corresponding fixes before RED.

Validation uses the existing release-slim artifacts and `-j2`, without alternate
profiles, rustflags or target directories. Current full rebuild occupies the
shared artifact lock. The previous owner's delivery job
`/tmp/bug244-live-init-green.log` failed on an absent dependency artifact:
`liblalrpop-990fe1e6d25803cb.rlib` while compiling merman-core's build script.
This is not behavioral RED or GREEN. Combined TUI validation is queued in
`/tmp/bug244-245-finish-tui.log`.

No edits to shared sessions handle_impl.rs, session_restore.rs, replay_content.rs,
replay_media.rs, background_session.rs or delivery journal. Future marker wiring
is restricted to the assigned archival_replay.rs projection.

Two supervisor messages returned session_busy and were not queued. Supervisor
can read this handoff without waiting for an idle messaging window.

Scoped rustfmt and `git diff --check` pass for the assigned files. Global
`fspec validate-tags` reports 526 pre-existing violations; none mention the five
BUG-244/245 feature files. Full output: `/tmp/bug244-245-finish-tags.log`.
Stale/duplicate TUI and sessions coverage line mappings are being reconciled
through the serialized fspec wrapper; new failing scenarios intentionally have
no implementation mapping until the executable RED gate has completed.

Additional shared-sessions handoff requiring coordination: `manifest_restore`
is set true by cold creation but is never cleared after successful resume
(only by clear_history). Consequently a previously cold-restored session still
reinstalls saved token counters on every later manual resume, even after new live
work. A feature-first real regression has been added to the already queued
`bug244_rendered_transcript` target. Expected repair: consume the cold restore
requirement only after model and token restoration both succeed, preserving it
on failures. This requires BackgroundSession API plus resume orchestration edits,
outside this worker's exclusive TUI/archival projection ownership. Supervisor or
delivery owner must coordinate before those files are modified.

Executable confirmation: `/tmp/bug244-245-finish-tui.log` now runs the rendered
target, with 4 passed and 1 failed. The new cold-restored active-session regression
fails its actual counter assertion: observed input tokens 0, expected 317.
The original four rendered delivery regressions remain GREEN. Because Cargo
stops at that failing target, this run does not claim history-inspector or the
two pre-existing resume targets have executed. The supervisor message requesting
the narrow shared initialization completion fix again returned session_busy.

The test fixture's `seed`/`cold_restore` methods were extracted unchanged into
`rust/fspec-tui/tests/bug244/history_fixture.rs`, imported only by targets that
use them, after the compiler reported unused-method warnings in the inspector
target. No warning suppression was added. Coverage mappings were refreshed to
the resulting test line numbers.

Still not implemented, pending executable RED: compaction marker projection,
inspector generation anchoring, historical provider-waiting isolation, completed
polling-task reaping. Still outside assigned exclusive ownership: consuming
successful shared cold initialization. Neither BUG-244 nor BUG-245 is done or
validating. All pending jobs retain shared release-slim/-j2 configuration.

Pending commands:

```
cargo test --profile release-slim -j2 -p codelet-sessions \
  --test bug245_archive_model_separation \
  compaction_metadata_marks_the_covered_original_boundary
cargo test --profile release-slim -j2 -p codelet-fspec-tui \
  --test bug244_replay_state --test bug245_history_access --no-fail-fast
```

Their logs are `/tmp/bug245-compaction-marker-red.log` and
`/tmp/bug244-245-audit-red.log`. The original TUI run compiled the inspector and
pre-existing resume targets but stopped execution after its rendered RED;
rerun those targets after repairs, rather than claiming they are GREEN.
