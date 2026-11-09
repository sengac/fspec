//! BUG-592: the untracked-file listing works without an index and never walks
//! ignored directories.
//!
//! Feature: spec/features/untracked-file-listing-works-without-an-index-and-never-walks-ignored-directories.feature
//!
//! Every repository is a fresh temp repo with a repo-local identity; every
//! git call ignores the user's global and system config. An unreadable
//! directory (mode 000) is the exact proof the walk did or did not enter it:
//! reading it fails, so a listing that succeeds never read it.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use codelet_git::status::get_untracked_files;
use codelet_test_helpers::{TestError, TestResult};

/// Run `git` in `cwd` without the user's global or system config; fail the
/// test on a non-zero exit.
fn git(cwd: &Path, args: &[&str]) -> Result<(), TestError> {
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
    Ok(())
}

/// A fresh repository with a repo-local identity.
fn init_repo(root: &Path) -> TestResult {
    git(root, &["init", "-q"])?;
    git(root, &["config", "user.name", "Test User"])?;
    git(root, &["config", "user.email", "test@example.com"])?;
    Ok(())
}

/// A directory made unreadable (mode 000) for the guard's lifetime; dropping
/// the guard makes it readable again so the temp dir can be removed.
struct Unreadable(PathBuf);

impl Unreadable {
    fn new(dir: PathBuf) -> Result<Self, TestError> {
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("inside.txt"), "inside\n")?;
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o000))?;
        let guard = Unreadable(dir);
        // Precondition: the directory really cannot be read (a root user
        // could read it, which would make the scenario prove nothing).
        if fs::read_dir(&guard.0).is_ok() {
            return Err(format!("{} is still readable", guard.0.display()).into());
        }
        Ok(guard)
    }
}

impl Drop for Unreadable {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o755));
    }
}

#[test]
fn a_fresh_repository_with_no_index_lists_every_file_the_ignore_rules_do_not_exclude() -> TestResult
{
    // @step Given a fresh git repository with no index file
    let dir = tempfile::tempdir()?;
    let root = dir.path().canonicalize()?;
    init_repo(&root)?;
    assert!(!root.join(".git").join("index").exists());

    // @step And its .gitignore holds "build/" and it has the files a.txt and build/out.bin
    fs::write(root.join(".gitignore"), "build/\n")?;
    fs::write(root.join("a.txt"), "a\n")?;
    fs::create_dir_all(root.join("build"))?;
    fs::write(root.join("build").join("out.bin"), "out\n")?;

    // @step When its untracked files are listed
    let listed = get_untracked_files(&root);

    // @step Then the listing succeeds with exactly .gitignore and a.txt
    let mut listed = listed?;
    listed.sort();
    assert_eq!(listed, vec![".gitignore".to_string(), "a.txt".to_string()]);
    Ok(())
}

#[test]
fn the_walk_never_enters_an_ignored_directory() -> TestResult {
    // @step Given a git repository with a commit whose .gitignore holds "target/"
    let dir = tempfile::tempdir()?;
    let root = dir.path().canonicalize()?;
    init_repo(&root)?;
    fs::write(root.join(".gitignore"), "target/\n")?;
    git(&root, &["add", ".gitignore"])?;
    git(&root, &["commit", "-q", "-m", "ignore target"])?;

    // @step And an untracked file notes.txt and a directory target/ that cannot be read
    fs::write(root.join("notes.txt"), "notes\n")?;
    let _target = Unreadable::new(root.join("target"))?;

    // @step When its untracked files are listed
    let listed = get_untracked_files(&root);

    // @step Then the listing succeeds with exactly notes.txt
    assert_eq!(listed?, vec!["notes.txt".to_string()]);
    Ok(())
}

#[test]
fn a_directory_that_is_not_ignored_and_cannot_be_read_is_an_error_naming_it() -> TestResult {
    // @step Given a git repository with a commit and no ignore rule for notes/
    let dir = tempfile::tempdir()?;
    let root = dir.path().canonicalize()?;
    init_repo(&root)?;
    fs::write(root.join("README.md"), "readme\n")?;
    git(&root, &["add", "README.md"])?;
    git(&root, &["commit", "-q", "-m", "initial commit"])?;

    // @step And a directory notes/ that cannot be read
    let _notes = Unreadable::new(root.join("notes"))?;

    // @step When its untracked files are listed
    let listed = get_untracked_files(&root);

    // @step Then the listing fails with an error that names notes
    let Err(error) = listed else {
        return Err(format!("the listing succeeded: {listed:?}").into());
    };
    let message = error.to_string();
    assert!(message.contains("notes"), "{message}");
    Ok(())
}
