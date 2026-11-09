# BUG-384 root cause

`check_prefill` (`rust/fspec-core/src/commands/update_work_unit_status/prefill.rs`):

```rust
let features = linked_feature_files(project_root, id)...;
let Some((_, content)) = features.first() else { return Ok(()) };
let matches = detect_prefill(content);
```

`linked_feature_files` collects every top-level `spec/features/*.feature` whose text contains `@<id>`, in
`std::fs::read_dir` order. That order is platform- and filesystem-dependent, so when a work unit is tagged in more
than one feature file only one arbitrary file is checked and prefill placeholders in the others pass the gate.

## Fix direction
Run `detect_prefill` over every linked feature file and report the matches per file (path + line), with a test that
tags two feature files and puts placeholders only in the one `read_dir` would not list first (e.g. check both orders
by naming the files so either sorts first).
