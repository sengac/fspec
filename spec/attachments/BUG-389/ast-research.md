# BUG-389 AST research

- `grep -n "mod " rust/fspec-tui/src/views/provider_settings/detail.rs` → `#[path = "detail_tests.rs"] mod tests;`.
- `grep -rn "render_detail" rust/fspec-tui/src` → defined in `detail.rs:207`, called only from `body_render.rs:33`;
  `status_text.rs` mentions it in a doc comment.
- `wc -l` → `detail.rs` 280, `detail_tests.rs` 48, `body_render.rs` 129.
- `grep -n "pub fn" rust/test-helpers/src/source_composition.rs` → `read_source` (file + include! expansion),
  `read_module_tree` and `read_tree_or_file` (whole module tree).
- `git show 724e13e2d -- rust/fspec-tui/tests/source_shape_rpc054.rs` → the REFAC-021 swap to `read_tree_or_file`.
