//! The commit gate (COV-077): a git pre-commit hook that runs only
//! `fspec audit --changed` and blocks the commit unless it exits 0. It prints
//! the audit's lines and never modifies files; there is no bypass and no
//! warn-only mode. `fspec init` and `fspec --sync-version` install it and
//! keep it current. A pre-commit hook fspec did not write is never
//! overwritten: fspec says the gate is not installed and names the line to
//! add. The hooks directory is git's own (`git rev-parse --git-path hooks`,
//! which honours core.hooksPath and linked worktrees).

use std::path::{Path, PathBuf};

use super::FSPEC_VERSION;
use crate::error::FspecCoreError;

/// The comment that marks a hook as fspec's.
const MARKER: &str = "# fspec commit gate: written by fspec ";

/// The line a hook fspec did not write needs to run the gate.
pub(crate) const GATE_LINE: &str = "fspec audit --changed || exit 1";

/// The hook fspec writes.
fn hook_text() -> String {
    format!(
        "#!/bin/sh\n{MARKER}{FSPEC_VERSION}; fspec init and fspec --sync-version keep it current.\n# It blocks the commit unless fspec audit --changed exits 0.\nexec fspec audit --changed\n"
    )
}

/// What installing the gate found.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Gate {
    /// fspec's hook was written (new or out of date).
    Installed(PathBuf),
    /// fspec's current hook was already there.
    Current,
    /// A hook fspec did not write is in the way.
    Foreign(PathBuf),
    /// The project root is not the top of a git working tree.
    NoRepository,
}

impl Gate {
    /// The line to print, if any.
    pub(crate) fn message(&self) -> Option<String> {
        match self {
            Gate::Installed(path) => Some(format!(
                "Installed the fspec commit gate: {} runs fspec audit --changed before every commit",
                path.display()
            )),
            Gate::Current => None,
            Gate::Foreign(path) => Some(format!(
                "The fspec commit gate is not installed: {} was not written by fspec. Add this line to it: {GATE_LINE}",
                path.display()
            )),
            Gate::NoRepository => Some(format!(
                "The fspec commit gate is not installed: the project is not the top of a git working tree. Add this line to its pre-commit hook: {GATE_LINE}"
            )),
        }
    }
}

/// `git <args>` in `project_root`: its trimmed stdout, or `None` when git
/// fails (not a repository).
fn git(project_root: &Path, args: &[&str]) -> Result<Option<String>, FspecCoreError> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(project_root)
        .output()
        .map_err(|source| FspecCoreError::Io {
            command: "init",
            source,
        })?;
    Ok(output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string()))
}

/// Install or refresh the gate in the git working tree at `project_root`.
pub(crate) fn install(project_root: &Path) -> Result<Gate, FspecCoreError> {
    let io = |source| FspecCoreError::Io {
        command: "init",
        source,
    };
    let Some(top) = git(project_root, &["rev-parse", "--show-toplevel"])? else {
        return Ok(Gate::NoRepository);
    };
    let same = |a: &Path, b: &Path| match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    };
    if !same(Path::new(&top), project_root) {
        return Ok(Gate::NoRepository);
    }
    let Some(hooks) = git(project_root, &["rev-parse", "--git-path", "hooks"])? else {
        return Ok(Gate::NoRepository);
    };
    let hooks = project_root.join(hooks);
    let path = hooks.join("pre-commit");
    let wanted = hook_text();
    match std::fs::read_to_string(&path) {
        Ok(existing) if existing == wanted => return Ok(Gate::Current),
        Ok(existing) if !existing.lines().any(|line| line.starts_with(MARKER)) => {
            return Ok(Gate::Foreign(path));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(io(error)),
    }
    std::fs::create_dir_all(&hooks).map_err(io)?;
    std::fs::write(&path, wanted).map_err(io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).map_err(io)?;
    }
    Ok(Gate::Installed(path))
}
