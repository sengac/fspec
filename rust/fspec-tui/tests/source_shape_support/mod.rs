//! Path and source-reading helpers shared by the RPC-018 and RPC-022
//! source-shape tests (REFAC-044).

use crate::common;
use codelet_test_helpers::OrFail;
pub fn src_dir() -> Result<std::path::PathBuf, codelet_test_helpers::TestError> {
    Ok(common::workspace_root()?.join("fspec-tui").join("src"))
}

pub fn rpc_types_lib() -> Result<std::path::PathBuf, codelet_test_helpers::TestError> {
    Ok(common::workspace_root()?
        .join("rpc-types")
        .join("src")
        .join("lib.rs"))
}

pub fn rpc_lib() -> Result<std::path::PathBuf, codelet_test_helpers::TestError> {
    Ok(common::workspace_root()?
        .join("rpc")
        .join("src")
        .join("lib.rs"))
}

pub fn core_session_manager_handle() -> Result<std::path::PathBuf, codelet_test_helpers::TestError>
{
    Ok(common::workspace_root()?
        .join("core")
        .join("src")
        .join("session_manager_handle.rs"))
}

pub fn read_raw(path: &std::path::Path) -> Result<String, codelet_test_helpers::TestError> {
    if [
        "fspec-tui/src/components/mod.rs",
        "fspec-tui/src/transport/mod.rs",
        "fspec-tui/src/transport/embedded.rs",
        "fspec-tui/src/transport/websocket.rs",
        "fspec-tui/src/views/navigator_mux_events.rs",
        "rpc/src/lib.rs",
    ]
    .iter()
    .any(|entry| path.ends_with(entry))
    {
        return Ok(codelet_test_helpers::source_composition::read_source(path)
            .or_fail("resolve composed source entrypoint")?);
    }
    common::read_to_string_or_fail(path)
}

/// BUG-371: the file's own physical lines (child modules are separate files
/// and are counted on their own).
pub fn count_lines_path(path: &std::path::Path) -> Result<usize, codelet_test_helpers::TestError> {
    Ok(std::fs::read_to_string(path)
        .or_fail("read physical source for line count")?
        .lines()
        .count())
}

/// REFAC-021/022: read a source file together with its include!d fragments
/// and file-backed child modules (split modules read as one text).
pub fn read_tree(path: &std::path::Path) -> Result<String, codelet_test_helpers::TestError> {
    Ok(
        codelet_test_helpers::source_composition::read_module_tree(path)
            .map_err(|error| format!("read {}: {error}", path.display()))?,
    )
}
