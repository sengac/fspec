//! BUG-347 — tools tests are isolated from the host's RLCD service and
//! user config.
//!
//! Feature: spec/features/tools-tests-isolated-from-host-rlcd.feature

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Tests that construct file tools whose operations reach the RLCD stage.
const FILE_TOOLS: [&str; 7] = [
    "ReadTool::new",
    "WriteTool::new",
    "EditTool::new",
    "ApplyPatch",
    "apply_patch",
    "LsTool::new",
    "MultiEdit",
];

#[test]
fn the_isolated_user_directory_disables_rlcd() -> TestResult {
    // @step Given a test binary that isolates its user directory
    let user = codelet_test_helpers::isolated_user_directory()?;
    // @step When the RLCD configuration is loaded
    let config = codelet_tools::rlcd::config::load_rlcd_config()?;
    // @step Then it comes from the isolated directory and RLCD is disabled
    assert_eq!(codelet_common::user_dir::fspec_user_dir()?.as_path(), user);
    assert!(!config.enabled, "{config:?}");
    Ok(())
}

#[test]
fn every_file_tool_test_isolates_its_user_directory() -> TestResult {
    // @step Given the codelet-tools test sources
    let tests = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut offenders = Vec::new();
    // @step When the tests that run file tools are scanned
    for entry in std::fs::read_dir(&tests)? {
        let path = entry?.path();
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let source = std::fs::read_to_string(&path)?;
        let runs_file_tools = FILE_TOOLS.iter().any(|tool| source.contains(tool));
        let isolated =
            source.contains("isolated_user_directory()") || source.contains("FSPEC_USER_DIR");
        if runs_file_tools && !isolated {
            offenders.push(path.display().to_string());
        }
    }
    // @step Then each of them isolates its user directory
    assert!(offenders.is_empty(), "{offenders:#?}");
    Ok(())
}
