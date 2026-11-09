//! WT-017: merging a worktree never merges shared spec state.
//!
//! Feature: spec/features/merging-a-worktree-never-merges-shared-spec-state.feature
//!
//! Each scenario builds a fresh git repository in a temp dir (repo-local
//! identity; every git call ignores the user's global and system config),
//! creates an isolated session worktree with codelet-git and inspects and
//! merges it through the real `get_session_diff` and
//! `merge_session_with_strategy`. HOME points at a per-process temp dir,
//! so the session manifest cleanup never touches the user's home.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use codelet_common::spec_paths::{is_shared_state_path, SHARED_STATE_PATHS};
use codelet_test_helpers::{TestError, TestResult};

fn test_environment() -> Result<(), TestError> {
    static ENV: OnceLock<Result<(), String>> = OnceLock::new();
    ENV.get_or_init(|| {
        let home = codelet_test_helpers::shared_temp_directory()?.join("home");
        std::fs::create_dir_all(&home).map_err(|e| e.to_string())?;
        std::env::set_var("HOME", &home);
        Ok(())
    })
    .clone()
    .map_err(Into::into)
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

fn write(root: &Path, path: &str, content: &str) -> Result<(), TestError> {
    let full = root.join(path);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(full, content)?;
    Ok(())
}

fn read(root: &Path, path: &str) -> Result<String, TestError> {
    Ok(std::fs::read_to_string(root.join(path))?)
}

/// A fresh repo holding `files` in one commit.
fn repo_with(files: &[(&str, &str)]) -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::tempdir()?;
    git(dir.path(), &["init", "-q"])?;
    git(dir.path(), &["config", "user.name", "Test User"])?;
    git(dir.path(), &["config", "user.email", "test@example.com"])?;
    write(
        dir.path(),
        ".gitignore",
        ".fspec/\nspec/.fspec-write.lock\n",
    )?;
    for (path, content) in files {
        write(dir.path(), path, content)?;
    }
    git(dir.path(), &["add", "-A"])?;
    git(dir.path(), &["commit", "-q", "-m", "initial commit"])?;
    Ok(dir)
}

fn start_session(repo: &Path) -> Result<(String, PathBuf), TestError> {
    let session_id = uuid::Uuid::new_v4().to_string();
    codelet_git::create_worktree(repo, &session_id)?;
    let worktree = repo
        .join(codelet_git::FSPEC_WORKTREES_DIR)
        .join(&session_id);
    Ok((session_id, worktree))
}

fn merge(repo: &Path, session_id: &str) -> codelet_git::Result<codelet_git::MergeResult> {
    codelet_git::merge_session_with_strategy(
        repo,
        session_id,
        &codelet_rpc_types::MergeStrategy::FastForward,
    )
}

#[test]
fn a_merge_carries_the_sessions_code_and_features_and_leaves_mains_board_alone() -> TestResult {
    test_environment()?;
    // @step Given a temp git repo with spec/work-units.json, a feature and src/lib.rs committed, and an isolated session
    let repo = repo_with(&[
        ("spec/work-units.json", "{\"board\": \"base\"}\n"),
        ("spec/features/existing.feature", "Feature: Existing\n"),
        ("src/lib.rs", "pub fn base() {}\n"),
    ])?;
    let main = repo.path();
    let (session_id, worktree) = start_session(main)?;

    // @step And main committed a board change and then changed its board again in the working tree
    write(
        main,
        "spec/work-units.json",
        "{\"board\": \"main committed\"}\n",
    )?;
    git(main, &["commit", "-q", "-am", "main moves the board"])?;
    write(
        main,
        "spec/work-units.json",
        "{\"board\": \"main working tree\"}\n",
    )?;

    // @step And the session changed src/lib.rs and added spec/features/login.feature
    write(&worktree, "src/lib.rs", "pub fn session() {}\n")?;
    write(&worktree, "spec/features/login.feature", "Feature: Login\n")?;

    // @step When the session is inspected and merged
    let inspected = codelet_git::get_session_diff(main, &session_id)?;
    let merged = merge(main, &session_id)?;

    // @step Then the inspect lists src/lib.rs and spec/features/login.feature and no shared path
    assert_eq!(inspected.files_changed, vec!["src/lib.rs".to_string()]);
    assert_eq!(
        inspected.files_added,
        vec!["spec/features/login.feature".to_string()]
    );
    assert!(inspected.files_deleted.is_empty(), "{inspected:?}");
    assert!(
        !inspected.diff.contains("work-units.json"),
        "{}",
        inspected.diff
    );

    // @step And main has the session's src/lib.rs and spec/features/login.feature
    assert_eq!(read(main, "src/lib.rs")?, "pub fn session() {}\n");
    assert_eq!(
        read(main, "spec/features/login.feature")?,
        "Feature: Login\n"
    );

    // @step And main's spec/work-units.json still holds main's working-tree content
    assert_eq!(
        read(main, "spec/work-units.json")?,
        "{\"board\": \"main working tree\"}\n"
    );

    // @step And the merge commit changes src/lib.rs and spec/features/login.feature only
    let commit = merged
        .merge_commit
        .ok_or("the merge made no commit for the session's changes")?;
    let mut touched: Vec<String> =
        git(main, &["show", "--name-only", "--format=", commit.as_str()])?
            .lines()
            .map(str::to_string)
            .collect();
    touched.sort();
    assert_eq!(touched, vec!["spec/features/login.feature", "src/lib.rs"]);
    assert_eq!(
        git(main, &["show", "HEAD:spec/work-units.json"])?,
        "{\"board\": \"main committed\"}\n"
    );
    Ok(())
}

#[test]
fn a_worktree_copy_of_a_shared_file_that_was_changed_refuses_the_merge() -> TestResult {
    test_environment()?;
    // @step Given a temp git repo with spec/work-units.json, spec/tags.json and src/lib.rs committed, and an isolated session
    let repo = repo_with(&[
        ("spec/work-units.json", "{\"board\": \"base\"}\n"),
        ("spec/tags.json", "{\"tags\": []}\n"),
        ("src/lib.rs", "pub fn base() {}\n"),
    ])?;
    let main = repo.path();
    let (session_id, worktree) = start_session(main)?;
    let head_before = git(main, &["rev-parse", "HEAD"])?;

    // @step And the session changed src/lib.rs, edited its copy of spec/work-units.json and deleted its copy of spec/tags.json
    write(&worktree, "src/lib.rs", "pub fn session() {}\n")?;
    write(
        &worktree,
        "spec/work-units.json",
        "{\"board\": \"forked\"}\n",
    )?;
    std::fs::remove_file(worktree.join("spec/tags.json"))?;

    // @step When the session is merged
    let outcome = merge(main, &session_id);

    // @step Then the merge fails naming spec/tags.json and spec/work-units.json
    let error = outcome
        .err()
        .ok_or("the merge succeeded with changed shared files")?
        .to_string();
    assert!(error.contains("spec/tags.json"), "{error}");
    assert!(error.contains("spec/work-units.json"), "{error}");

    // @step And main's files and HEAD are unchanged
    assert_eq!(read(main, "src/lib.rs")?, "pub fn base() {}\n");
    assert_eq!(
        read(main, "spec/work-units.json")?,
        "{\"board\": \"base\"}\n"
    );
    assert_eq!(read(main, "spec/tags.json")?, "{\"tags\": []}\n");
    assert_eq!(git(main, &["rev-parse", "HEAD"])?, head_before);

    // @step And the session worktree still exists
    assert!(worktree.join("src/lib.rs").is_file());
    Ok(())
}

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            rust_sources(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

#[test]
fn the_merge_takes_shared_state_from_the_resolvers_definition() -> TestResult {
    // @step Given the codelet-git sources
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut sources = Vec::new();
    rust_sources(&src, &mut sources)?;
    let mut texts = Vec::new();
    for path in &sources {
        texts.push((path.clone(), std::fs::read_to_string(path)?));
    }

    // @step When they are scanned for the shared-state definition
    let users: Vec<&PathBuf> = texts
        .iter()
        .filter(|(_, text)| text.contains("codelet_common::spec_paths::is_shared_state_path"))
        .map(|(path, _)| path)
        .collect();

    // @step Then the merge filters paths with codelet_common::spec_paths::is_shared_state_path
    assert!(
        !users.is_empty(),
        "no codelet-git source uses is_shared_state_path"
    );
    assert!(is_shared_state_path(Path::new("spec/work-units.json")));

    // @step And no codelet-git source names a shared spec file
    let mut named = Vec::new();
    for shared in SHARED_STATE_PATHS {
        let name = shared.rsplit('/').next().unwrap_or(shared);
        for (path, text) in &texts {
            if text.contains(&format!("\"{shared}\"")) || text.contains(&format!("\"{name}\"")) {
                named.push(format!("{} names {shared}", path.display()));
            }
        }
    }
    assert!(named.is_empty(), "{named:#?}");
    Ok(())
}
