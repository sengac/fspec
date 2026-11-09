//! RPC-407 regression tests — the standalone `fspec` binary MUST
//! initialize the project blocklist root at service startup.
//!
//! Feature: spec/features/project-blocklist-initialization.feature
//!
//! `codelet_tools::blocklist` keeps a process-global
//! `BLOCKLIST_PROJECT_ROOT`. Before RPC-407 it was set only by the
//! legacy napi path (`rust/napi/src/blocklist.rs`), so the Rust
//! binary silently ignored `<workspace>/.fspec/blocklist.json`.
//! These tests prove `common::build_service` now performs the init
//! for BOTH entry modes (daemon.rs and combined.rs both call it).
//!
//! The blocklist root is process-global and the system blocklist is
//! read from `$HOME/.fspec/blocklist.json`, so every test that touches
//! either (directly or via `build_service`) is `#[serial]` and holds a
//! `GlobalBlocklistGuard`: it redirects `HOME` to an empty temp dir for
//! the test's duration (so the real user/CI system blocklist can never
//! interfere) and, in `Drop`, unconditionally restores `HOME`, clears
//! session allowances, and resets the root via `init_blocklist(None)` —
//! even when an assertion panics mid-test.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use codelet_tools::blocklist::{
    check_bash_command, clear_session_allowances, init_blocklist, BlocklistAction, BlocklistConfig,
    BlocklistRule,
};
use serial_test::serial;

use crate::common::build_service;

const SENTINEL: &str = "sentinel-rpc407";
const RULE_ID: &str = "rpc407-project-rule";
const RULE_REASON: &str = "RPC-407 project blocklist rule";

/// Write `<root>/.fspec/blocklist.json` with a single Block rule for
/// the RPC-407 sentinel pattern.
fn write_project_blocklist(root: &Path) {
    let fspec_dir = root.join(".fspec");
    std::fs::create_dir_all(&fspec_dir).expect("create .fspec directory");
    let config = BlocklistConfig {
        version: "1.0.0".to_string(),
        rules: vec![BlocklistRule {
            id: RULE_ID.to_string(),
            pattern: SENTINEL.to_string(),
            action: BlocklistAction::Block,
            reason: RULE_REASON.to_string(),
            guidance: None,
        }],
    };
    let json = serde_json::to_string_pretty(&config).expect("serialize blocklist config");
    std::fs::write(fspec_dir.join("blocklist.json"), json).expect("write blocklist.json");
}

/// RAII guard for the process-global blocklist state.
///
/// On construction it redirects `HOME` to a fresh empty temp dir so the
/// real `~/.fspec/blocklist.json` (system rules) can never block or
/// prompt on the sentinel command. On drop — including a panicking
/// unwind from a failed assertion — it restores the prior `HOME`,
/// clears session allowances, and resets `BLOCKLIST_PROJECT_ROOT` via
/// `init_blocklist(None)` so later `#[serial]` tests start clean.
struct GlobalBlocklistGuard {
    prior_user_dir: Option<std::ffi::OsString>,
    prior_home: Option<std::ffi::OsString>,
    _home_tmp: tempfile::TempDir,
}

impl GlobalBlocklistGuard {
    fn new() -> Self {
        let home_tmp = tempfile::tempdir().expect("tempdir for HOME redirect");
        let prior_home = std::env::var_os("HOME");
        std::env::set_var("HOME", home_tmp.path());
        // BUG-656: the system blocklist lives in the user directory, which a
        // test build resolves only from FSPEC_USER_DIR; RLCD stays off.
        let prior_user_dir = std::env::var_os("FSPEC_USER_DIR");
        let user_dir = home_tmp.path().join(".fspec");
        std::fs::create_dir_all(&user_dir).expect("create the test user directory");
        std::fs::write(
            user_dir.join("fspec-config.json"),
            "{\"rlcd\": {\"enabled\": false}}\n",
        )
        .expect("write the test user config");
        std::env::set_var("FSPEC_USER_DIR", &user_dir);
        Self {
            prior_user_dir,
            prior_home,
            _home_tmp: home_tmp,
        }
    }
}

impl Drop for GlobalBlocklistGuard {
    fn drop(&mut self) {
        clear_session_allowances();
        init_blocklist(None);
        match self.prior_home.take() {
            Some(home) => std::env::set_var("HOME", home),
            None => std::env::remove_var("HOME"),
        }
        match self.prior_user_dir.take() {
            Some(directory) => std::env::set_var("FSPEC_USER_DIR", directory),
            None => std::env::remove_var("FSPEC_USER_DIR"),
        }
    }
}

/// Scenario: Project blocklist rules are enforced after service startup
#[test]
#[serial]
fn scenario_project_blocklist_rules_are_enforced_after_service_startup() {
    let _guard = GlobalBlocklistGuard::new();

    // @step Given a workspace containing ".fspec/blocklist.json" with a block rule for pattern "sentinel-rpc407"
    let tmp = tempfile::tempdir().expect("tempdir");
    let workspace = tmp.path();
    write_project_blocklist(workspace);

    // @step When the fspec binary builds its service via build_service against that workspace
    let _service = build_service(workspace).expect("build_service");

    // @step Then running a command matching "sentinel-rpc407" is blocked with the project rule id
    let result = check_bash_command("sentinel-rpc407 --do-something", uuid::Uuid::nil());
    let err = result.err().unwrap_or_else(|| {
        panic!(
            "build_service must init the project blocklist root; \
             command matching the project block rule was allowed"
        )
    });
    assert_eq!(
        err.rule_id, RULE_ID,
        "blocked error must carry the project rule id"
    );

    // @step And the blocked error carries the reason from the project blocklist rule
    assert!(
        err.reason.contains(RULE_REASON),
        "blocked error reason must come from the project rule; got: {}",
        err.reason
    );
}

/// Scenario: Blocking comes from the project config and not any other source
#[test]
#[serial]
fn scenario_blocking_comes_from_the_project_config_and_not_any_other_source() {
    let _guard = GlobalBlocklistGuard::new();

    // @step Given the project blocklist was initialized against a workspace with a "sentinel-rpc407" block rule
    let with_rule = tempfile::tempdir().expect("tempdir with rule");
    write_project_blocklist(with_rule.path());
    let _service = build_service(with_rule.path()).expect("build_service (with rule)");
    let blocked = check_bash_command("sentinel-rpc407", uuid::Uuid::nil());
    assert!(
        blocked.is_err(),
        "precondition failed: sentinel command must be blocked after build_service \
         against the rule workspace"
    );

    // @step When the blocklist is re-initialized against a different workspace without a project blocklist
    let without_rule = tempfile::tempdir().expect("tempdir without rule");
    let _service2 = build_service(without_rule.path()).expect("build_service (without rule)");

    // @step Then running a command matching "sentinel-rpc407" is allowed
    let result = check_bash_command("sentinel-rpc407", uuid::Uuid::nil());
    assert!(
        result.is_ok(),
        "sentinel command must be allowed once the blocklist root points at a workspace \
         without a project blocklist — the earlier block came from the project config"
    );
}

/// Scenario: Startup seam covers both daemon and combined modes
#[test]
fn scenario_startup_seam_covers_both_daemon_and_combined_modes() {
    // @step Given the codelet-fspec binary crate after RPC-407 lands
    let crate_root = env!("CARGO_MANIFEST_DIR");

    // @step When I open rust/fspec/src/common.rs
    let common_src =
        std::fs::read_to_string(format!("{crate_root}/src/common.rs")).expect("read src/common.rs");

    // @step Then the build_service function contains a literal init_blocklist call
    let bs_start = common_src
        .find("pub fn build_service")
        .expect("build_service definition must exist in common.rs");
    let bs_end = common_src[bs_start..]
        .find("\n}\n")
        .map(|i| bs_start + i)
        .unwrap_or(common_src.len());
    let body = &common_src[bs_start..bs_end];
    assert!(
        body.contains("init_blocklist(Some(workspace))"),
        "build_service must contain a literal `init_blocklist(Some(workspace))` call \
         (RPC-407: project .fspec/blocklist.json rules are silently ignored without it)"
    );

    // @step And both daemon.rs and combined.rs reach build_service so neither mode can skip blocklist initialization
    let daemon_src =
        std::fs::read_to_string(format!("{crate_root}/src/daemon.rs")).expect("read src/daemon.rs");
    let combined_src = std::fs::read_to_string(format!("{crate_root}/src/combined.rs"))
        .expect("read src/combined.rs");
    assert!(
        daemon_src.contains("common::build_service(&workspace)"),
        "daemon.rs must build its service via common::build_service so it inherits blocklist init"
    );
    assert!(
        combined_src.contains("common::build_service(&workspace)"),
        "combined.rs must build its service via common::build_service so it inherits blocklist init"
    );
}

/// Scenario: System blocklist rules are enforced after service startup
#[test]
#[serial]
fn scenario_system_blocklist_rules_are_enforced_after_service_startup() {
    let _guard = GlobalBlocklistGuard::new();

    // @step Given a system blocklist at "~/.fspec/blocklist.json" with a block rule for pattern "sentinel-system-rpc407" and a workspace without a project blocklist
    let system_path =
        codelet_tools::blocklist::system_config_path().expect("FSPEC_USER_DIR is set by the guard");
    std::fs::create_dir_all(system_path.parent().expect("~/.fspec")).expect("create ~/.fspec");
    let config = BlocklistConfig {
        version: "1.0.0".to_string(),
        rules: vec![BlocklistRule {
            id: "rpc407-system-rule".to_string(),
            pattern: "sentinel-system-rpc407".to_string(),
            action: BlocklistAction::Block,
            reason: "RPC-407 system blocklist rule".to_string(),
            guidance: None,
        }],
    };
    std::fs::write(
        &system_path,
        serde_json::to_string_pretty(&config).expect("serialize system blocklist"),
    )
    .expect("write ~/.fspec/blocklist.json");
    let tmp = tempfile::tempdir().expect("tempdir");
    let workspace = tmp.path();
    assert!(!workspace.join(".fspec").join("blocklist.json").exists());

    // @step When the fspec binary builds its service via build_service against that workspace
    let _service = build_service(workspace).expect("build_service");

    // @step Then running a command matching "sentinel-system-rpc407" is blocked with the system rule id
    let err = check_bash_command("sentinel-system-rpc407 --run", uuid::Uuid::nil())
        .expect_err("the system block rule blocks the command");
    assert_eq!(err.rule_id, "rpc407-system-rule");
    assert_eq!(err.reason, "RPC-407 system blocklist rule");
}
