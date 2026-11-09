# REFAC-058 AST research

- `diff` of the helper blocks (rpc073 HEAD 43-146 vs rpc070 HEAD 60-176): only comment lines differ.
- After extraction the tests call `read_handle_impl_src`, `strip_comments` and `extract_method_body`;
  `workspace_root` and `handle_impl_path` are used only inside the support module (private there).
- Noticed, filed separately: the runtime scenarios build `SessionManager::new()` without redirecting the data
  directory, so they read (and can write) the developer's real fspec data directory.
