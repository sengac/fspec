# REFAC-044 AST research

## grep -n 'include!' source_shape_rpc018.rs source_shape_rpc022.rs (before)
source_shape_rpc018.rs:265:include!("source_shape_rpc018/surface_invariants.rs");
source_shape_rpc022.rs:65:include!("source_shape_rpc022/provider_service.rs");

## Coverage before
rpc018-source-shape testFiles: source_shape_rpc018.rs, source_shape_rpc018/surface_invariants.rs
rpc022-source-shape testFiles: source_shape_rpc022/provider_service.rs, source_shape_rpc022.rs

## Helper identity check
The python comparison of `fn src_dir` .. `fn count_lines_path` and `fn read_tree` in both files reported them identical, so a single shared support module replaces both copies.
