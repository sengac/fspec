//! Blocking filesystem helpers for `init`: agent config persistence, home
//! directory resolution (BUG-533), and command-tagged `std::fs` wrappers.

use std::path::{Path, PathBuf};

use serde_json::json;

use crate::error::FspecCoreError;

/// Write spec/fspec-config.json recording `agent_id`, preserving any existing
/// keys (read-modify-write) — port of writeAgentConfig
/// (`src/utils/agentRuntimeConfig.ts:65-90`). 2-space-indented JSON. A
/// broken file is an error and is left as it was (BUG-640).
pub(super) fn write_agent_config(
    project_root: &Path,
    agent_id: &str,
) -> Result<(), FspecCoreError> {
    let spec_dir = crate::io::spec_paths::state_spec_dir(project_root)?;
    fs_create_dir_all(&spec_dir)?;
    let config_path = spec_dir.join("fspec-config.json");

    // BUG-640: one read-modify-write under the spec lock, replaced
    // atomically; an unreadable or non-object file is never written over.
    codelet_common::fspec_config::update_config_file(&config_path, |config| {
        let root = config.as_object_mut().ok_or_else(|| {
            FspecCoreError::Message("spec/fspec-config.json is not an object".into())
        })?;
        root.insert("agent".to_string(), json!(agent_id));
        Ok::<_, FspecCoreError>(())
    })
}

/// The platform whose home-directory convention `init` follows (BUG-533).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomePlatform {
    /// Linux, macOS and every other non-Windows platform.
    Unix,
    /// Windows.
    Windows,
}

impl HomePlatform {
    /// The platform this binary was built for.
    pub fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else {
            Self::Unix
        }
    }

    /// The variable holding the home directory, as Node's `os.homedir()`
    /// (the TypeScript original) reads it: `USERPROFILE` on Windows, `HOME`
    /// everywhere else.
    pub fn home_var(self) -> &'static str {
        match self {
            Self::Unix => "HOME",
            Self::Windows => "USERPROFILE",
        }
    }
}

/// The home directory `init` writes the Codex prompt to and reads the user
/// config from, resolved from an injected platform and environment so tests
/// never depend on the OS they run on or the machine's real home (BUG-533).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeEnv {
    platform: HomePlatform,
    home: Option<PathBuf>,
    /// BUG-640: the fspec user directory whose config `init` reads.
    user_dir: Result<PathBuf, String>,
}

impl HomeEnv {
    /// The running process: this platform's variable from the process env.
    /// BUG-640: its user directory is the one resolver's
    /// ([`codelet_common::user_dir::fspec_user_dir`]).
    pub fn process() -> Self {
        Self {
            user_dir: codelet_common::user_dir::fspec_user_dir(),
            ..Self::from_vars(HomePlatform::current(), |name| std::env::var_os(name))
        }
    }

    /// `platform` with the variables `lookup` returns; only the platform's
    /// own home variable is read, and an empty value counts as unset. The
    /// user directory is `<home>/.fspec`.
    pub fn from_vars(
        platform: HomePlatform,
        lookup: impl Fn(&str) -> Option<std::ffi::OsString>,
    ) -> Self {
        let home = lookup(platform.home_var())
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        let user_dir = codelet_common::user_dir::user_dir_from(None, home.as_deref());
        Self {
            platform,
            home,
            user_dir,
        }
    }

    /// The fspec user directory, or why it cannot be resolved.
    pub(super) fn user_dir(&self) -> Result<&Path, &str> {
        self.user_dir.as_deref().map_err(String::as_str)
    }

    /// The home directory, if the platform's variable is set.
    pub(super) fn home_dir(&self) -> Option<&Path> {
        self.home.as_deref()
    }

    /// The home directory, or the refusal naming the unset variable. Nothing
    /// falls back to the current or project directory.
    pub(super) fn require_home_dir(&self) -> Result<&Path, FspecCoreError> {
        self.home_dir().ok_or_else(|| {
            FspecCoreError::Message(format!(
                "Cannot find the home directory for the Codex prompt: {} is not set",
                self.platform.home_var()
            ))
        })
    }
}

/// Blocking `create_dir_all` mapping errors into the command-tagged variant.
pub(super) fn fs_create_dir_all(path: &Path) -> Result<(), FspecCoreError> {
    std::fs::create_dir_all(path).map_err(|source| FspecCoreError::Io {
        command: "init",
        source,
    })
}

/// Blocking `write` mapping errors into the command-tagged variant.
pub(super) fn fs_write(path: &Path, contents: &str) -> Result<(), FspecCoreError> {
    std::fs::write(path, contents).map_err(|source| FspecCoreError::Io {
        command: "init",
        source,
    })
}
