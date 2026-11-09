# BUG-389 root cause

`source_shape_rpc054::every_file_under_provider_settings_stays_under_300_lines` fails at HEAD (e351762be). It is a
per-file size ceiling, but REFAC-021 (724e13e2d) mechanically swapped its `std::fs::read_to_string` for
`codelet_test_helpers::source_composition::read_tree_or_file`, which appends every out-of-line child module:

- `mod.rs` (299 lines) is the module root, so its tree is the whole directory (about 5,000 lines);
- `detail.rs` (280 lines) declares `#[cfg(test)] #[path = "detail_tests.rs"] mod tests;`, so its tree is 330 lines.

No file is actually 300 lines or more; the ceiling measures the wrong text.

Fix: the ceiling reads each file with `read_source` (the file plus any `include!`d fragments, so an include split
cannot dodge the ceiling) instead of its module tree. A first attempt moved `render_detail` out of `detail.rs`; it was
reverted once the composed `mod.rs` tree showed the measurement itself was wrong.
