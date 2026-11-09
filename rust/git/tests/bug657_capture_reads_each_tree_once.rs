//! BUG-657: a changed-files capture reads each HEAD tree exactly once.
//!
//! Feature: spec/features/git-captures-read-each-tree-once-and-the-tui-draws-before-git-state.feature
//!
//! The staged pass resolved every index path from the root HEAD tree, so
//! each tree on the path was read and inflated again for every file under
//! it. These tests build a large fixture repository whose objects are all
//! loose and count the tree reads of one capture through a counting
//! `ObjectSource` wrapping the opened repository: an exact count, never a
//! clock.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

use codelet_git::status::{capture_changed_files_with, ChangedFileEntry, ObjectSource};
use codelet_test_helpers::{TestError, TestResult};

/// Counts every tree read, per object id, and forwards it to the repository.
struct CountingSource<'a> {
    repo: &'a gix::Repository,
    reads: RefCell<BTreeMap<String, usize>>,
}

impl<'a> CountingSource<'a> {
    fn new(repo: &'a gix::Repository) -> Self {
        Self {
            repo,
            reads: RefCell::new(BTreeMap::new()),
        }
    }

    fn total(&self) -> usize {
        self.reads.borrow().values().sum()
    }
}

impl ObjectSource for CountingSource<'_> {
    fn read_tree(&self, id: &gix::oid) -> codelet_git::Result<gix::objs::Tree> {
        *self.reads.borrow_mut().entry(id.to_string()).or_insert(0) += 1;
        self.repo.read_tree(id)
    }
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

fn write(root: &Path, path: &str, content: &str) -> TestResult {
    let full = root.join(path);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(full, content)?;
    Ok(())
}

/// The fixture's 1,200 tracked files: 408 features in one flat directory,
/// 150 attachment directories of 2 files, 10 identical attachment
/// directories (one tree object at ten paths), and 10 crates of 12 modules
/// of 4 sources each: 305 directories (the root included), 296 distinct
/// tree objects.
fn fixture_files() -> Vec<String> {
    let mut files = vec!["README.md".to_string(), "spec/work-units.json".to_string()];
    for feature in 0..408 {
        files.push(format!("spec/features/f-{feature:04}.feature"));
    }
    for card in 0..150 {
        files.push(format!("spec/attachments/CARD-{card:03}/note.md"));
        files.push(format!("spec/attachments/CARD-{card:03}/log.txt"));
    }
    for copy in 0..10 {
        files.push(format!("spec/attachments/SAME-{copy}/same.md"));
    }
    for krate in 0..10 {
        for module in 0..12 {
            for file in 0..4 {
                files.push(format!(
                    "rust/crate-{krate}/src/mod-{module:02}/file-{file}.rs"
                ));
            }
        }
    }
    files
}

/// A repository holding the fixture in three commits (so older versions of
/// the big trees are loose objects too), with every object loose.
fn fixture_repo() -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::tempdir()?;
    let root = dir.path();
    git(root, &["init", "-q"])?;
    git(root, &["config", "user.name", "Test User"])?;
    git(root, &["config", "user.email", "test@example.com"])?;
    git(root, &["config", "gc.auto", "0"])?;
    for round in 0..3 {
        for path in fixture_files() {
            let content = if path.ends_with("same.md") {
                format!("the same in every copy, round {round}\n")
            } else {
                format!("{path} round {round}\n")
            };
            write(root, &path, &content)?;
        }
        git(root, &["add", "-A"])?;
        git(root, &["commit", "-q", "-m", &format!("round {round}")])?;
    }
    Ok(dir)
}

/// The id of every tree in HEAD, the root included.
fn head_trees(root: &Path) -> Result<BTreeSet<String>, TestError> {
    let mut trees: BTreeSet<String> = git(
        root,
        &["ls-tree", "-r", "-d", "--format=%(objectname)", "HEAD"],
    )?
    .lines()
    .map(str::to_string)
    .collect();
    trees.insert(git(root, &["rev-parse", "HEAD^{tree}"])?.trim().to_string());
    Ok(trees)
}

fn loose_and_packed(root: &Path) -> Result<(usize, usize), TestError> {
    let counts = git(root, &["count-objects", "-v"])?;
    let field = |name: &str| -> Result<usize, TestError> {
        counts
            .lines()
            .find_map(|line| line.strip_prefix(name))
            .ok_or_else(|| format!("count-objects has no {name}"))?
            .trim()
            .parse::<usize>()
            .map_err(Into::into)
    };
    Ok((field("count:")?, field("in-pack:")?))
}

#[test]
fn a_changed_files_capture_reads_every_head_tree_exactly_once_in_a_large_loose_object_repo(
) -> TestResult {
    // @step Given a fixture repository with 1,200 tracked files in 305 directories, ten of them identical, whose objects are all loose
    let dir = fixture_repo()?;
    let root = dir.path();
    let tracked = git(root, &["ls-files"])?.lines().count();
    assert_eq!(tracked, 1_200, "tracked files in the fixture");
    let trees = head_trees(root)?;
    let directories = git(root, &["ls-tree", "-r", "-d", "HEAD"])?.lines().count() + 1;
    assert_eq!(directories, 305, "directories in HEAD, the root included");
    assert_eq!(trees.len(), 296, "distinct trees in HEAD");
    let (loose, packed) = loose_and_packed(root)?;
    assert_eq!(packed, 0, "every fixture object is loose");
    assert!(
        loose > 3 * tracked,
        "three commits of loose objects ({loose})"
    );
    // A staged edit makes the staged pass do real work.
    write(root, "spec/features/f-0001.feature", "edited\n")?;
    git(root, &["add", "spec/features/f-0001.feature"])?;

    // @step When one changed-files capture runs through a counting object source
    let repo = gix::open(root)?;
    let source = CountingSource::new(&repo);
    let changed = capture_changed_files_with(&repo, &source)?;
    assert_eq!(changed.len(), 1, "the staged edit is the only change");

    // @step Then every tree object of the HEAD commit is read exactly once, a tree at several paths included
    let reads = source.reads.borrow().clone();
    let read_more_than_once: Vec<(&String, &usize)> =
        reads.iter().filter(|(_, &count)| count != 1).collect();
    assert!(
        read_more_than_once.is_empty(),
        "trees read more than once ({} reads in all, {} trees in HEAD): {:?}",
        source.total(),
        trees.len(),
        read_more_than_once.iter().take(5).collect::<Vec<_>>()
    );

    // @step And no other tree is read
    let read: BTreeSet<String> = reads.keys().cloned().collect();
    assert_eq!(read, trees, "the capture reads exactly HEAD's trees");
    assert_eq!(source.total(), trees.len());
    Ok(())
}

/// `(path, letter, staged)` for each entry of `git status --porcelain`.
fn porcelain(root: &Path) -> Result<BTreeSet<(String, String, bool)>, TestError> {
    let mut out = BTreeSet::new();
    for line in git(root, &["status", "--porcelain", "--untracked-files=all"])?.lines() {
        let (codes, path) = line.split_at(3);
        let mut codes = codes.chars();
        let index = codes.next().ok_or("empty porcelain line")?;
        let worktree = codes.next().ok_or("short porcelain line")?;
        if index != ' ' && index != '?' {
            out.insert((path.to_string(), index.to_string(), true));
        }
        if worktree != ' ' {
            let letter = if worktree == '?' { 'A' } else { worktree };
            out.insert((path.to_string(), letter.to_string(), false));
        }
    }
    Ok(out)
}

fn entry(path: &str, change_type: &str, staged: bool) -> ChangedFileEntry {
    ChangedFileEntry {
        path: path.to_string(),
        change_type: change_type.to_string(),
        staged,
    }
}

#[test]
fn the_single_walk_returns_the_same_changed_files_list() -> TestResult {
    // @step Given the fixture repository with a staged edit, a staged new file, a staged deletion and an unstaged edit
    let dir = fixture_repo()?;
    let root = dir.path();
    write(root, "spec/features/f-0001.feature", "edited\n")?;
    git(root, &["add", "spec/features/f-0001.feature"])?;
    write(root, "spec/features/new.feature", "new\n")?;
    git(root, &["add", "spec/features/new.feature"])?;
    git(root, &["rm", "-q", "rust/crate-3/src/mod-02/file-1.rs"])?;
    write(root, "rust/crate-0/src/mod-00/file-0.rs", "unstaged edit\n")?;

    // @step When one changed-files capture runs through a counting object source
    let repo = gix::open(root)?;
    let source = CountingSource::new(&repo);
    let changed = capture_changed_files_with(&repo, &source)?;

    // @step Then it lists the staged edit as M, the staged new file as A and the staged deletion as D, all staged, in that order
    assert_eq!(
        changed[..3],
        [
            entry("spec/features/f-0001.feature", "M", true),
            entry("spec/features/new.feature", "A", true),
            entry("rust/crate-3/src/mod-02/file-1.rs", "D", true),
        ]
    );

    // @step And it then lists the unstaged edit as M, not staged
    assert_eq!(
        changed[3..],
        [entry("rust/crate-0/src/mod-00/file-0.rs", "M", false)]
    );

    // @step And the entries match what git status --porcelain reports for the same repository
    let captured: BTreeSet<(String, String, bool)> = changed
        .iter()
        .map(|e| (e.path.clone(), e.change_type.clone(), e.staged))
        .collect();
    assert_eq!(captured, porcelain(root)?);
    Ok(())
}
