//! BUG-451 — RLCD config errors are reported instead of replaced by defaults.
//!
//! Feature: spec/features/rlcd-config-errors-are-reported.feature

use codelet_tools::blocklist::check_bash_command;
use codelet_tools::rlcd::config::{load_rlcd_config_from, RlcdConfig};
use serial_test::serial;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
#[serial]
fn malformed_rlcd_config_is_an_error() -> TestResult {
    // @step Given fspec-config.json files with invalid JSON, a non-object root and an rlcd section with "enabled": "yes"
    let dir = codelet_test_helpers::setup_temp_directory()?;
    let cases = [
        ("invalid.json", "{ not json", "invalid.json"),
        ("array.json", "[1, 2]", "array.json"),
        ("section.json", r#"{"rlcd": {"enabled": "yes"}}"#, "rlcd"),
    ];
    for (name, content, named) in cases {
        let path = dir.path().join(name);
        std::fs::write(&path, content)?;
        // @step When each is loaded
        let loaded = load_rlcd_config_from(&path);
        // @step Then each load fails naming the file or the rlcd section
        let error = loaded.err().ok_or(format!("{name} loaded"))?;
        assert!(error.contains(named), "{name}: {error}");
    }
    Ok(())
}

#[test]
#[serial]
fn a_missing_file_or_missing_section_is_the_documented_default() -> TestResult {
    // @step Given no fspec-config.json and one without an rlcd section
    let dir = codelet_test_helpers::setup_temp_directory()?;
    let without_section = dir.path().join("fspec-config.json");
    std::fs::write(&without_section, r#"{"tui": {}}"#)?;
    // @step When each is loaded
    let missing = load_rlcd_config_from(&dir.path().join("absent.json"))?;
    let no_section = load_rlcd_config_from(&without_section)?;
    // @step Then both give the default RLCD config
    let default = serde_json::to_value(RlcdConfig::default())?;
    assert_eq!(serde_json::to_value(missing)?, default);
    assert_eq!(serde_json::to_value(no_section)?, default);
    Ok(())
}

#[test]
#[serial]
fn a_bash_command_is_refused_when_the_rlcd_config_is_malformed() -> TestResult {
    // @step Given the user's fspec-config.json has a malformed rlcd section
    let user = codelet_test_helpers::setup_temp_directory()?;
    std::fs::write(
        user.path().join("fspec-config.json"),
        r#"{"rlcd": {"enabled": "yes"}}"#,
    )?;
    std::env::set_var("FSPEC_USER_DIR", user.path());
    // @step When a Bash command is checked by the blocklist middleware
    let checked = check_bash_command("ls", uuid::Uuid::nil());
    std::env::remove_var("FSPEC_USER_DIR");
    // @step Then it is refused with the RLCD config error
    let error = checked.err().ok_or("the command was allowed")?.to_string();
    assert!(error.contains("rlcd"), "{error}");
    Ok(())
}
