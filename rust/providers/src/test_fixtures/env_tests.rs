//! BUG-550: the shared FSPEC_HOME redirect resolves every credential path
//! inside its own directory and restores the previous value.
//!
//! Feature: spec/features/providers-lib-tests-share-one-environment-guard.feature

use super::FspecHome;

#[test]
#[serial_test::serial]
fn tests_in_different_modules_redirect_fspec_home_through_one_shared_fixture(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given the oauth, copilot and manager lib tests redirect FSPEC_HOME through the shared env fixture under the default serial guard
    let before = std::env::var_os("FSPEC_HOME");
    let home = FspecHome::redirect()?;

    // @step When a test redirects FSPEC_HOME and resolves the credential paths
    let paths = [
        crate::oauth::fspec_home(),
        crate::copilot::get_copilot_auth_path(),
        crate::claude_auth::get_claude_auth_path(),
    ];

    // @step Then the cred, Copilot and Claude credential paths all resolve inside its own directory, and the previous FSPEC_HOME is back when the fixture is dropped
    for path in &paths {
        assert!(
            path.starts_with(home.path()),
            "{} is outside {}",
            path.display(),
            home.path().display()
        );
    }
    drop(home);
    assert_eq!(std::env::var_os("FSPEC_HOME"), before);
    Ok(())
}
