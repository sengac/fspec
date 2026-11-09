//! Feature: spec/features/fspec-config-writes-are-atomic-and-tests-never-touch-the-real-user-config.feature
//!
//! BUG-656 (folded into BUG-640): the system blocklist and the system stage
//! permissions live in the user directory, so a test build resolves them
//! from FSPEC_USER_DIR only and never names the real `~/.fspec`. The binary
//! holds one test, so the environment it changes is its own.

use codelet_common::user_dir::NOT_ISOLATED;
use codelet_test_helpers::{OrFail, TestResult};

#[test]
fn a_test_builds_system_blocklist_is_the_isolated_user_directorys() -> TestResult {
    // @step Given a tools test build with FSPEC_USER_DIR set to a temp dir
    let user_dir = tempfile::tempdir().or_fail("a temp user directory")?;
    std::env::set_var("FSPEC_USER_DIR", user_dir.path());

    // @step When it resolves the system blocklist path
    let blocklist = codelet_tools::blocklist::system_config_path();
    let stage_permissions = codelet_tools::stage_permissions_system_config_path();

    // @step Then the path is <temp>/blocklist.json
    assert_eq!(blocklist, Ok(user_dir.path().join("blocklist.json")));
    assert_eq!(
        stage_permissions,
        Ok(user_dir.path().join("stage-permissions.json"))
    );

    // @step And with FSPEC_USER_DIR unset it fails with the test-support error instead of naming <home>/.fspec/blocklist.json
    std::env::remove_var("FSPEC_USER_DIR");
    assert_eq!(
        codelet_tools::blocklist::system_config_path(),
        Err(NOT_ISOLATED.to_string())
    );
    assert_eq!(
        codelet_tools::stage_permissions_system_config_path(),
        Err(NOT_ISOLATED.to_string())
    );
    Ok(())
}
