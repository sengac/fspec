# BUG-249/250 agent-loop caller completion audit

Status remains parent-coordinated; no commits or status changes by this caller
worker. Compaction settlement and core lifecycle ownership are not edited here.

## Executed evidence

- `/tmp/bug249-250-finish-integrated-direct-green.log`: 10 current-source caller
  tests pass; one subprocess-only helper is intentionally ignored by the parent
  runner and explicitly launched by its four actual process-kill tests.
- Process termination covers acknowledged prefix, unknown sentinel side effect,
  aborted/replacement response attempts, and an invocation stopped before dispatch.
- Actual executor admission still refuses side effects when journaling fails.
  Genuine interrupted running tools remain outcome-unknown. Full shared admission
  preserves the unpublished prefix as an authentic original once, per BUG-246.
- Terminal records remain scrubbed and separate from model originals. Sequential
  retry cannot recreate the transcript of a deleted session.
- `/tmp/bug250-deleted-recovery-red.log` is behavioral RED for that last caller
  gap. The shared spool commit now requires an existing owning manifest before
  transcript append. Its exact scenario lives in deleted-transcript-recovery.feature.
- The test original reader now follows exact manifest references rather than the
  archival display API, which correctly gained transcript records concurrently.
- Current-source Clippy with warnings denied and scoped formatting pass. Global
  Gherkin validation passes all 2102 features; changed feature tag checks pass.

Direct runs use real optimized integration executables against current production
source and matching Cargo dependency identities, with release-slim's opt-level 3,
16 codegen units and thin-LTO linking. Cargo profile/configuration is unchanged.

## Parent integration gates

Final Cargo commands remain queued in `/tmp/bug249-caller-finish-baseline.log`
and `/tmp/bug250-deleted-recovery-cargo.log`; these are not passing evidence.
BUG-246's seven-target final Cargo run and Cargo quality checks are also pending.

Core delete_owned now removes the transcript lane, but core lifecycle completion
still needs coordinated recovery-spool cleanup and atomic live-session append
ownership. The caller's sequential manifest check does not serialize a concurrent
delete/retry race. Avoid introducing inverse session/recovery lock acquisition.
Exact coordination is in `spec/attachments/BUG-246/coordination-contract.md`.
