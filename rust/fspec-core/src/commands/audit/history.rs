//! BUG-591: the git facts `fspec audit --fix` needs before it records the
//! line text of a legacy implementation mapping (one without `firstLine` and
//! `lastLine`): the commit that linked the mapping, and the implementation
//! file's lines at that commit. BUG-635: the same commit is where a drifted
//! mapping is carried from, through the line diff between the file then and
//! now ([`History::carry`]). BUG-666: every committed mapping whose anchor
//! lines still read their text is carried too, so a range that grew or
//! shrank under them is reported ([`History::range_change`]).
//!
//! Feature: spec/features/audit-fix-records-line-text-only-where-git-shows-the-lines-did-not-move.feature
//!
//! The linking commit is the oldest commit, walking back from HEAD through
//! the commits that changed the coverage file (`git log --first-parent`),
//! from which every later version of the coverage file holds the mapping
//! (the same scenario, test file, implementation file and line list). The
//! mapping's lines did not move when lines 1 through its last line read
//! exactly as they did at that commit. git is asked only for exact facts
//! (commit lists and file contents); nothing is inferred about where code
//! moved. BUG-666: git runs in a fixed number of processes per audit run —
//! one `git log --first-parent --name-only` for every path's commits and one
//! `git cat-file --batch` for every file version — and the line diff
//! ([`super::carry::line_diff`], Myers) is computed in-process. Every answer
//! is read once per run.

use std::cell::{OnceCell, RefCell};
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::rc::Rc;

use crate::error::FspecCoreError;
use crate::types::coverage::CoverageFile;

/// Whether a legacy mapping's lines are verified not to have moved.
pub(crate) enum Verified {
    /// Lines 1 through the mapping's last line read as at the linking commit.
    Unmoved,
    /// Why git cannot show that the lines did not move: the fact the
    /// problem adds.
    Unverified(String),
}

/// BUG-635: where a mapping lies now, carried through the git line diff, or
/// why git cannot show that exactly (the fact the problem adds).
pub(crate) enum Carried {
    Lines(Vec<i64>),
    Reported(String),
}

/// One implementation mapping as a coverage file holds it.
pub(crate) struct Mapping<'a> {
    pub(crate) scenario: &'a str,
    pub(crate) test_file: &'a str,
    pub(crate) impl_file: &'a str,
    pub(crate) lines: &'a [i64],
}

/// Answers keyed by (commit, path).
type AtCommit<T> = RefCell<HashMap<(String, String), Option<Rc<T>>>>;

/// One implementation mapping as a set key: scenario, test file,
/// implementation file and line list.
type MappingKey = (String, String, String, Vec<i64>);

/// BUG-666: the first-parent history read once per run.
#[derive(Default)]
struct Log {
    /// Commits, newest first.
    commits: Vec<String>,
    /// The indexes (into `commits`, newest first) of the commits that changed
    /// each path, relative to the project root.
    by_path: HashMap<String, Vec<usize>>,
    /// The project root relative to the top of the working tree (`git
    /// rev-parse --show-prefix`), prefixed to paths in object names.
    prefix: String,
}

/// BUG-666: one `git cat-file --batch` process answering every file read.
struct CatFile {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl CatFile {
    fn start(project_root: &Path) -> std::io::Result<Self> {
        let mut child = Command::new("git")
            .args(["cat-file", "--batch"])
            .current_dir(project_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let stdin = child.stdin.take();
        let stdout = child
            .stdout
            .take()
            .map(BufReader::new)
            .ok_or_else(|| std::io::Error::other("git cat-file has no stdout"))?;
        Ok(Self {
            child,
            stdin,
            stdout,
        })
    }

    /// The bytes of object `name` (`<commit>:<path>`); `None` when missing.
    fn read(&mut self, name: &str) -> std::io::Result<Option<Vec<u8>>> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| std::io::Error::other("git cat-file stdin is closed"))?;
        writeln!(stdin, "{name}")?;
        stdin.flush()?;
        let mut header = String::new();
        self.stdout.read_line(&mut header)?;
        let header = header.trim_end();
        if header.ends_with(" missing") || header.ends_with(" ambiguous") {
            return Ok(None);
        }
        let size: usize = header
            .rsplit(' ')
            .next()
            .and_then(|size| size.parse().ok())
            .ok_or_else(|| std::io::Error::other(format!("git cat-file printed '{header}'")))?;
        let mut body = vec![0; size + 1];
        self.stdout.read_exact(&mut body)?;
        body.truncate(size);
        Ok(Some(body))
    }
}

impl Drop for CatFile {
    fn drop(&mut self) {
        drop(self.stdin.take());
        let _ = self.child.wait();
    }
}

/// The git answers of one audit run.
#[derive(Default)]
pub(crate) struct History {
    /// The first-parent history (BUG-666: read once).
    log: OnceCell<Rc<Log>>,
    /// The cat-file process, started on the first read.
    cat: RefCell<Option<CatFile>>,
    /// A file's text at a commit (`None`: the commit has no such file).
    files: AtCommit<String>,
    /// The mappings of a coverage file at a commit (`None`: missing or
    /// unparsable).
    coverage: AtCommit<HashSet<MappingKey>>,
}

impl History {
    /// Whether `mapping` of `coverage_file` (both relative to
    /// `project_root`) still lies where it was linked: lines 1 through its
    /// last line of `current` (the implementation file's text now) read as
    /// at the linking commit.
    pub(crate) fn verify(
        &self,
        project_root: &Path,
        coverage_file: &str,
        mapping: &Mapping<'_>,
        current: &str,
    ) -> Result<Verified, FspecCoreError> {
        if !project_root.join(".git").exists() {
            return Ok(Verified::Unverified(
                "the project is not a git working tree, so git cannot show that its lines did not move"
                    .to_string(),
            ));
        }
        let Some(commit) = self.linking_commit(project_root, coverage_file, mapping)? else {
            return Ok(Verified::Unverified(
                "the mapping is not committed, so git cannot show that its lines did not move"
                    .to_string(),
            ));
        };
        let last = mapping.lines.iter().copied().max().unwrap_or(0);
        let then = self.file(project_root, &commit, mapping.impl_file)?;
        let unmoved = then.is_some_and(|then| head(&then, last) == head(current, last));
        Ok(if unmoved {
            Verified::Unmoved
        } else {
            Verified::Unverified(format!(
                "{} lines 1-{last} changed since the mapping was linked in commit {}",
                mapping.impl_file,
                &commit[..commit.len().min(12)]
            ))
        })
    }

    /// BUG-635: `mapping` (whose recorded text is `first_line` and
    /// `last_line`) carried through the line diff from the file at the commit
    /// that linked it to `content`, the file the audit reads (whose trimmed
    /// lines are `current`). The carried first and last lines must read the
    /// recorded text.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn carry(
        &self,
        project_root: &Path,
        coverage_file: &str,
        mapping: &Mapping<'_>,
        first_line: &str,
        last_line: &str,
        content: &str,
        current: &[&str],
    ) -> Result<Carried, FspecCoreError> {
        if !project_root.join(".git").exists() {
            return Ok(Carried::Reported(
                "the project is not a git working tree, so git cannot show how its lines moved"
                    .to_string(),
            ));
        }
        let Some(commit) = self.linking_commit(project_root, coverage_file, mapping)? else {
            return Ok(Carried::Reported(
                "the mapping is not committed, so git cannot show how its lines moved".to_string(),
            ));
        };
        let short = &commit[..commit.len().min(12)];
        let first = mapping.lines.iter().copied().min().unwrap_or(0);
        let last = mapping.lines.iter().copied().max().unwrap_or(0);
        let then = self.file(project_root, &commit, mapping.impl_file)?;
        let then = then.as_deref().map(String::as_str).unwrap_or_default();
        if !reads(then, first, first_line) || !reads(then, last, last_line) {
            return Ok(Carried::Reported(format!(
                "lines {first} and {last} did not read the recorded text at commit {short} either, so git cannot show how they moved"
            )));
        }
        Ok(carried(
            mapping.lines,
            then,
            content,
            current,
            first_line,
            last_line,
            short,
        ))
    }

    /// BUG-666: for a committed `mapping` whose anchors still read
    /// `first_line` and `last_line` in `content` (trimmed lines `current`),
    /// whether the line diff since its linking commit changed its range:
    /// `None` when it did not, or when git cannot carry it (not a git working
    /// tree, not committed, or not reading its text at the linking commit;
    /// the text check alone applies, as before).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn range_change(
        &self,
        project_root: &Path,
        coverage_file: &str,
        mapping: &Mapping<'_>,
        first_line: &str,
        last_line: &str,
        content: &str,
        current: &[&str],
    ) -> Result<Option<Carried>, FspecCoreError> {
        if !project_root.join(".git").exists() {
            return Ok(None);
        }
        // A file whose last commit is no newer than the coverage file's
        // first one, and which reads now as at HEAD, cannot have changed
        // since any version of the coverage file linked the mapping.
        let log = self.log(project_root)?;
        let file_newest = log.by_path.get(mapping.impl_file).and_then(|c| c.first());
        let coverage_oldest = log.by_path.get(coverage_file).and_then(|c| c.last());
        if let (Some(file_newest), Some(coverage_oldest), Some(head)) =
            (file_newest, coverage_oldest, log.commits.first())
        {
            if file_newest >= coverage_oldest
                && self
                    .file(project_root, head, mapping.impl_file)?
                    .is_some_and(|text| text.as_str() == content)
            {
                return Ok(None);
            }
        }
        let Some(commit) = self.linking_commit(project_root, coverage_file, mapping)? else {
            return Ok(None);
        };
        let Some(then) = self.file(project_root, &commit, mapping.impl_file)? else {
            return Ok(None);
        };
        if then.as_str() == content {
            return Ok(None);
        }
        let first = mapping.lines.iter().copied().min().unwrap_or(0);
        let last = mapping.lines.iter().copied().max().unwrap_or(0);
        if !reads(&then, first, first_line) || !reads(&then, last, last_line) {
            return Ok(None);
        }
        let short = &commit[..commit.len().min(12)];
        let mut mapped = mapping.lines.to_vec();
        mapped.sort_unstable();
        mapped.dedup();
        Ok(
            match carried(
                mapping.lines,
                &then,
                content,
                current,
                first_line,
                last_line,
                short,
            ) {
                Carried::Lines(lines) if lines == mapped => None,
                change => Some(change),
            },
        )
    }

    /// The commit that linked `mapping`, or `None` when HEAD's coverage file
    /// does not hold it.
    fn linking_commit(
        &self,
        project_root: &Path,
        coverage_file: &str,
        mapping: &Mapping<'_>,
    ) -> Result<Option<String>, FspecCoreError> {
        let log = self.log(project_root)?;
        let key: MappingKey = (
            mapping.scenario.to_string(),
            mapping.test_file.to_string(),
            mapping.impl_file.to_string(),
            mapping.lines.to_vec(),
        );
        let mut linked = None;
        for index in log.by_path.get(coverage_file).into_iter().flatten() {
            let commit = &log.commits[*index];
            let holds = self
                .coverage(project_root, commit, coverage_file)?
                .is_some_and(|mappings| mappings.contains(&key));
            if !holds {
                break;
            }
            linked = Some(commit.clone());
        }
        Ok(linked)
    }

    /// BUG-666: every path's first-parent commits, read with one `git log`.
    fn log(&self, project_root: &Path) -> Result<Rc<Log>, FspecCoreError> {
        if let Some(log) = self.log.get() {
            return Ok(Rc::clone(log));
        }
        let log = Rc::new(read_log(project_root)?);
        Ok(Rc::clone(self.log.get_or_init(|| log)))
    }

    fn file(
        &self,
        project_root: &Path,
        commit: &str,
        path: &str,
    ) -> Result<Option<Rc<String>>, FspecCoreError> {
        let key = (commit.to_string(), path.to_string());
        if let Some(text) = self.files.borrow().get(&key) {
            return Ok(text.clone());
        }
        let prefix = self.log(project_root)?.prefix.clone();
        let mut cat = self.cat.borrow_mut();
        if cat.is_none() {
            *cat = Some(CatFile::start(project_root).map_err(|error| cat_error(&error))?);
        }
        let bytes = match cat.as_mut() {
            Some(cat) => cat
                .read(&format!("{commit}:{prefix}{path}"))
                .map_err(|error| cat_error(&error))?,
            None => None,
        };
        drop(cat);
        let text = bytes.map(|bytes| Rc::new(String::from_utf8_lossy(&bytes).into_owned()));
        self.files.borrow_mut().insert(key, text.clone());
        Ok(text)
    }

    fn coverage(
        &self,
        project_root: &Path,
        commit: &str,
        coverage_file: &str,
    ) -> Result<Option<Rc<HashSet<MappingKey>>>, FspecCoreError> {
        let key = (commit.to_string(), coverage_file.to_string());
        if let Some(coverage) = self.coverage.borrow().get(&key) {
            return Ok(coverage.clone());
        }
        let coverage = self
            .file(project_root, commit, coverage_file)?
            .and_then(|text| serde_json::from_str::<CoverageFile>(&text).ok())
            .map(|coverage| Rc::new(mapping_keys(&coverage)));
        self.coverage.borrow_mut().insert(key, coverage.clone());
        Ok(coverage)
    }
}

/// `mapping`'s lines carried through the line diff from `then` to `content`
/// (trimmed lines `current`), checked to read `first_line` and `last_line`.
fn carried(
    lines: &[i64],
    then: &str,
    content: &str,
    current: &[&str],
    first_line: &str,
    last_line: &str,
    short: &str,
) -> Carried {
    let hunks = super::carry::line_diff(then, content);
    let carried = match super::carry::carry(lines, &hunks) {
        Ok(carried) => carried,
        Err(reason) => return Carried::Reported(format!("{reason} since commit {short}")),
    };
    let (Some(new_first), Some(new_last)) = (carried.first(), carried.last()) else {
        return Carried::Reported(format!("every mapped line changed since commit {short}"));
    };
    if line_at(current, *new_first) != Some(first_line)
        || line_at(current, *new_last) != Some(last_line)
    {
        return Carried::Reported(format!(
            "carried through the diff since commit {short} to lines {new_first}-{new_last}, which do not read the recorded text"
        ));
    }
    Carried::Lines(carried)
}

/// Whether line `line` of `text`, trimmed, reads `expected`.
fn reads(text: &str, line: i64, expected: &str) -> bool {
    usize::try_from(line - 1)
        .ok()
        .and_then(|index| text.lines().nth(index))
        .is_some_and(|found| found.trim() == expected)
}

fn cat_error(error: &std::io::Error) -> FspecCoreError {
    FspecCoreError::InvalidArgs {
        command: "audit",
        reason: format!("cannot read files from git (git cat-file --batch): {error}"),
    }
}

/// Every implementation mapping of `coverage` as a set key.
fn mapping_keys(coverage: &CoverageFile) -> HashSet<MappingKey> {
    coverage
        .scenarios
        .iter()
        .flat_map(|scenario| {
            scenario.test_mappings.iter().flat_map(move |test| {
                test.impl_mappings.iter().filter_map(move |implementation| {
                    super::impl_ranges::impl_lines(&implementation.lines).map(|lines| {
                        (
                            scenario.name.clone(),
                            test.file.clone(),
                            implementation.file.clone(),
                            lines,
                        )
                    })
                })
            })
        })
        .collect()
}

/// BUG-666: one `git log --first-parent --format=%x01%H --name-only` over
/// the whole history, keyed by paths relative to `project_root`; empty for a
/// repository without commits.
fn read_log(project_root: &Path) -> Result<Log, FspecCoreError> {
    let has_head = git(project_root, &["rev-parse", "--verify", "--quiet", "HEAD"])?
        .status
        .success();
    if !has_head {
        return Ok(Log::default());
    }
    let failed = |what: &str, output: &std::process::Output| FspecCoreError::InvalidArgs {
        command: "audit",
        reason: format!(
            "git {what} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ),
    };
    let prefix_output = git(project_root, &["rev-parse", "--show-prefix"])?;
    if !prefix_output.status.success() {
        return Err(failed("rev-parse --show-prefix", &prefix_output));
    }
    let prefix = String::from_utf8_lossy(&prefix_output.stdout)
        .trim()
        .to_string();
    let output = git(
        project_root,
        &[
            "-c",
            "core.quotePath=false",
            "log",
            "--first-parent",
            "--no-renames",
            "--format=%x01%H",
            "--name-only",
        ],
    )?;
    if !output.status.success() {
        return Err(failed("log", &output));
    }
    let mut log = Log {
        prefix: prefix.clone(),
        ..Log::default()
    };
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if let Some(commit) = line.strip_prefix('\u{1}') {
            log.commits.push(commit.to_string());
        } else if let (Some(path), Some(index)) = (
            line.strip_prefix(prefix.as_str())
                .filter(|path| !path.is_empty()),
            log.commits.len().checked_sub(1),
        ) {
            log.by_path.entry(path.to_string()).or_default().push(index);
        }
    }
    Ok(log)
}

/// Line `line` (1-based) of `text`.
fn line_at<'a>(text: &[&'a str], line: i64) -> Option<&'a str> {
    usize::try_from(line - 1)
        .ok()
        .and_then(|index| text.get(index))
        .copied()
}

/// Lines 1 through `last` of `text`, split on `\n` exactly.
fn head(text: &str, last: i64) -> Vec<&str> {
    let count = usize::try_from(last).unwrap_or(0);
    text.split('\n').take(count).collect()
}

/// Run git in `project_root`; an error when git cannot be run.
fn git(project_root: &Path, args: &[&str]) -> Result<std::process::Output, FspecCoreError> {
    Command::new("git")
        .args(args)
        .current_dir(project_root)
        .output()
        .map_err(|error| FspecCoreError::InvalidArgs {
            command: "audit",
            reason: format!("cannot run git {}: {error}", args.join(" ")),
        })
}
