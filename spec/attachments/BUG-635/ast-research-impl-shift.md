# AST research: BUG-635 audit --fix implementation shift

## fspec astgrep --pattern 'fn shifted($$$) -> $R { $$$ }' --lang rust --path rust/fspec-core/src/commands/audit
```
rust/fspec-core/src/commands/audit/impl_ranges.rs:97:1:fn shifted(text: &[&str], lines: &[i64], first_line: &str, last_line: &str) -> Option<Vec<i64>> {

```

## fspec astgrep --pattern 'ImplRepair::Shift { $$$ }' --lang rust --path rust/fspec-core/src/commands/audit
```
rust/fspec-core/src/commands/audit/impl_ranges.rs:222:42:ImplRepair::Shift { new_lines }

```

## fspec astgrep --pattern 'history.verify($$$)' --lang rust --path rust/fspec-core/src/commands/audit
```
rust/fspec-core/src/commands/audit/impl_ranges.rs:239:31:history.verify(project_root, coverage_file, &mapping, content)

```

## fspec astgrep --pattern 'fn linking_commit($$$) -> $R { $$$ }' --lang rust --path rust/fspec-core/src/commands/audit
```
rust/fspec-core/src/commands/audit/history.rs:97:5:fn linking_commit(

```

## Findings

- impl_ranges.rs `shifted`: --fix moves every line of a text-changed mapping by one offset (s - first) when firstLine occurs once at s and the line at the same offset from s reads lastLine. Lines inserted inside a range never extend it, and a '}' at the old offset passes the check, so the repaired range ends one line short (the BUG-635 report).
- history.rs already finds the commit that linked a mapping (`linking_commit`, the oldest commit from which every later coverage version holds the same mapping) and reads a file at a commit (`file`), for BUG-591. The carry reuses both and adds `git diff -U0 --diff-algorithm=myers <commit> -- <file>` (`--cached` under `--changed`, where the audit reads the index).
- fix.rs applies `ImplRepair::Shift { new_lines }` unchanged; only the computation of new_lines changes.
- COV-079's cov079 test "a mapping whose code moved unchanged is shifted by fix" links without committing; under the carry it must commit the mapping first. "moved and grew is not shifted" (uncommitted) stays reported.
