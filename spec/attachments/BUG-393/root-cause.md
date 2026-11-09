# BUG-393 root cause

REFAC-021 (724e13e2d) replaced `std::fs::read_to_string` with
`codelet_test_helpers::source_composition::read_tree_or_file` in source-shape tests across crates so a module split
into files still reads as one text. That is right for "does this module contain X" checks and wrong for per-file
line ceilings: a module root then measures its whole directory and a file with an out-of-line `mod tests;` measures
its tests too. BUG-389 found this in `source_shape_rpc054` (mod.rs measured ~5,000 lines). Ceilings that still pass
may be measuring the wrong text too, and a ceiling that measures a tree can fail for files that are each small.

Fix direction: for each test that enforces a per-file line ceiling, read the file with `read_source` (file plus
`include!`d fragments); keep `read_tree_or_file` for content checks.
