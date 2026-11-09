//! Feature: spec/features/rpc017-build-service-cwd-attachment.feature
//!
//! RPC-017 test, moved out of part_1.rs so each test file maps one feature (COV-075).

use super::*;

/// RPC-017 regression: production `build_service` MUST attach the
/// workspace cwd to the SharedFspecService via `.with_cwd(...)`.
#[test]
#[serial]
fn build_service_attaches_workspace_cwd() {
    // @step Given the codelet-fspec binary crate after RPC-017 lands
    let tmp = tempfile::tempdir().expect("tempdir");
    let workspace = tmp.path();

    // @step When common::build_service(workspace) is invoked against a temp workspace path
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    let service = build_service(workspace).expect("build_service");

    // @step Then the returned Arc<SharedFspecService>::cwd() returns Some equal to that workspace path
    assert_eq!(
        service.cwd(),
        Some(&workspace.to_path_buf()),
        "build_service must call SharedFspecService::with_cwd(workspace)",
    );

    // @step And rust/fspec/src/common.rs contains the substring ".with_cwd(workspace.to_path_buf())"
    let src = codelet_test_helpers::source_composition::read_tree_or_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/common.rs"
    ))
    .expect("read common.rs");
    assert!(
        src.contains(".with_cwd(workspace.to_path_buf())"),
        "common.rs must contain the literal .with_cwd(workspace.to_path_buf()) chain in build_service",
    );
}
