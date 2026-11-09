//! BUG-285 / INIT-011 — the top-level `fspec --sync-version <version>` flag.
//!
//! Feature: spec/features/automatic-version-check-and-update-for-slash-command-files.feature
//!
//! The generated /fspec slash command runs `fspec --sync-version <embedded>`
//! as its first step; before BUG-285 the Rust CLI rejected the flag with
//! "unknown option '--sync-version'". Each test maps 1:1 to a scenario and
//! drives the real `fspec` binary inside a temp project (HOME redirected into
//! the sandbox, FSPEC_AGENT cleared so agent detection reads the project).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

use codelet_test_helpers::setup_temp_directory;

mod common;

use common::fspec_bin;

/// The running binary version — the same workspace version fspec-core embeds.
const CURRENT: &str = env!("CARGO_PKG_VERSION");
const STALE: &str = "0.0.1";

/// Run the fspec binary in `cwd`, returning (exit code, stdout, stderr).
fn run_fspec(cwd: &Path, args: &[&str]) -> (i32, String, String) {
    let output = Command::new(fspec_bin())
        .args(args)
        .current_dir(cwd)
        .env("HOME", cwd)
        .env_remove("FSPEC_AGENT")
        .env_remove("FSPEC_CAPTURE_MODE")
        .output()
        .expect("spawn fspec");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn read(root: &Path, rel: &str) -> String {
    fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("parent dir")).expect("mkdir");
    fs::write(path, body).expect("write fixture");
}

fn init_claude(root: &Path) {
    let (code, _, stderr) = run_fspec(root, &["init", "--agent=claude"]);
    assert_eq!(code, 0, "fspec init must succeed; stderr={stderr}");
}

fn stale_slash_command() -> String {
    format!("# fspec Command\n\nfspec --sync-version {STALE}\n")
}

#[test]
fn scenario_version_mismatch_detected_on_upgrade_claude_code() {
    let tmp = setup_temp_directory().expect("tempdir");
    let root = tmp.path();
    // @step Given a project with .claude/commands/fspec.md containing "fspec --sync-version 0.0.1"
    init_claude(root);
    write(root, ".claude/commands/fspec.md", &stale_slash_command());
    write(root, "spec/AGENTS.md", "stale docs");
    // @step And spec/fspec-config.json contains agent "claude"
    let config: serde_json::Value =
        serde_json::from_str(&read(root, "spec/fspec-config.json")).expect("config JSON");
    assert_eq!(config["agent"], "claude");
    // @step And the installed fspec binary reports a newer version than 0.0.1
    let (_, version_out, _) = run_fspec(root, &["--version"]);
    assert_eq!(version_out.trim(), format!("fspec {CURRENT}"));
    assert_ne!(CURRENT, STALE);

    // @step When the AI agent runs "fspec --sync-version 0.0.1" as the first command of /fspec
    let (code, stdout, stderr) = run_fspec(root, &["--sync-version", STALE]);

    // @step Then it should update .claude/commands/fspec.md with new content including "fspec --sync-version" and the current version
    let slash = read(root, ".claude/commands/fspec.md");
    assert!(
        slash.contains(&format!("fspec --sync-version {CURRENT}")),
        "{slash}"
    );
    // @step And it should update spec/AGENTS.md with latest documentation
    let doc = read(root, "spec/AGENTS.md");
    assert_ne!(doc, "stale docs");
    assert!(doc.contains("fspec"), "AGENTS.md must be regenerated");
    // @step And it should print "⚠️  fspec files updated from v0.0.1 to v" followed by the current version
    let banner = format!("⚠️  fspec files updated from v{STALE} to v{CURRENT}");
    assert!(stdout.contains(&banner), "stdout={stdout} stderr={stderr}");
    // @step And it should print Claude-specific restart message wrapped in <system-reminder> tags
    assert!(
        stdout.trim_start().starts_with("<system-reminder>"),
        "{stdout}"
    );
    assert!(
        stdout.trim_end().ends_with("</system-reminder>"),
        "{stdout}"
    );
    // @step And the restart message should say "Exit this conversation and start a new one. Run /fspec again."
    assert!(stdout.contains("Exit this conversation and start a new one. Run /fspec again."));
    // @step And it should exit with code 1 (stopping workflow)
    assert_eq!(code, 1, "stdout={stdout} stderr={stderr}");
}

#[test]
fn scenario_version_match_no_update_needed() {
    let tmp = setup_temp_directory().expect("tempdir");
    let root = tmp.path();
    // BUG-607: the config is written before init, as in a configured project,
    // so the installed doc already holds the configured test command.
    // @step And spec/fspec-config.json has a test command configured
    write(
        root,
        "spec/fspec-config.json",
        r#"{"agent":"claude","tools":{"test":{"command":"cargo test"}}}"#,
    );
    // @step Given a project initialised with "fspec init --agent=claude" by the installed fspec binary
    init_claude(root);
    let tracked = [
        ".claude/commands/fspec.md",
        "spec/AGENTS.md",
        "spec/fspec-config.json",
    ];
    let before: Vec<String> = tracked.iter().map(|rel| read(root, rel)).collect();

    // @step When the AI agent runs "fspec --sync-version" with the current version
    let (code, stdout, stderr) = run_fspec(root, &["--sync-version", CURRENT]);

    // @step Then it should not update any files
    let after: Vec<String> = tracked.iter().map(|rel| read(root, rel)).collect();
    assert_eq!(before, after);
    // @step And it should not print any output (silent)
    assert_eq!(stdout, "", "stdout must be empty");
    assert_eq!(stderr, "", "stderr must be empty");
    // @step And it should exit with code 0 (continuing workflow)
    assert_eq!(code, 0);
}

#[test]
fn scenario_version_mismatch_detected_for_cursor_agent_no_system_reminders() {
    let tmp = setup_temp_directory().expect("tempdir");
    let root = tmp.path();
    // @step Given a project with .cursor/commands/fspec.md containing "fspec --sync-version 0.0.1"
    write(root, ".cursor/commands/fspec.md", &stale_slash_command());
    // @step And spec/fspec-config.json contains agent "cursor"
    write(root, "spec/fspec-config.json", r#"{"agent":"cursor"}"#);

    // @step When the AI agent runs "fspec --sync-version 0.0.1"
    let (code, stdout, stderr) = run_fspec(root, &["--sync-version", STALE]);

    // @step Then it should update .cursor/commands/fspec.md with "fspec --sync-version" and the current version
    let slash = read(root, ".cursor/commands/fspec.md");
    assert!(
        slash.contains(&format!("fspec --sync-version {CURRENT}")),
        "{slash}"
    );
    // @step And it should update spec/CURSOR.md with latest documentation
    assert!(read(root, "spec/CURSOR.md").contains("fspec"));
    // @step And it should print "⚠️  fspec files updated from v0.0.1 to v" as plain text (no system-reminder tags)
    assert!(
        stdout.contains(&format!(
            "⚠️  fspec files updated from v{STALE} to v{CURRENT}"
        )),
        "stdout={stdout} stderr={stderr}"
    );
    assert!(!stdout.contains("<system-reminder>"), "{stdout}");
    // @step And it should print Cursor-specific restart instructions "Restart Cursor and run /fspec again"
    assert!(
        stdout.contains("Restart Cursor and run /fspec again"),
        "{stdout}"
    );
    // @step And it should exit with code 1
    assert_eq!(code, 1);
}

#[test]
fn scenario_agent_detection_fallback_when_config_missing() {
    let tmp = setup_temp_directory().expect("tempdir");
    let root = tmp.path();
    // @step Given a project with .claude/commands/fspec.md containing "fspec --sync-version 0.0.1"
    write(root, ".claude/commands/fspec.md", &stale_slash_command());
    // @step And spec/fspec-config.json is missing
    assert!(!root.join("spec/fspec-config.json").exists());

    // @step When the AI agent runs "fspec --sync-version 0.0.1"
    let (code, stdout, stderr) = run_fspec(root, &["--sync-version", STALE]);

    // @step Then it should detect the agent from the filesystem (.claude/ directory exists)
    // @step And it should update both files (.claude/commands/fspec.md and spec/AGENTS.md)
    let slash = read(root, ".claude/commands/fspec.md");
    assert!(
        slash.contains(&format!("fspec --sync-version {CURRENT}")),
        "{slash}"
    );
    assert!(read(root, "spec/AGENTS.md").contains("fspec"));
    // @step And it should wrap the message in <system-reminder> tags because Claude supports them
    assert!(
        stdout.contains("<system-reminder>"),
        "stdout={stdout} stderr={stderr}"
    );
    // @step And it should print generic restart instructions "Restart your AI agent and run /fspec again"
    assert!(
        stdout.contains("Restart your AI agent and run /fspec again"),
        "{stdout}"
    );
    assert!(!stdout.contains("Exit this conversation"), "{stdout}");
    // @step And it should exit with code 1
    assert_eq!(code, 1);
}

#[test]
fn scenario_fspec_init_embeds_current_version_automatically() {
    let tmp = setup_temp_directory().expect("tempdir");
    let root = tmp.path();
    // @step Given an empty project directory
    assert_eq!(fs::read_dir(root).expect("read tempdir").count(), 0);

    // @step When I run "fspec init --agent=claude"
    init_claude(root);

    // @step Then it should generate .claude/commands/fspec.md with "fspec --sync-version" followed by the version reported by "fspec --version"
    let (_, version_out, _) = run_fspec(root, &["--version"]);
    let version = version_out
        .trim()
        .strip_prefix("fspec ")
        .expect("fspec --version prints `fspec <version>`")
        .to_string();
    let check = format!("fspec --sync-version {version}");
    let slash = read(root, ".claude/commands/fspec.md");
    assert!(slash.contains(&check), "{slash}");
    // @step And the version check command should appear right after the "IMMEDIATELY - run this command" line
    let mut lines = slash
        .lines()
        .skip_while(|l| !l.starts_with("IMMEDIATELY - run this command"));
    assert!(lines.next().is_some(), "IMMEDIATELY line missing:\n{slash}");
    let first_command = lines.find(|l| !l.trim().is_empty());
    assert_eq!(first_command, Some(check.as_str()));
    // @step And it should create spec/AGENTS.md with latest documentation
    assert!(read(root, "spec/AGENTS.md").contains("fspec"));
    // @step And it should create spec/fspec-config.json with agent "claude"
    let config: serde_json::Value =
        serde_json::from_str(&read(root, "spec/fspec-config.json")).expect("config JSON");
    assert_eq!(config["agent"], "claude");
    // @step And "fspec bootstrap" should print "fspec --sync-version" followed by the same version instead of a hard-coded one
    let (code, bootstrap_out, stderr) = run_fspec(root, &["bootstrap"]);
    assert_eq!(code, 0, "bootstrap stderr={stderr}");
    assert!(
        bootstrap_out.contains(&check),
        "bootstrap must embed {check}"
    );
    assert!(!bootstrap_out.contains("fspec --sync-version 0.9.3"));
}

#[test]
fn scenario_emit_tool_config_checks_when_versions_match() {
    let tmp = setup_temp_directory().expect("tempdir");
    let root = tmp.path();
    // @step Given a project initialised with "fspec init --agent=claude" by the installed fspec binary
    init_claude(root);
    // @step And spec/fspec-config.json does not have tools configured
    let config: serde_json::Value =
        serde_json::from_str(&read(root, "spec/fspec-config.json")).expect("config JSON");
    assert!(config.get("tools").is_none(), "{config}");

    // @step When the AI agent runs "fspec --sync-version" with the current version
    let (code, stdout, stderr) = run_fspec(root, &["--sync-version", CURRENT]);

    // @step Then sync-version should emit the configure-tools test command check
    assert!(
        stdout.contains("fspec configure-tools --test-command <cmd>"),
        "stdout={stdout} stderr={stderr}"
    );
    // @step And sync-version should emit the configure-tools quality commands check
    assert!(
        stdout.contains("No quality check commands configured"),
        "{stdout}"
    );
    // @step And the output should contain a system-reminder with "NO TEST COMMAND CONFIGURED"
    assert!(
        stdout.contains("<system-reminder>\nNO TEST COMMAND CONFIGURED"),
        "{stdout}"
    );
    // @step And it should exit with code 1 so the AI agent runs "fspec configure-tools" first
    assert_eq!(code, 1);
}

#[test]
fn scenario_emit_tool_config_checks_when_config_file_does_not_exist() {
    let tmp = setup_temp_directory().expect("tempdir");
    let root = tmp.path();
    // @step Given a project initialised with "fspec init --agent=claude" by the installed fspec binary
    init_claude(root);
    // @step And spec/fspec-config.json was then deleted
    fs::remove_file(root.join("spec/fspec-config.json")).expect("delete fspec-config.json");

    // @step When the AI agent runs "fspec --sync-version" with the current version
    let (code, stdout, stderr) = run_fspec(root, &["--sync-version", CURRENT]);

    // @step Then the output should contain "NO TEST COMMAND CONFIGURED"
    assert!(
        stdout.contains("NO TEST COMMAND CONFIGURED"),
        "stdout={stdout} stderr={stderr}"
    );
    // @step And the output should tell the AI agent to run "fspec configure-tools --test-command <cmd>"
    assert!(
        stdout.contains("fspec configure-tools --test-command <cmd>"),
        "{stdout}"
    );
    // @step And it should exit with code 1 so the AI agent runs "fspec configure-tools" first
    assert_eq!(code, 1, "stdout={stdout} stderr={stderr}");
    assert!(
        !root.join("spec/fspec-config.json").exists(),
        "sync-version must not write a config"
    );
}
