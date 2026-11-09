//! Feature: spec/features/rpc025-fspec-binary-persistence-bootstrap.feature
//!
//! RPC-025 test, moved out of part_1.rs so each test file maps one feature (COV-075).

use super::*;

/// RPC-025 regression: the production `fspec` binary MUST call
/// `codelet_common::set_data_directory(~/.fspec)` BEFORE exposing
/// any persistence_*_history RPC method. Without this,
/// `HistoryStore::new()` returns `Err("Data directory not
/// initialized")` and `dispatch_rpc025` silently swallows the
/// error — making Shift+↑/↓ appear inert in the live binary even
/// though unit tests pass (because they call
/// `set_data_directory(temp.path())` themselves).
#[test]
#[serial]
fn build_service_initializes_global_data_directory_for_persistence() {
    // @step Given the rust/fspec binary crate after RPC-025 lands
    let tmp = tempfile::tempdir().expect("tempdir");
    let workspace = tmp.path();

    // @step When common::build_service is invoked against a tempdir workspace
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    let _service = build_service(workspace).expect("build_service");

    // @step Then codelet_common::get_data_dir() returns Ok with a path ending in ".fspec"
    let data_dir = codelet_common::get_data_dir()
        .expect("codelet_common::get_data_dir() must be Ok after build_service");
    assert!(
        data_dir.ends_with(".fspec"),
        "build_service must initialise the data directory to a `.fspec` path; got {}",
        data_dir.display()
    );

    // @step And rust/fspec/src/common.rs contains the substring "codelet_common::set_data_directory"
    let src = codelet_test_helpers::source_composition::read_tree_or_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/common.rs"
    ))
    .expect("read common.rs");
    assert!(
        src.contains("codelet_common::set_data_directory"),
        "common.rs must contain a literal `codelet_common::set_data_directory(...)` call (RPC-025 regression)",
    );

    // @step And the set_data_directory call appears BEFORE the WorkUnitsWatcher::new(workspace) call in build_service
    let set_idx = src
        .find("codelet_common::set_data_directory")
        .expect("set_data_directory call must exist");
    let watcher_idx = src
        .find("WorkUnitsWatcher::new(workspace)")
        .expect("WorkUnitsWatcher::new(workspace) call must exist in build_service");
    assert!(
        set_idx < watcher_idx,
        "set_data_directory must be called BEFORE WorkUnitsWatcher::new in build_service",
    );
}
