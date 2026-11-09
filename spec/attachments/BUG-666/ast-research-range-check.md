# AST research: BUG-666 range drift under matching anchors

## fspec astgrep --pattern 'history.carry($$$)' --lang rust --path rust/fspec-core/src/commands/audit
```
rust/fspec-core/src/commands/audit/impl_ranges.rs:212:27:history.carry(

```

## Findings

- impl_ranges.rs `impl_range_problems`: a mapping whose recorded firstLine/lastLine still read at its mapped first/last line is skipped (`continue`) before any git check; only a text mismatch reaches `History::carry` (BUG-635). That `continue` is where in-range growth under matching anchors is missed.
- history.rs: `linking_commit` walks `git log --first-parent -- <coverage file>` (one process per coverage file) and reads each version with `git show <commit>:./<file>` (one process per blob). Fine for the few text-changed mappings; too many processes if every mapping is checked (13.8k mappings over 2456 coverage files, 1302 commits touch spec/features).
- Plan: batch git once per run — one `git log --first-parent --format=%H --name-only` (every path's commits, newest first), one long-lived `git cat-file --batch` for file versions, and `git diff -U0` per linking commit over all files needing it; a mapping whose file has no commit after its linking commit and is unchanged in the source read (working tree or index) is skipped without a diff.
- carry.rs `carry` gives the carried lines; equal to the mapped lines means the range did not change.
