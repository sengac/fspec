//! The files `fspec audit` reads (COV-077): the working tree, or, under
//! `--changed`, the git index, so the commit gate audits exactly what the
//! commit holds. Every read of the audit goes through one [`Source`], so all
//! areas see the same snapshot; a file staged as deleted is missing.

use std::path::Path;

use crate::error::FspecCoreError;

/// Where the audit reads files from.
pub(crate) enum Source {
    WorkTree,
    Staged(Box<codelet_git::StagedFiles>),
}

fn git_error(error: codelet_git::GitError) -> FspecCoreError {
    FspecCoreError::InvalidArgs {
        command: "audit",
        reason: format!("cannot read the git index: {error}"),
    }
}

impl Source {
    /// The git index of the working tree at `project_root`, which must be
    /// the top of that working tree (staged paths are relative to it).
    pub(crate) fn staged(project_root: &Path) -> Result<Self, FspecCoreError> {
        if !project_root.join(".git").exists() {
            return Err(FspecCoreError::InvalidArgs {
                command: "audit",
                reason: format!(
                    "--changed audits the staged changes, but {} is not the top of a git working tree",
                    project_root.display()
                ),
            });
        }
        codelet_git::StagedFiles::open(project_root)
            .map(|staged| Source::Staged(Box::new(staged)))
            .map_err(git_error)
    }

    /// Whether the project-relative `path` exists.
    pub(crate) fn exists(&self, project_root: &Path, path: &str) -> bool {
        match self {
            Source::WorkTree => project_root.join(path).exists(),
            Source::Staged(staged) => staged.contains(path),
        }
    }

    /// The bytes of `path`; `None` when it does not exist.
    pub(crate) fn read_bytes(
        &self,
        project_root: &Path,
        path: &str,
    ) -> Result<Option<Vec<u8>>, FspecCoreError> {
        match self {
            Source::WorkTree => match std::fs::read(project_root.join(path)) {
                Ok(bytes) => Ok(Some(bytes)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(source) => Err(FspecCoreError::Io {
                    command: "audit",
                    source,
                }),
            },
            Source::Staged(staged) => staged.read(path).map_err(git_error),
        }
    }

    /// The text of `path`; `None` when it does not exist. Text that is not
    /// UTF-8 is an error naming the file.
    pub(crate) fn read(
        &self,
        project_root: &Path,
        path: &str,
    ) -> Result<Option<String>, FspecCoreError> {
        self.read_bytes(project_root, path)?
            .map(|bytes| {
                String::from_utf8(bytes).map_err(|error| FspecCoreError::InvalidArgs {
                    command: "audit",
                    reason: format!("{path} is not UTF-8 text: {error}"),
                })
            })
            .transpose()
    }

    /// The project-relative paths under `spec/features/` ending in `suffix`,
    /// sorted.
    pub(crate) fn spec_files(
        &self,
        project_root: &Path,
        suffix: &str,
    ) -> Result<Vec<String>, FspecCoreError> {
        let mut files: Vec<String> = match self {
            Source::WorkTree => {
                let mut found = Vec::new();
                let mut pending = vec![crate::io::spec_paths::features_dir_of(project_root)];
                while let Some(directory) = pending.pop() {
                    let entries = match std::fs::read_dir(&directory) {
                        Ok(entries) => entries,
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                        Err(source) => {
                            return Err(FspecCoreError::Io {
                                command: "audit",
                                source,
                            })
                        }
                    };
                    for entry in entries {
                        let path = entry
                            .map_err(|source| FspecCoreError::Io {
                                command: "audit",
                                source,
                            })?
                            .path();
                        if path.is_dir() {
                            pending.push(path);
                        } else if let Ok(relative) = path.strip_prefix(project_root) {
                            let relative = relative.to_string_lossy().replace('\\', "/");
                            if relative.ends_with(suffix) {
                                found.push(relative);
                            }
                        }
                    }
                }
                found
            }
            Source::Staged(staged) => staged
                .paths()
                .filter(|path| path.starts_with("spec/features/") && path.ends_with(suffix))
                .map(str::to_string)
                .collect(),
        };
        files.sort();
        Ok(files)
    }
}
