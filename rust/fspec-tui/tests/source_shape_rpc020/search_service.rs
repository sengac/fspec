/// Scenario: codelet_core::file_search helper module exists with the documented surface
#[test]
fn codelet_core_file_search_helper_exists_with_documented_surface() {
    // @step Given rust/core/src/file_search.rs after RPC-020 lands
    let path = workspace_codelet_dir()
        .join("core")
        .join("src")
        .join("file_search.rs");
    // @step Then the file exists
    assert!(path.is_file(), "{} must exist", path.display());
    let body = read_raw(&path);
    // @step And the file contains the substring "pub fn search"
    assert!(body.contains("pub fn search"));
}

/// Scenario: codelet_core::lib re-exports the new file_search module
#[test]
fn codelet_core_lib_re_exports_file_search() {
    // @step Given rust/core/src/lib.rs after RPC-020 lands
    let body = read_raw(
        &workspace_codelet_dir()
            .join("core")
            .join("src")
            .join("lib.rs"),
    );
    // @step Then the file contains the substring "pub mod file_search"
    assert!(body.contains("pub mod file_search"));
}

/// Scenario: FspecService trait gains the search_files RPC method
#[test]
fn fspec_service_trait_gains_search_files_rpc_method() {
    // @step Given rust/rpc/src/lib.rs after RPC-020 lands
    let body = read_raw(
        &workspace_codelet_dir()
            .join("rpc")
            .join("src")
            .join("lib.rs"),
    );
    // @step Then the file contains the substring "async fn search_files"
    assert!(body.contains("async fn search_files"));
    // @step And the file contains the substring "prefix: String"
    assert!(body.contains("prefix: String"));
    // @step And the file contains the substring "limit: u32"
    assert!(body.contains("limit: u32"));
}
