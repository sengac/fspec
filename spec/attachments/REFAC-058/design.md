# REFAC-058 design

rust/fspec/tests/rpc070_create_session_no_panic.rs and rpc073_clear_history_no_panic.rs each carried the same five
helpers (`workspace_root`, `handle_impl_path`, `read_handle_impl_src`, `strip_comments`,
`extract_method_body`; identical code, different comments). They move once into
`tests/handle_impl_source_support/mod.rs` (the RPC-070 copy, which has the fuller comments); both test files declare
the module and import the three helpers they call. The long module docs are condensed to their substance. Tests are
unchanged: rpc070 is 299 lines, rpc073 297. Both features keep one test file; their test mappings are remapped.
