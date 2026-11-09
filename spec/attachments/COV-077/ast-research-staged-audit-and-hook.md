# COV-077 AST research (2026-11-07)

```
## Sidecars::load($$$A)            (rust/fspec-core/src/commands/audit.rs)
rust/fspec-core/src/commands/audit.rs:346:20:Sidecars::load(project_root)
## glob_feature_files($$$A)
rust/fspec-core/src/commands/audit.rs:341:44:glob_feature_files(project_root)
## std::fs::read_to_string($$$A)   (audit.rs and audit/)
rust/fspec-core/src/commands/audit.rs:240:24:std::fs::read_to_string(&path)
rust/fspec-core/src/commands/audit.rs:379:13:std::fs::read_to_string(project_root.join(&feature_file))
rust/fspec-core/src/commands/audit.rs:563:15:std::fs::read_to_string(&path)
rust/fspec-core/src/commands/audit.rs:585:27:std::fs::read_to_string(file)
rust/fspec-core/src/commands/audit/fix.rs:100:19:std::fs::read_to_string(&path)
rust/fspec-core/src/commands/audit/drift.rs:65:32:std::fs::read_to_string(&test_path)
rust/fspec-core/src/commands/audit/impl_ranges.rs:176:21:std::fs::read_to_string(&path)
```

Callers (grep): drifted_mappings and impl_range_problems are called only from
audit/coverage.rs; coverage_sharing::coverage_files only from audit.rs
Sidecars::load; validate_tags::load_registry/load_work_units read spec/tags.json
and spec/work-units.json from disk (ensure_tags_file creates tags.json when missing).
init.rs install_agent_files and sync_version.rs on_match/on_mismatch are the
install points (reinstall_agent_files is the --sync-version path).

Conclusion: every read the audit makes goes through a handful of call sites in
commands/audit*, so one file source (working tree or git index) threaded through
detect -> Sidecars::load / coverage::problems / drift / impl_ranges / relink, plus
validate_tags helpers that take parsed content, makes --changed read exactly the
staged snapshot. codelet-git gains StagedFiles (index paths + blobs, b8a500205);
staged A/M/D comes from get_staged_files_with_change_type. The hook is installed
next to the agent files in init.rs (new init/commit_gate.rs) and re-installed by
sync_version.rs; the hooks directory comes from `git rev-parse --git-path hooks`
(honours core.hooksPath and linked worktrees), as checkpoints.rs already shells to git.
