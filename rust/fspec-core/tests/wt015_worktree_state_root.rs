//! WT-015: isolated worktrees share the main checkout's board through one
//! state-root resolver.
//!
//! Feature: spec/features/isolated-worktrees-share-the-main-checkouts-board-through-one-state-root-resolver.feature
//!
//! Commands run through `dispatch_command`, the front door every agent
//! tool call and CLI bridge uses. "A plain copy" is a directory holding a
//! copy of the main checkout's spec/ and a `.fspec/state-root` pointer
//! written as fspec writes it (JSON `{stateRoot, sessionId}`), with no git
//! involved: the resolver reads a plain file. The git scenario creates a
//! real session worktree with codelet-git in a temp repo whose identity is
//! repo-local and whose git calls ignore the user's config.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;

use codelet_common::spec_lock::{SpecLock, SPEC_LOCK_TIMEOUT_ENV};
use codelet_fspec_core::{dispatch_command, DispatchRequest};
use codelet_test_helpers::{TestError, TestResult};
use serde_json::{json, Value};

const POINTER: &str = ".fspec/state-root";

// ---------------------------------------------------------------- fixture

fn board_json() -> Value {
    let unit = |id: &str| {
        json!({
            "id": id, "title": id, "type": "story", "status": "specifying",
            "createdAt": "2026-11-07T00:00:00.000Z", "updatedAt": "2026-11-07T00:00:00.000Z",
            "stateHistory": [{"state": "specifying", "timestamp": "2026-11-07T00:00:00.000Z"}]
        })
    };
    json!({
        "meta": {"version": "0.7.1", "lastUpdated": "2026-11-07T00:00:00.000Z"},
        "workUnits": {"STORY-001": unit("STORY-001")},
        "states": {
            "backlog": [], "specifying": ["STORY-001"], "testing": [],
            "implementing": [], "validating": [], "done": [], "blocked": []
        }
    })
}

/// Write a minimal fspec project (marker, board, prefixes, foundation,
/// features dir) at `root`.
fn write_project(root: &Path) -> Result<(), TestError> {
    let spec = root.join("spec");
    fs::create_dir_all(spec.join("features"))?;
    fs::write(root.join(".gitignore"), ".fspec/\nspec/.fspec-write.lock\n")?;
    fs::write(
        spec.join("work-units.json"),
        serde_json::to_string_pretty(&board_json())?,
    )?;
    let prefix =
        |p: &str| json!({"prefix": p, "description": p, "createdAt": "2026-11-07T00:00:00.000Z"});
    fs::write(
        spec.join("prefixes.json"),
        serde_json::to_string_pretty(
            &json!({"prefixes": {"BUG": prefix("BUG"), "STORY": prefix("STORY")}}),
        )?,
    )?;
    let foundation = json!({
        "version": "2.0.0",
        "project": {"name": "wt015", "vision": "v", "projectType": "cli-tool"},
        "problemSpace": {"primaryProblem": {"title": "t", "description": "d", "impact": "high"}},
        "solutionSpace": {"overview": "o", "capabilities": []},
        "personas": [],
        "architectureDiagrams": []
    });
    fs::write(
        spec.join("foundation.json"),
        serde_json::to_string_pretty(&foundation)?,
    )?;
    Ok(())
}

fn write_pointer(copy: &Path, state_root: &Path) -> Result<(), TestError> {
    fs::create_dir_all(copy.join(".fspec"))?;
    fs::write(
        copy.join(POINTER),
        json!({"stateRoot": state_root, "sessionId": "s1"}).to_string(),
    )?;
    Ok(())
}

/// A main checkout and a plain copy of it (inside main's .fspec/worktrees,
/// where fspec puts session worktrees) holding a pointer to main.
fn main_and_copy() -> Result<(tempfile::TempDir, PathBuf, PathBuf), TestError> {
    let dir = tempfile::tempdir()?;
    let main = dir.path().canonicalize()?.join("main");
    write_project(&main)?;
    let copy = main.join(".fspec").join("worktrees").join("s1");
    write_project(&copy)?;
    write_pointer(&copy, &main)?;
    Ok((dir, main, copy))
}

fn read(path: &Path) -> Result<String, TestError> {
    Ok(fs::read_to_string(path)?)
}

fn board(root: &Path) -> Result<Value, TestError> {
    Ok(serde_json::from_str(&read(
        &root.join("spec").join("work-units.json"),
    )?)?)
}

fn dispatch(root: &Path, command: &str, args: Value) -> Result<Value, String> {
    let result = dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: root.to_path_buf(),
    });
    if !result.success {
        return Err(result.error.unwrap_or_else(|| "no error text".to_string()));
    }
    let data: Value = serde_json::from_str(&result.data).unwrap_or(Value::Null);
    if data.get("success") == Some(&Value::Bool(false)) {
        return Err(result.data);
    }
    Ok(data)
}

fn rule_texts(board: &Value, id: &str) -> Vec<String> {
    board["workUnits"][id]["rules"]
        .as_array()
        .map(|rules| {
            rules
                .iter()
                .filter_map(|r| r["text"].as_str().or_else(|| r.as_str()))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Ids of the board's work units whose prefix is `prefix`.
fn ids_with_prefix(board: &Value, prefix: &str) -> Vec<String> {
    board["workUnits"]
        .as_object()
        .map(|units| {
            units
                .keys()
                .filter(|id| id.starts_with(&format!("{prefix}-")))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// Run `git` in `cwd` without the user's global or system config.
fn git(cwd: &Path, args: &[&str]) -> Result<String, TestError> {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()?;
    if !out.status.success() {
        return Err(format!(
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        )
        .into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

// -------------------------------------------------------------- scenarios

#[test]
fn board_commands_run_in_a_git_worktree_change_the_main_checkouts_board() -> TestResult {
    // @step Given a temp git repo with a board committed and an isolated worktree created from it
    let dir = tempfile::tempdir()?;
    let main = dir.path().canonicalize()?;
    write_project(&main)?;
    git(&main, &["init", "-q"])?;
    git(&main, &["config", "user.name", "Test User"])?;
    git(&main, &["config", "user.email", "test@example.com"])?;
    git(&main, &["add", "-A"])?;
    git(&main, &["commit", "-q", "-m", "initial commit"])?;
    let created = codelet_git::create_worktree(&main, "s1")?;
    let worktree = created.info.path;
    let worktree_board_before = read(&worktree.join("spec").join("work-units.json"))?;

    // @step When create-story and add-rule run in the worktree
    dispatch(
        &worktree,
        "create-story",
        json!({"prefix": "STORY", "title": "Made in the worktree"}),
    )?;
    dispatch(
        &worktree,
        "add-rule",
        json!({"workUnitId": "STORY-001", "rule": "rule from the worktree"}),
    )?;

    // @step Then the new card and its rule are in the main checkout's spec/work-units.json
    let main_board = board(&main)?;
    assert_eq!(
        main_board["workUnits"]["STORY-002"]["title"],
        json!("Made in the worktree"),
        "main's STORY ids: {:?}",
        ids_with_prefix(&main_board, "STORY")
    );
    assert_eq!(
        rule_texts(&main_board, "STORY-001"),
        vec!["rule from the worktree"]
    );

    // @step And the worktree's spec/work-units.json is byte-for-byte unchanged
    assert_eq!(
        read(&worktree.join("spec").join("work-units.json"))?,
        worktree_board_before
    );
    Ok(())
}

#[test]
fn a_plain_copy_holding_a_pointer_keeps_feature_files_with_the_branch() -> TestResult {
    // @step Given a main checkout with a board and a plain copy of it holding a state-root pointer
    let (_dir, main, copy) = main_and_copy()?;

    // @step When create-feature "login" runs in the copy
    dispatch(&copy, "create-feature", json!({"name": "login"}))?;

    // @step Then the copy has spec/features/login.feature
    assert!(copy.join("spec/features/login.feature").is_file());

    // @step And the main checkout has no spec/features/login.feature
    assert!(!main.join("spec/features/login.feature").exists());
    Ok(())
}

#[test]
fn ids_allocated_from_the_main_checkout_and_a_worktree_never_collide() -> TestResult {
    // @step Given a main checkout with a board and a plain copy of it holding a state-root pointer
    let (_dir, main, copy) = main_and_copy()?;
    let copy_board_before = read(&copy.join("spec").join("work-units.json"))?;

    // @step When create-bug runs in the main checkout, then in the copy, then in the main checkout
    for (root, title) in [(&main, "first"), (&copy, "second"), (&main, "third")] {
        dispatch(root, "create-bug", json!({"prefix": "BUG", "title": title}))?;
    }

    // @step Then the three bugs have distinct ids
    let mut ids = ids_with_prefix(&board(&main)?, "BUG");
    ids.sort();
    assert_eq!(ids, vec!["BUG-001", "BUG-002", "BUG-003"]);

    // @step And the copy's spec/work-units.json is byte-for-byte unchanged
    assert_eq!(
        read(&copy.join("spec").join("work-units.json"))?,
        copy_board_before
    );
    Ok(())
}

#[test]
fn a_writer_in_a_worktree_waits_on_the_main_checkouts_spec_lock() -> TestResult {
    // @step Given a main checkout with a board and a plain copy of it holding a state-root pointer
    let (_dir, main, copy) = main_and_copy()?;
    let main_before = read(&main.join("spec").join("work-units.json"))?;
    let copy_before = read(&copy.join("spec").join("work-units.json"))?;

    // @step And another writer holds the main checkout's spec lock and the lock timeout is 0 ms
    std::env::set_var(SPEC_LOCK_TIMEOUT_ENV, "0");
    let (held_tx, held_rx) = mpsc::channel::<Result<(), String>>();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let main_spec = main.join("spec");
    let holder = std::thread::spawn(move || match SpecLock::acquire(&main_spec) {
        Ok(lock) => {
            let _ = held_tx.send(Ok(()));
            let _ = release_rx.recv();
            drop(lock);
        }
        Err(error) => {
            let _ = held_tx.send(Err(error.to_string()));
        }
    });
    held_rx.recv()??;

    // @step When add-rule runs in the copy
    let outcome = dispatch(
        &copy,
        "add-rule",
        json!({"workUnitId": "STORY-001", "rule": "blocked rule"}),
    );
    let _ = release_tx.send(());
    holder.join().map_err(|_| "lock holder panicked")?;

    // @step Then it fails with the standard lock message naming the main checkout's spec/.fspec-write.lock
    let error = outcome
        .err()
        .ok_or("add-rule succeeded while main's lock was held")?;
    let lock_file = main.join("spec").join(".fspec-write.lock");
    assert!(
        error.contains(&format!(
            "Failed to acquire file lock on {}",
            lock_file.display()
        )),
        "{error}"
    );

    // @step And neither board changed
    assert_eq!(
        read(&main.join("spec").join("work-units.json"))?,
        main_before
    );
    assert_eq!(
        read(&copy.join("spec").join("work-units.json"))?,
        copy_before
    );
    Ok(())
}

#[test]
fn a_broken_pointer_is_a_hard_error_naming_the_pointer_file() -> TestResult {
    // @step Given a plain copy whose state-root pointer is relative, chained, aimed at a root without spec/, or not valid JSON
    let (_dir, main, copy) = main_and_copy()?;
    let other = main.join("other");
    write_project(&other)?;
    write_pointer(&other, &main)?;
    let no_spec = main.join("no-spec");
    fs::create_dir_all(&no_spec)?;
    let pointers = [
        (
            "relative",
            json!({"stateRoot": "main", "sessionId": "s1"}).to_string(),
        ),
        (
            "chained",
            json!({"stateRoot": other, "sessionId": "s1"}).to_string(),
        ),
        (
            "without spec/",
            json!({"stateRoot": no_spec, "sessionId": "s1"}).to_string(),
        ),
        ("not valid JSON", "{".to_string()),
    ];
    let main_before = read(&main.join("spec").join("work-units.json"))?;
    let copy_before = read(&copy.join("spec").join("work-units.json"))?;
    let pointer_path = copy.join(POINTER);

    for (case, pointer) in pointers {
        fs::write(&pointer_path, pointer)?;

        // @step When add-rule runs in the copy
        let outcome = dispatch(
            &copy,
            "add-rule",
            json!({"workUnitId": "STORY-001", "rule": format!("rule with a {case} pointer")}),
        );

        // @step Then it fails naming the copy's .fspec/state-root
        let error = outcome
            .err()
            .ok_or_else(|| format!("add-rule succeeded with a {case} pointer"))?;
        assert!(
            error.contains(&pointer_path.display().to_string()),
            "{case}: {error}"
        );

        // @step And neither checkout's spec/ changed
        assert_eq!(
            read(&main.join("spec").join("work-units.json"))?,
            main_before,
            "{case}"
        );
        assert_eq!(
            read(&copy.join("spec").join("work-units.json"))?,
            copy_before,
            "{case}"
        );
    }
    Ok(())
}

#[test]
fn a_worktree_of_a_project_without_a_committed_spec_keeps_its_features_in_its_own_spec(
) -> TestResult {
    // @step Given a main checkout with a board and a copy of it with no spec/, a state-root pointer and a subdirectory "rust" holding Cargo.toml
    let dir = tempfile::tempdir()?;
    let main = dir.path().canonicalize()?.join("main");
    write_project(&main)?;
    let copy = main.join(".fspec").join("worktrees").join("s1");
    fs::create_dir_all(copy.join("rust"))?;
    fs::write(copy.join(".gitignore"), "")?;
    fs::write(copy.join("rust").join("Cargo.toml"), "[workspace]\n")?;
    write_pointer(&copy, &main)?;

    // @step When create-feature "login" runs with the copy's "rust" directory as its project root
    dispatch(
        &copy.join("rust"),
        "create-feature",
        json!({"name": "login"}),
    )?;

    // @step Then the copy has spec/features/login.feature
    assert!(copy.join("spec/features/login.feature").is_file());

    // @step And the main checkout's spec/features/ is unchanged
    let main_features: Vec<_> = fs::read_dir(main.join("spec/features"))?.collect();
    assert!(main_features.is_empty(), "{main_features:?}");
    Ok(())
}

/// Scenario: A broken pointer fails every reader command too, naming the pointer file
#[test]
fn a_broken_pointer_fails_every_reader_command_too_naming_the_pointer_file() -> TestResult {
    // @step Given a plain copy whose state-root pointer names a folder that does not exist
    let (_dir, main, copy) = main_and_copy()?;
    fs::write(
        copy.join("spec").join("features").join("login.feature"),
        "@WT-015\nFeature: Login\n\n  Scenario: Sign in\n    Given a user\n",
    )?;
    write_pointer(&copy, &main.join("missing"))?;
    let pointer_path = copy.join(POINTER).display().to_string();

    // @step When list-work-units, show-feature, list-feature-tags, context or any other command runs in the copy
    let readers = [
        ("list-work-units", json!({})),
        ("show-work-unit", json!({"workUnitId": "STORY-001"})),
        ("show-feature", json!({"feature": "login"})),
        (
            "list-feature-tags",
            json!({"file": "spec/features/login.feature"}),
        ),
        ("context", json!({"text": "Sign"})),
        ("list-tags", json!({})),
        ("tag-stats", json!({})),
        ("validate-tags", json!({})),
        ("show-epic", json!({"epicId": "none"})),
    ];
    let every_command = codelet_fspec_core::canonical::CANONICAL_COMMANDS
        .iter()
        .map(|command| (command.name, json!({})));
    for (command, args) in readers.into_iter().chain(every_command) {
        let outcome = dispatch(&copy, command, args);

        // @step Then each fails naming the copy's .fspec/state-root
        // @step And none prints an empty board or an empty result
        let error = match outcome {
            Ok(data) => {
                return Err(format!("{command} succeeded with a broken pointer: {data}").into())
            }
            Err(error) => error,
        };
        assert!(error.contains(&pointer_path), "{command}: {error}");
    }
    Ok(())
}
