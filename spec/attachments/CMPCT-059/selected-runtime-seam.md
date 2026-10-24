# Selected runtime seam — APIs on disk

CLI exclusive owner 68ffd563; current session 3ff18601.

The exact public contract now implemented in `codelet_cli::session::Session`:

```rust
pub fn install_selected_preparer(&mut self, preparer: SelectedRequestPreparer);
pub fn commit_selected_compaction(
    &mut self,
    source: CompactionSource,
    operation_id: Uuid,
    content: &str,
    publish: impl FnOnce(&CommittedGeneration) -> Result<(), String>,
) -> Result<SessionManifest, CompactionCommitError>;
```

Use `capture_durable_compaction(id)` before builder execution and pass its exact
source to commit. The preparer is attached at actual stream entry using
`RigAgent::selected_request_preparer`; explicit gate fixtures may attach the
same actual selected agent preparer. Absence or changed provider/model fails
closed. No caller-supplied empty obligation map exists: central commit collects
runtime and persisted structural annotations and retains mandatory reminders.

`run_compactor_sub_agent_round` invokes this seam and only publishes success
after exact durable promotion, provider installation and reconciliation.

Current retained behavioral RED: `/tmp/cmpct-runtime-audit-red.log` (six failed
targets;079 negative controls green). Obligation-specific RED completed in the
shared ci-test cache: `/tmp/cmpct-runtime-obligations-red.log` (two dropped-file
failures, two passing durable installation/publication controls). Deadline retry
RED is queued as `/tmp/cmpct-runtime-deadline-red.log`. No completion claimed.

Recovery checkpoint: both APIs are present on disk and the first post-fix run
`/tmp/cmpct-runtime-selected-green1.log` passes052(5),057(4),058(2),059(4),079(4).
071 has3/4 passing; missing-capacity fixture erroneously required ordinary
inference with unavailable limits and is being corrected to attach the actual
selected preparer directly for that negative row.082 has3/4 passing; provider
unknown-limit defaults remain the fourth failure. Reminder quote/mixed-content
behavioral tests are being added before further preservation changes.
Loop/gate may call the published APIs immediately; this is not completion proof.

Sessions handoff: CLI now implements
`restore_compaction_boundary(&SessionManifest) -> Result<(), CompactionCommitError>`.
Call before replay. Validates originals/boundary, checks selected identity,
installs checkpoint, restores manifest occupancy and all saved spend counters,
owns pending identity only while publication_pending, and blocks inference until
exact reconciliation. Reconciliation now restores tokens from the latest durable
manifest, not stale boundary estimates. Existing behavioral RED:
`/tmp/cmpct068-pending-completion-red2.log` and pending-projection-red.log.
Sessions owner must wire this method in its owned resume path; CLI cannot edit it.
