//! Feature: spec/features/fspec-config-writes-are-atomic-and-tests-never-touch-the-real-user-config.feature
//!
//! BUG-640: this crate depends on codelet-common with its `test-support`
//! feature, so this binary is a test-support build. With `FSPEC_USER_DIR`
//! unset the one user-directory resolver refuses to fall back to
//! `<home>/.fspec`, and a config writer that resolves through it touches
//! nothing. The binary holds one test, so the environment it changes is its
//! own.

use codelet_common::fspec_config::update_config_file;
use codelet_common::user_dir::{fspec_user_dir, NOT_ISOLATED};
use codelet_test_helpers::{OrFail, TestResult};

const EXPECTED: &str = "FSPEC_USER_DIR is not set in a test-support build; isolate the user \
                        directory with codelet_test_helpers::isolated_user_directory() first";

#[test]
fn a_test_support_build_that_has_not_isolated_its_user_directory_cannot_resolve_the_real_one(
) -> TestResult {
    // @step Given a build with codelet-common's test-support feature and FSPEC_USER_DIR unset
    let home = tempfile::tempdir().or_fail("a temp home")?;
    std::env::set_var("HOME", home.path());
    std::env::remove_var("FSPEC_USER_DIR");

    // @step When it resolves the fspec user directory
    let resolved = fspec_user_dir();
    // An empty FSPEC_USER_DIR counts as unset.
    std::env::set_var("FSPEC_USER_DIR", "");
    let resolved_with_empty = fspec_user_dir();
    std::env::remove_var("FSPEC_USER_DIR");

    // @step Then it fails with "FSPEC_USER_DIR is not set in a test-support build; isolate the user directory with codelet_test_helpers::isolated_user_directory() first"
    assert_eq!(NOT_ISOLATED, EXPECTED);
    assert_eq!(resolved, Err(EXPECTED.to_string()));
    assert_eq!(resolved_with_empty, Err(EXPECTED.to_string()));

    // @step And nothing under <home>/.fspec is read or written
    let saved = fspec_user_dir().and_then(|directory| {
        update_config_file(&directory.join("fspec-config.json"), |root| {
            root["tui"] = serde_json::json!({"lastUsedModel": "anthropic/claude-opus-4-5"});
            Ok::<_, String>(())
        })
    });
    assert_eq!(saved, Err(EXPECTED.to_string()));
    assert!(
        !home.path().join(".fspec").exists(),
        "the test-support build created <home>/.fspec"
    );
    Ok(())
}
