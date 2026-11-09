# AUDIT-003 AST research (2026-11-07)

```
rust/fspec-core/src/dispatch.rs:567:33:commands::audit_coverage::run(args_json, project_root)

rust/fspec-core/src/dispatch.rs:569:24:commands::check::run(args_json, project_root)

No matches found

No matches found

No matches found

No matches found

rust/fspec-core/src/commands/audit_coverage_drift.rs:91:37:missing_steps(&steps, &text)
rust/fspec-core/src/commands/link_coverage/step.rs:81:19:missing_steps(&feature_steps, &linked)

```

```
rust/fspec-core/src/commands/audit_coverage.rs:154:18:crate::commands::audit_coverage_drift::drifted_mappings(

rust/fspec-core/src/commands/audit_coverage.rs:185:9:crate::commands::audit_coverage_impl::impl_range_problems(project_root, &coverage)

rust/fspec-core/src/commands/update_work_unit_status/step_docstrings.rs:115:22:crate::commands::coverage_sharing::shared_test_files(
rust/fspec-core/src/commands/audit_coverage.rs:166:18:crate::commands::coverage_sharing::shared_test_files(

rust/fspec-core/src/commands/validate.rs:362:33:coverage_drift::coverage_drift(project_root, file_path, &feature)

rust/fspec-core/src/commands/link_coverage/step.rs:65:22:crate::commands::audit_coverage_drift::line_ranges(lines)

rust/fspec-core/src/commands/link_coverage/step.rs:119:1:fn extract_step_comments(test_content: &str) -> Vec<StepComment> {

```

Conclusion: the coverage checks enter only through audit_coverage::run (dispatch.rs "audit-coverage"), which calls drifted_mappings (BUG-375/501/525), impl_range_problems (BUG-515) and shared_test_files (BUG-357, which re-reads every sidecar per call). check::run (dispatch.rs "check") re-implements the Gherkin parse and calls validate_tags::run through its JSON envelope. validate.rs owns the per-file Gherkin check, the BUG-485/557 coverage drift (validate_coverage_drift.rs) and the BUG-522 orphan list. line_ranges and missing_steps are shared with link-coverage's step check, so the @step comment-marker rule lands once in link_coverage/step.rs::extract_step_comments. fspec audit becomes one core in commands/audit/ calling these functions with a sidecar index read once.
