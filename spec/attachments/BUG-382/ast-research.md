# BUG-382 AST research

- `cargo fmt --all --check 2>&1 | grep '^Diff in' | sort -u` -> 80 files (list captured during BUG-379; re-run to
  refresh before fixing).
- `grep -n "fmt" AGENTS.md` -> only `cargo fmt -p codelet-fspec` under "Common Build Commands"; the quality gate
  section has no format check.
- No `rustfmt.toml` in `rust/`; edition 2021 defaults apply.
