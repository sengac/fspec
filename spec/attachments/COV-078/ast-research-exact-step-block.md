# COV-078 AST research (2026-11-07)

```
## replace_test_mapping($$$A)
rust/fspec-core/src/commands/link_coverage.rs:253:28:replace_test_mapping(entry, tf, tl)

## extract_step_comments($$$A)
rust/fspec-core/src/commands/link_coverage/step.rs:94:20:extract_step_comments(test_text)
rust/fspec-core/src/commands/link_coverage/step.rs:315:24:extract_step_comments(
rust/fspec-core/src/commands/link_coverage/step.rs:325:24:extract_step_comments(

## missing_steps($$$A)
rust/fspec-core/src/commands/link_coverage/step.rs:81:19:missing_steps(&feature_steps, &linked)
rust/fspec-core/src/commands/audit/drift.rs:114:37:missing_steps(&steps, &text)

## fix::apply($$$A)
rust/fspec-core/src/commands/audit.rs:290:17:fix::apply(project_root, &problems)

## get_untracked_files($$$A)
rust/fspec-core/src/commands/restore_checkpoint.rs:312:21:get_untracked_files(project_root)
```

Conclusion: the @step extractor (link_coverage/step.rs extract_step_comments) is the one matcher both link-coverage and fspec audit's drift check use (via missing_steps); the exact-block search belongs next to it so it reads comments the same way, extended with each comment's first and last line. The BUG-502 replace operation (link_coverage/mapping.rs replace_test_mapping) is the only writer that swaps a scenario's test ranges while keeping implMappings, so --fix reuses it (with an optional new file for moves). fspec audit --fix (audit/fix.rs, COV-079) already applies repairs per coverage file inside SidecarTransaction; COV-078 adds test-mapping, stale-entry and orphan-coverage repairs there. Added files come from codelet-git status (get_untracked_files, get_staged_files_with_change_type Added), as restore_checkpoint already uses.
