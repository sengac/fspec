use crate::support::{success, Workspace, SCENARIO, SIDECAR};
use crate::TestResult;

#[test]
fn coverage_commands_retain_cwd_and_non_global_workspace_behavior() -> TestResult {
    // @step Given a temporary current directory and a conflicting workspace decoy
    for command in ["show-coverage", "generate-coverage", "link-coverage", "unlink-coverage"] {
        let workspace = Workspace::new()?;
        let decoy = Workspace::new()?;
        std::fs::remove_file(decoy.path().join(SIDECAR))?;
        let decoy_before = decoy.snapshot()?;
        let path = decoy.path().to_str().ok_or("temporary path is not UTF-8")?;
        let mut arguments = match command {
            "show-coverage" => vec![command, "thing", "--format", "json"],
            "generate-coverage" => vec![command, "--dry-run"],
            "link-coverage" => vec![command, "thing", "--scenario", SCENARIO, "--test-file", "tests/thing.rs", "--impl-file", "src/second.rs", "--impl-lines", "1-3"],
            _ => vec![command, "thing", "--scenario", SCENARIO, "--test-file", "tests/thing.rs"],
        };
        // @step When coverage commands receive workspace before and after the subcommand
        let mut preceding = vec!["--workspace", path];
        preceding.extend(arguments.clone());
        let output = workspace.cli(&preceding)?;
        success(&output);
        arguments.extend(["--workspace", path]);
        let trailing = workspace.cli(&arguments)?;
        // @step Then preceding workspace never redirects coverage from CWD and trailing workspace remains an unknown option
        assert_eq!(trailing.status.code(), Some(1));
        assert!(trailing.stdout.is_empty());
        assert_eq!(trailing.stderr, b"error: unknown option '--workspace'\n");
        assert_eq!(decoy.snapshot()?, decoy_before);
    }
    Ok(())
}

#[test]
fn additive_coverage_help_and_legacy_cli_errors_preserve_stream_and_exit_contracts() -> TestResult {
    // @step Given frozen original help fixtures and explicit approved coverage options
    crate::help::assert_originals()?;
    let workspace = Workspace::new()?;
    let before = workspace.snapshot()?;
    // @step When the four coverage commands receive help unknown options and missing option values
    for contract in crate::help::CONTRACTS {
        let command = contract.command;
        let help = workspace.cli(&[command, "--help"])?;
        success(&help);
        let intercepted = workspace.cli(&[command, "--definitely-unknown", "--help"])?;
        success(&intercepted);
        assert_eq!(help.stdout, intercepted.stdout);
        let unknown = workspace.cli(&[command, "--definitely-unknown"])?;
        assert_eq!(unknown.status.code(), Some(1));
        assert!(unknown.stdout.is_empty());
        assert_eq!(unknown.stderr, b"error: unknown option '--definitely-unknown'\n");
        // @step Then help explicitly advertises additions before parsing and original stdout stderr newline and exit behavior is preserved
        let text = String::from_utf8(help.stdout)?;
        assert!(text.ends_with("\n\n"), "rich help newline tail changed for {command}");
        assert_eq!(text, contract.expected, "{command} help differs from the exact additive contract");
    }
    for command in ["link-coverage", "unlink-coverage"] {
        let missing = workspace.cli(&[command, "thing", "--scenario"])?;
        assert_eq!(missing.status.code(), Some(1));
        assert!(missing.stdout.is_empty());
        assert_eq!(missing.stderr, b"error: option '--scenario <name>' argument missing\n");
    }
    assert_eq!(workspace.snapshot()?, before);
    Ok(())
}
