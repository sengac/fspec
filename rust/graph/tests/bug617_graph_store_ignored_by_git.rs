//! BUG-617: a graph store ignores itself, so an indexed repository shows no
//! graph files in git status.
//!
//! Feature: spec/features/the-code-graph-store-is-ignored-by-git.feature
//!
//! Every repository is a temp directory. git runs without the user's global
//! or system config, so no global excludes file can hide the graph.

use std::path::{Path, PathBuf};
use std::process::Command;

use codelet_graph::ast_index::dispatch_ast_index;
use codelet_graph::registry;
use codelet_test_helpers::{setup_temp_directory, TestError, TestResult};

fn git(root: &Path, args: &[&str]) -> Result<String, TestError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn repository() -> Result<(tempfile::TempDir, PathBuf), TestError> {
    let directory = setup_temp_directory()?;
    let root = directory.path().join("repo");
    std::fs::create_dir_all(&root)?;
    let root = root.canonicalize()?;
    git(&root, &["init", "--quiet"])?;
    std::fs::write(root.join("main.py"), "def entry():\n    return 1\n")?;
    Ok((directory, root))
}

async fn index(root: &Path) -> TestResult {
    let output: serde_json::Value =
        serde_json::from_str(&dispatch_ast_index(root, false, false).await)?;
    if let Some(error) = output.get("error") {
        return Err(format!("ast_index failed: {error}").into());
    }
    Ok(())
}

/// Every untracked path git status reports, one file per line.
fn untracked(root: &Path) -> Result<Vec<String>, TestError> {
    let status = git(root, &["status", "--porcelain", "--untracked-files=all"])?;
    Ok(status
        .lines()
        .filter_map(|line| line.strip_prefix("?? "))
        .map(str::to_owned)
        .collect())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_indexed_repository_shows_no_graph_files_in_git_status() -> TestResult {
    // @step Given a git repository with one Python file and no ignore rules
    let (_directory, root) = repository()?;

    // @step When ast_index runs for the repository
    index(&root).await?;
    assert!(registry::code_graph_path(&root).is_dir());

    let paths = untracked(&root)?;
    // @step Then git status lists the Python file as untracked
    assert!(paths.contains(&"main.py".to_owned()), "{paths:?}");

    // @step And git status lists nothing under .fspec
    assert_eq!(paths, vec!["main.py".to_owned()]);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_graph_directory_made_before_the_fix_is_ignored_after_its_next_index() -> TestResult {
    // @step Given a git repository whose .fspec/graph directory holds a code graph and no .gitignore
    let (_directory, root) = repository()?;
    index(&root).await?;
    let graph_directory = registry::code_graph_path(&root)
        .parent()
        .ok_or("code graph path has no parent")?
        .to_path_buf();
    let ignore_file = graph_directory.join(".gitignore");
    if ignore_file.exists() {
        std::fs::remove_file(&ignore_file)?;
    }
    assert!(
        untracked(&root)?
            .iter()
            .any(|path| path.starts_with(".fspec/")),
        "the graph is not visible to git without the ignore file"
    );

    // @step When ast_index runs for the repository
    index(&root).await?;

    // @step Then git status lists nothing under .fspec
    assert_eq!(untracked(&root)?, vec!["main.py".to_owned()]);
    Ok(())
}
