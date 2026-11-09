//! BUG-656 — the blocklist middleware unit tests never read the user's
//! real `~/.fspec/blocklist.json`.
//!
//! Feature: spec/features/blocklist-middleware-unit-tests-never-read-the-real-home.feature

use codelet_test_helpers::TestResult;

/// The call every middleware unit test makes first.
const ISOLATE: &str = "codelet_test_helpers::isolated_home()";

/// The `#[test]` functions of the middleware unit tests whose body does
/// not start with [`ISOLATE`].
fn tests_that_do_not_isolate_their_home(source: &str) -> Result<Vec<String>, String> {
    let module = source
        .split_once("mod tests {")
        .map(|(_, module)| module)
        .ok_or("middleware.rs has no `mod tests` module")?;
    let mut offenders = Vec::new();
    for test in module.split("#[test]").skip(1) {
        let (signature, body) = test
            .split_once('{')
            .ok_or("a #[test] function has no body")?;
        let name = signature
            .split("fn ")
            .nth(1)
            .and_then(|rest| rest.split('(').next())
            .ok_or("a #[test] function has no name")?
            .trim()
            .to_string();
        let first_statement = body
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty() && !line.starts_with("//"))
            .unwrap_or_default();
        if !first_statement.contains(ISOLATE) {
            offenders.push(name);
        }
    }
    Ok(offenders)
}

fn middleware_source() -> std::io::Result<String> {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/blocklist/middleware.rs"),
    )
}

#[test]
fn the_middleware_unit_tests_load_only_the_isolated_empty_system_blocklist() -> TestResult {
    // @step Given the user's real ~/.fspec/blocklist.json has a rule blocking "this_command_does_not_exist_in_any_blocklist"
    let command = "this_command_does_not_exist_in_any_blocklist";
    let real_home = dirs::home_dir();

    // @step When the blocklist middleware unit tests run
    let source = middleware_source()?;

    // @step Then every test first points HOME at a temporary directory and FSPEC_USER_DIR at its ".fspec" directory
    let offenders = tests_that_do_not_isolate_their_home(&source)?;
    assert!(offenders.is_empty(), "{offenders:#?}");
    let home = codelet_test_helpers::isolated_home()?;
    assert_eq!(dirs::home_dir().as_deref(), Some(home));
    assert_ne!(dirs::home_dir(), real_home);
    assert_eq!(
        std::env::var_os("FSPEC_USER_DIR").map(std::path::PathBuf::from),
        Some(home.join(".fspec"))
    );

    // @step And that directory holds a blocklist.json with no rules and an fspec-config.json that disables RLCD
    assert_eq!(
        codelet_tools::blocklist::system_config_path(),
        Ok(home.join(".fspec").join("blocklist.json"))
    );
    assert!(!codelet_tools::rlcd::config::load_rlcd_config()?.enabled);

    // @step And the system blocklist the tests load is that empty one, so "this_command_does_not_exist_in_any_blocklist" is not blocked
    assert!(codelet_tools::blocklist::load_blocklist_config(None)
        .rules
        .is_empty());
    let checked = codelet_tools::blocklist::check_bash_command(command, uuid::Uuid::nil());
    assert!(checked.is_ok(), "{checked:?}");
    Ok(())
}

#[test]
fn a_middleware_unit_test_that_does_not_isolate_its_home_fails_the_guard() -> TestResult {
    // @step Given a middleware unit test that calls check_bash_command without calling isolate_home first
    let source = "mod tests {\n    #[test]\n    fn isolated() {\n        codelet_test_helpers::isolated_home().unwrap();\n    }\n\n    #[test]\n    #[serial]\n    fn leaks_the_real_home() {\n        // a comment\n        let result = check_bash_command(\"ls\", uuid::Uuid::nil());\n    }\n}\n";

    // @step When the guard test reads the middleware unit tests
    let offenders = tests_that_do_not_isolate_their_home(source)?;

    // @step Then it fails naming that test
    assert_eq!(offenders, vec!["leaks_the_real_home".to_string()]);
    Ok(())
}
