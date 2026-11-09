#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/virtual-hook-git-context-and-post-status-hooks.feature
//
// HOOK-019: update-work-unit-status runs a work unit's post-<current> and
// pre-<new> virtual hooks before the move, and a gitContext hook receives
// the staged and unstaged files on stdin.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};
use tempfile::TempDir;

/// Write a board holding task AUTH-001 in `implementing` with `hooks` as its
/// virtualHooks.
fn board(root: &Path, hooks: Value) {
    fs::create_dir_all(root.join("spec")).unwrap();
    let board = json!({
        "meta": {"version": "1.0.0", "lastUpdated": "2026-11-07T00:00:00.000Z"},
        "workUnits": {"AUTH-001": {
            "id": "AUTH-001", "title": "Login", "type": "task", "status": "implementing",
            "createdAt": "2026-11-07T00:00:00.000Z", "updatedAt": "2026-11-07T00:00:00.000Z",
            "stateHistory": [
                {"state": "backlog", "timestamp": "2026-11-07T00:00:00.000Z"},
                {"state": "specifying", "timestamp": "2026-11-07T00:00:01.000Z"},
                {"state": "implementing", "timestamp": "2026-11-07T00:00:02.000Z"}
            ],
            "virtualHooks": hooks
        }},
        "states": {"backlog": [], "specifying": [], "testing": [], "implementing": ["AUTH-001"],
                   "validating": [], "done": [], "blocked": []}
    });
    fs::write(root.join("spec/work-units.json"), board.to_string()).unwrap();
}

fn script(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn move_to(root: &Path, status: &str) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: "update-work-unit-status".to_string(),
        args_json: json!({"workUnitId": "AUTH-001", "status": status}).to_string(),
        project_root: root.to_path_buf(),
    })
}

fn status(root: &Path) -> String {
    let raw = fs::read_to_string(root.join("spec/work-units.json")).unwrap();
    let board: Value = serde_json::from_str(&raw).unwrap();
    board["workUnits"]["AUTH-001"]["status"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Run git in `root` with no global or system configuration.
fn git(root: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args([
            "-c",
            "user.name=Hook Test",
            "-c",
            "user.email=hooks@example.invalid",
        ])
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .current_dir(root)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("HOME", root)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn context(root: &Path, file: &str) -> Value {
    serde_json::from_str(&fs::read_to_string(root.join(file)).unwrap()).unwrap()
}

#[test]
fn blocking_virtual_hook_prevents_workflow_transition_until_fixed() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    // @step Given work unit "AUTH-001" has a virtual hook "eslint" configured as blocking at post-implementing
    // @step And the work unit is in implementing status
    board(
        root,
        json!([{"name": "eslint", "event": "post-implementing", "command": "spec/hooks/eslint.sh", "blocking": true}]),
    );
    // @step And the eslint hook fails with "3 errors found" on stderr
    script(
        root,
        "spec/hooks/eslint.sh",
        "[ -f eslint-fixed ] && exit 0\necho '3 errors found' >&2\nexit 1",
    );
    // @step When I run "fspec update-work-unit-status AUTH-001 validating"
    let blocked = move_to(root, "validating");
    // @step Then the transition should be blocked
    assert!(!blocked.success, "{blocked:?}");
    let error = blocked.error.unwrap_or_default();
    // @step And the error should contain a system-reminder naming "eslint", "AUTH-001" and "post-implementing"
    let start = error
        .find("<system-reminder>")
        .unwrap_or_else(|| panic!("{error}"));
    let end = error
        .find("</system-reminder>")
        .unwrap_or_else(|| panic!("{error}"));
    let reminder = &error[start..end];
    for part in ["'eslint'", "AUTH-001", "post-implementing"] {
        assert!(reminder.contains(part), "{part} missing from {reminder}");
    }
    // @step And the error should contain "3 errors found"
    assert!(reminder.contains("3 errors found"), "{reminder}");
    // @step And work unit status should be "implementing"
    assert_eq!(status(root), "implementing");
    // @step When the eslint hook passes
    fs::write(root.join("eslint-fixed"), "").unwrap();
    // @step And I run "fspec update-work-unit-status AUTH-001 validating" again
    let passed = move_to(root, "validating");
    // @step Then the transition should succeed
    assert!(passed.success, "{passed:?}");
    // @step And work unit status should be "validating"
    assert_eq!(status(root), "validating");
}

#[test]
fn leaving_and_entering_hooks_run_before_the_move() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    // @step Given work unit "AUTH-001" is in implementing status
    // @step And it has a virtual hook "leave" at post-implementing and a virtual hook "enter" at pre-validating
    board(
        root,
        json!([
            {"name": "enter", "event": "pre-validating", "command": "spec/hooks/record.sh enter", "blocking": false},
            {"name": "leave", "event": "post-implementing", "command": "spec/hooks/record.sh leave", "blocking": false}
        ]),
    );
    // @step And each hook records its name and the saved status of "AUTH-001"
    script(
        root,
        "spec/hooks/record.sh",
        "saved=$(sed -n 's/.*\"status\":\"\\([a-z]*\\)\".*/\\1/p' spec/work-units.json | head -n 1)\n\
         echo \"$1 $saved\" >> hooks.log",
    );
    // @step When I run "fspec update-work-unit-status AUTH-001 validating"
    let result = move_to(root, "validating");
    // @step Then the transition should succeed
    assert!(result.success, "{result:?}");
    assert_eq!(status(root), "validating");
    // @step And the hooks should have recorded "leave implementing" and then "enter implementing"
    let log = fs::read_to_string(root.join("hooks.log")).unwrap();
    assert_eq!(
        log.lines().collect::<Vec<_>>(),
        vec!["leave implementing", "enter implementing"]
    );
}

#[test]
fn create_virtual_hook_with_git_context_for_changed_files_only() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    // @step Given I am working on work unit "AUTH-001" in a git repository
    board(
        root,
        json!([{"name": "eslint-changed", "event": "post-implementing", "command": "cat > git-context.json",
                "blocking": true, "gitContext": true}]),
    );
    fs::create_dir_all(root.join("src/hooks")).unwrap();
    fs::write(root.join("src/hooks/virtual.ts"), "export {};\n").unwrap();
    fs::write(root.join(".gitignore"), "git-context.json\n").unwrap();
    git(root, &["init", "-q"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "base"]);
    // @step And I have staged files: src/auth.ts, src/utils.ts
    fs::write(root.join("src/auth.ts"), "export const auth = 1;\n").unwrap();
    fs::write(root.join("src/utils.ts"), "export const utils = 1;\n").unwrap();
    git(root, &["add", "src/auth.ts", "src/utils.ts"]);
    // @step And I have unstaged files: src/hooks/virtual.ts
    fs::write(
        root.join("src/hooks/virtual.ts"),
        "export const changed = 1;\n",
    )
    .unwrap();
    // @step And work unit "AUTH-001" has a virtual hook "eslint-changed" with git context at post-implementing
    // (configured in the board above)
    // @step When I run "fspec update-work-unit-status AUTH-001 validating"
    let result = move_to(root, "validating");
    assert!(result.success, "{result:?}");
    // @step Then the hook context should include git info: {"stagedFiles": ["src/auth.ts", "src/utils.ts"], "unstagedFiles": ["src/hooks/virtual.ts"]}
    let ctx = context(root, "git-context.json");
    assert_eq!(
        ctx["stagedFiles"],
        json!(["src/auth.ts", "src/utils.ts"]),
        "{ctx}"
    );
    assert_eq!(
        ctx["unstagedFiles"],
        json!(["src/hooks/virtual.ts"]),
        "{ctx}"
    );
    assert_eq!(ctx["workUnitId"], "AUTH-001");
    assert_eq!(ctx["event"], "post-implementing");
}

#[test]
fn detect_git_staged_and_unstaged_files_for_hooks() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    // @step Given I am not in a git repository
    assert!(Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(root)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .map(|o| !o.status.success())
        .unwrap());
    // @step And work unit "AUTH-001" has a virtual hook with gitContext=true and a virtual hook without it at post-implementing
    board(
        root,
        json!([
            {"name": "with-git", "event": "post-implementing", "command": "cat > with-git.json",
             "blocking": true, "gitContext": true},
            {"name": "without-git", "event": "post-implementing", "command": "cat > without-git.json",
             "blocking": true}
        ]),
    );
    // @step When I run "fspec update-work-unit-status AUTH-001 validating"
    let result = move_to(root, "validating");
    assert!(result.success, "{result:?}");
    let with_git = context(root, "with-git.json");
    let without_git = context(root, "without-git.json");
    // @step Then the gitContext hook context should have empty stagedFiles and unstagedFiles arrays
    assert_eq!(with_git["stagedFiles"], json!([]), "{with_git}");
    assert_eq!(with_git["unstagedFiles"], json!([]), "{with_git}");
    // @step And the other hook context should have no stagedFiles or unstagedFiles keys
    assert!(without_git.get("stagedFiles").is_none(), "{without_git}");
    assert!(without_git.get("unstagedFiles").is_none(), "{without_git}");
    // @step And both hooks should receive their context via stdin as JSON
    assert_eq!(with_git["event"], "post-implementing");
    assert_eq!(without_git["event"], "post-implementing");
}
