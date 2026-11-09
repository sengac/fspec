# REFAC-055 AST research

- Helpers in the test file: lines 18-60 at HEAD (one static, four fns); each is used only by this file's three tests.
- Imports the helpers need: `std::path::PathBuf`, `codelet_rpc_types::SessionInfo`, `tokio::sync::Mutex`.
- The test bodies still use `Arc`, `SessionManagerHandle`, `SessionManager` and `SessionInfo`.
