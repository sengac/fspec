//! BLOCK-012 — auto-install of the default system blocklist template.
//!
//! Feature: spec/features/auto-install-default-system-blocklist-template-when-fspec-blocklist-json-is-missing.feature
//!
//! When `~/.fspec/blocklist.json` does not exist, the first
//! `load_blocklist_config` call (the chokepoint every
//! `check_bash_command`/`check_file_path` passes through) writes the
//! embedded default template to that path and loads it in the same call.
//!
//! The system blocklist path is derived from `$HOME`, so every test
//! redirects `HOME` to a fresh temp dir via a `HomeGuard` RAII (mirroring
//! `GlobalBlocklistGuard` in `rust/fspec/src/blocklist_init_tests.rs`,
//! RPC-407) and runs `#[serial]` — the blocklist root and session
//! allowances are process-global.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use codelet_tools::blocklist::{
    check_bash_command, clear_session_allowances, default_blocklist_config, init_blocklist,
    load_blocklist_config, system_config_path, BlocklistAction, BlocklistConfig, BlocklistRule,
};
use serial_test::serial;

const TEMPLATE_RULE_COUNT: usize = 68;
const SENTINEL_RULE_ID: &str = "sentinel-block012";

/// RAII guard: redirects `HOME` to a fresh temp dir for the test's
/// duration so the real `~/.fspec/blocklist.json` can never interfere.
/// On drop — including a panicking unwind — restores the prior `HOME`,
/// clears session allowances, and resets the project root via
/// `init_blocklist(None)` so later `#[serial]` tests start clean.
struct HomeGuard {
    prior_home: Option<std::ffi::OsString>,
    home_tmp: tempfile::TempDir,
}

impl HomeGuard {
    fn new() -> Self {
        let home_tmp = tempfile::tempdir().expect("tempdir for HOME redirect");
        let prior_home = std::env::var_os("HOME");
        std::env::set_var("HOME", home_tmp.path());
        Self {
            prior_home,
            home_tmp,
        }
    }

    /// The redirected home directory.
    fn home(&self) -> &Path {
        self.home_tmp.path()
    }

    /// The system blocklist path under the redirected home.
    fn system_blocklist(&self) -> std::path::PathBuf {
        self.home().join(".fspec").join("blocklist.json")
    }
}

impl Drop for HomeGuard {
    fn drop(&mut self) {
        clear_session_allowances();
        init_blocklist(None);
        match self.prior_home.take() {
            Some(home) => std::env::set_var("HOME", home),
            None => std::env::remove_var("HOME"),
        }
    }
}

/// Parse the system blocklist file on disk (under the redirected HOME).
fn read_system_blocklist() -> BlocklistConfig {
    let path = system_config_path().expect("HOME must be set under HomeGuard");
    BlocklistConfig::load_from_file(&path).expect("system blocklist must parse")
}

/// Scenario: Embedded template is valid and complete
#[test]
fn scenario_embedded_template_is_valid_and_complete() {
    // @step Given the codelet-tools crate compiles with the bundled default blocklist template
    // (compilation is a precondition of running this test)

    // @step When the template is parsed as a BlocklistConfig
    let config = default_blocklist_config().expect("embedded template must parse");

    // @step Then the template has version "1.0.0"
    assert_eq!(config.version, "1.0.0", "template version must be 1.0.0");

    // @step And the template contains exactly 68 rules
    assert_eq!(
        config.rules.len(),
        TEMPLATE_RULE_COUNT,
        "template must contain exactly {TEMPLATE_RULE_COUNT} rules"
    );

    // @step And every rule id in the template is unique
    let mut ids: Vec<&str> = config.rules.iter().map(|r| r.id.as_str()).collect();
    let total = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(
        ids.len(),
        total,
        "all template rule ids must be unique"
    );
}

/// Scenario: Template is installed and active on first check when no system blocklist exists
#[test]
#[serial]
fn scenario_template_is_installed_and_active_on_first_check_when_no_system_blocklist_exists() {
    let _guard = HomeGuard::new();

    // @step Given a fresh environment where "~/.fspec/blocklist.json" does not exist
    assert!(
        !_guard.system_blocklist().exists(),
        "precondition: no system blocklist in the redirected HOME"
    );

    // @step When the AI runs "git stash" via Bash
    let result = check_bash_command("git stash", uuid::Uuid::nil());

    // @step Then "~/.fspec/blocklist.json" exists on disk
    assert!(
        _guard.system_blocklist().exists(),
        "first check must install the template at ~/.fspec/blocklist.json"
    );

    // @step And the file on disk parses as a BlocklistConfig with 68 rules
    let installed = read_system_blocklist();
    assert_eq!(
        installed.rules.len(),
        TEMPLATE_RULE_COUNT,
        "installed file must contain all {TEMPLATE_RULE_COUNT} template rules"
    );

    // @step And the command is blocked with rule id "git-stash-block"
    let err = result
        .err()
        .unwrap_or_else(|| panic!("git stash must be blocked by the freshly installed template in the same call"));
    assert_eq!(
        err.rule_id, "git-stash-block",
        "blocked error must carry the template rule id"
    );

    // @step And the blocked error carries the reason from the template rule
    assert!(
        err.reason.contains("git stash"),
        "blocked error reason must come from the template rule; got: {}",
        err.reason
    );
}

/// Scenario: Install creates the ~/.fspec parent directory when it is missing
#[test]
#[serial]
fn scenario_install_creates_the_fspec_parent_directory_when_it_is_missing() {
    let _guard = HomeGuard::new();

    // @step Given a fresh environment where the "~/.fspec" directory itself does not exist
    assert!(
        !_guard.home().join(".fspec").exists(),
        "precondition: no ~/.fspec directory in the redirected HOME"
    );

    // @step When the blocklist is loaded
    let _config = load_blocklist_config(None);

    // @step Then the "~/.fspec" directory is created
    assert!(
        _guard.home().join(".fspec").is_dir(),
        "load must create the ~/.fspec parent directory"
    );

    // @step And "~/.fspec/blocklist.json" contains the embedded template
    let installed = read_system_blocklist();
    assert_eq!(
        installed.rules.len(),
        TEMPLATE_RULE_COUNT,
        "installed file must contain the full embedded template"
    );
}

/// Scenario: Existing user blocklist is never overwritten
#[test]
#[serial]
fn scenario_existing_user_blocklist_is_never_overwritten() {
    let _guard = HomeGuard::new();

    // @step Given a user blocklist at "~/.fspec/blocklist.json" containing a single block rule for pattern "sentinel-block012"
    let fspec_dir = _guard.home().join(".fspec");
    std::fs::create_dir_all(&fspec_dir).expect("create ~/.fspec");
    let custom = BlocklistConfig {
        version: "1.0.0".to_string(),
        rules: vec![BlocklistRule {
            id: SENTINEL_RULE_ID.to_string(),
            pattern: "sentinel-block012".to_string(),
            action: BlocklistAction::Block,
            reason: "BLOCK-012 user rule".to_string(),
            guidance: None,
        }],
    };
    let original = serde_json::to_string_pretty(&custom).expect("serialize custom config");
    std::fs::write(_guard.system_blocklist(), &original).expect("write custom blocklist");

    // @step When the blocklist is loaded
    let _config = load_blocklist_config(None);

    // @step Then the file on disk is byte-identical to the user's original file
    let on_disk = std::fs::read_to_string(_guard.system_blocklist()).expect("read blocklist");
    assert_eq!(
        on_disk, original,
        "existing user blocklist must never be overwritten"
    );

    // @step And running a command matching "sentinel-block012" is blocked by the user rule
    let result = check_bash_command("sentinel-block012", uuid::Uuid::nil());
    let err = result
        .err()
        .unwrap_or_else(|| panic!("sentinel command must be blocked by the user rule"));
    assert_eq!(err.rule_id, SENTINEL_RULE_ID, "blocked by the user rule");

    // @step And the template rule "git-stash-block" is not present in the file on disk
    assert!(
        !on_disk.contains("git-stash-block"),
        "template rules must not leak into the user's existing file"
    );
}

/// Scenario: Deleted system blocklist is re-installed on the next check
#[test]
#[serial]
fn scenario_deleted_system_blocklist_is_reinstalled_on_the_next_check() {
    let _guard = HomeGuard::new();

    // @step Given a fresh environment where the first check already installed "~/.fspec/blocklist.json"
    let first = check_bash_command("git stash", uuid::Uuid::nil());
    assert!(first.is_err(), "first check must be blocked by the installed template");
    assert!(
        _guard.system_blocklist().exists(),
        "first check must have installed the template"
    );

    // @step When the system blocklist file is deleted
    std::fs::remove_file(_guard.system_blocklist()).expect("delete system blocklist");

    // @step And the AI runs "git stash" via Bash again
    let second = check_bash_command("git stash", uuid::Uuid::nil());

    // @step Then "~/.fspec/blocklist.json" exists on disk again
    assert!(
        _guard.system_blocklist().exists(),
        "next check must re-install the template after deletion"
    );

    // @step And the command is blocked with rule id "git-stash-block"
    let err = second
        .err()
        .unwrap_or_else(|| panic!("second check must be blocked after re-install"));
    assert_eq!(err.rule_id, "git-stash-block");
}

/// Scenario: Install failure degrades gracefully without breaking command checking
#[test]
#[serial]
fn scenario_install_failure_degrades_gracefully_without_breaking_command_checking() {
    let _guard = HomeGuard::new();

    // @step Given a fresh environment where "~/.fspec" exists as a regular file so the template write fails
    std::fs::write(_guard.home().join(".fspec"), "not a directory").expect("write ~/.fspec file");

    // @step When the AI runs "echo hello" via Bash
    let result = check_bash_command("echo hello", uuid::Uuid::nil());

    // @step Then the command check completes without error
    assert!(
        result.is_ok(),
        "a failed template install must never break command checking"
    );

    // @step And no panic or install failure propagates to the caller
    // (reaching this point proves the call returned normally — the
    // `result.is_ok()` assertion above already covers it)
}
