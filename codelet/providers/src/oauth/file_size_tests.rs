//! Tests for file size compliance (PROV-060)
//!
//! Feature: spec/features/shared-oauth-building-blocks.feature
//! Scenario: All new shared module files comply with 300-line limit

// @step Given the new oauth/ module directory contains credential_store, http_middleware,
//       device_flow, callback_server, token_refresh, engine, building_blocks, and script_provider
// @step When each file's line count is checked
// @step Then every file is under 300 lines

#[test]
fn all_oauth_module_files_under_300_lines() {
    let oauth_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/oauth");

    let files = [
        "mod.rs",
        "credential_store.rs",
        "token_refresh.rs",
        "http_middleware.rs",
        "device_flow.rs",
        "callback_server.rs",
        "engine.rs",
        "building_blocks.rs",
        "script_provider.rs",
    ];

    for file_name in &files {
        let path = oauth_dir.join(file_name);
        assert!(
            path.exists(),
            "Expected file {file_name} to exist at {}",
            path.display()
        );

        let content = std::fs::read_to_string(&path).unwrap();
        let line_count = content.lines().count();

        assert!(
            line_count <= 300,
            "File {file_name} has {line_count} lines, exceeds 300-line limit"
        );
    }
}
