//! BUG-627: test daemons never touch the real runtime dir.
//!
//! Feature: spec/features/test-daemons-never-touch-the-real-runtime-dir.feature
//!
//! `fspec daemon` and combined mode write `daemon.json` and
//! `daemons/<pid>.json` under `$XDG_RUNTIME_DIR/fspec` (else `~/.fspec`)
//! and their logs and data under `~/.fspec`. A test that starts one with
//! the test process's own environment replaces the user's real
//! default-daemon pointer. Every daemon, combined-mode and PTY spawn of the
//! fspec binary is built with [`IsolatedCommand`] (or
//! [`fspec_pty_command`]), which refuses to start a process unless HOME is
//! set inside the system temp dir and XDG_RUNTIME_DIR is set inside it or
//! removed ([`check_isolated`]).

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};

use tempfile::TempDir;

/// A temp root holding the HOME (`home/`) and XDG_RUNTIME_DIR
/// (`runtime/`) a test daemon uses. Declare it before the daemon's
/// [`super::ChildGuard`] so the daemon stops before the dirs go.
pub struct TestHome {
    root: TempDir,
}

impl TestHome {
    pub fn new() -> Self {
        let root = tempfile::tempdir().expect("BUG-627: create the test home");
        std::fs::create_dir_all(root.path().join("home")).expect("BUG-627: mkdir home");
        std::fs::create_dir_all(root.path().join("runtime")).expect("BUG-627: mkdir runtime");
        Self { root }
    }

    /// The daemon's HOME.
    pub fn home(&self) -> PathBuf {
        self.root.path().join("home")
    }

    /// The daemon's XDG_RUNTIME_DIR.
    pub fn runtime_dir(&self) -> PathBuf {
        self.root.path().join("runtime")
    }

    /// `<runtime>/fspec/daemon.json`.
    pub fn daemon_json(&self) -> PathBuf {
        self.runtime_dir().join("fspec").join("daemon.json")
    }

    /// `<runtime>/fspec/daemons`.
    pub fn daemons_dir(&self) -> PathBuf {
        self.runtime_dir().join("fspec").join("daemons")
    }
}

impl Default for TestHome {
    fn default() -> Self {
        Self::new()
    }
}

/// How a spawn treats one variable.
enum Setting<'a> {
    /// Not touched: the child inherits the test process's value.
    Inherited,
    /// Removed for the child.
    Removed,
    /// Set for the child.
    Set(&'a OsStr),
}

/// The system temp dir, where every test temp dir lives.
fn temp_root() -> PathBuf {
    let temp = std::env::temp_dir();
    temp.canonicalize().unwrap_or(temp)
}

fn inside_temp_root(value: &OsStr) -> bool {
    let path = Path::new(value);
    if !path.is_absolute() {
        return false;
    }
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let root = temp_root();
    path.starts_with(&root) && path != root
}

fn check(home: Setting<'_>, runtime: Setting<'_>) -> Result<(), String> {
    let root = temp_root();
    match home {
        Setting::Set(value) if inside_temp_root(value) => {}
        Setting::Set(value) => {
            return Err(format!(
                "refusing to start a test daemon: HOME={} is not inside the temp dir {}",
                Path::new(value).display(),
                root.display()
            ))
        }
        Setting::Removed => {
            return Err("refusing to start a test daemon: HOME is removed; set it to a temp dir".to_owned())
        }
        Setting::Inherited => {
            return Err(
                "refusing to start a test daemon: HOME is inherited from the test process; set it to a temp dir"
                    .to_owned(),
            )
        }
    }
    match runtime {
        Setting::Removed => Ok(()),
        Setting::Set(value) if inside_temp_root(value) => Ok(()),
        Setting::Set(value) => Err(format!(
            "refusing to start a test daemon: XDG_RUNTIME_DIR={:?} is not inside the temp dir {}",
            Path::new(value).display().to_string(),
            root.display()
        )),
        Setting::Inherited => Err(
            "refusing to start a test daemon: XDG_RUNTIME_DIR is inherited from the test process; set it to a temp dir or remove it"
                .to_owned(),
        ),
    }
}

/// BUG-627: refuse `command` unless HOME is set inside the temp dir and
/// XDG_RUNTIME_DIR is set inside it or removed.
pub fn check_isolated(command: &Command) -> Result<(), String> {
    let setting = |key: &str| match command
        .get_envs()
        .find(|(name, _)| *name == OsStr::new(key))
    {
        None => Setting::Inherited,
        Some((_, None)) => Setting::Removed,
        Some((_, Some(value))) => Setting::Set(value),
    };
    check(setting("HOME"), setting("XDG_RUNTIME_DIR"))
}

/// BUG-627: [`check_isolated`] for a PTY command, which starts from a copy
/// of the test process's environment and records what the test set.
pub fn check_isolated_pty(command: &portable_pty::CommandBuilder) -> Result<(), String> {
    let set_by_test = |key: &str| command.iter_extra_env_as_str().any(|(name, _)| name == key);
    let setting = |key: &'static str| match command.get_env(key) {
        None => Setting::Removed,
        Some(value) if set_by_test(key) => Setting::Set(value),
        Some(_) => Setting::Inherited,
    };
    check(setting("HOME"), setting("XDG_RUNTIME_DIR"))
}

/// BUG-627: a PTY command for the fspec binary with `home`'s HOME and
/// XDG_RUNTIME_DIR, checked by [`check_isolated_pty`].
pub fn fspec_pty_command(home: &TestHome) -> portable_pty::CommandBuilder {
    let mut command = portable_pty::CommandBuilder::new(super::fspec_bin());
    command.env("HOME", home.home());
    // BUG-640: a test build resolves the user directory only from
    // FSPEC_USER_DIR.
    command.env("FSPEC_USER_DIR", home.home().join(".fspec"));
    command.env("XDG_RUNTIME_DIR", home.runtime_dir());
    if let Err(refusal) = check_isolated_pty(&command) {
        panic!("{refusal}");
    }
    command
}

/// BUG-627: a `std::process::Command` that refuses to start unless it is
/// isolated ([`check_isolated`]). It has the builder methods daemon tests
/// use and no way to reach the inner command's `spawn` unchecked.
pub struct IsolatedCommand {
    command: Command,
}

impl IsolatedCommand {
    pub fn new(program: impl AsRef<OsStr>) -> Self {
        Self {
            command: Command::new(program),
        }
    }

    /// `fspec` (the test build) with `home`'s HOME and XDG_RUNTIME_DIR.
    pub fn fspec(home: &TestHome) -> Self {
        let mut command = Self::new(super::fspec_bin());
        command
            .env("HOME", home.home())
            // BUG-640: a test build resolves the user directory only from
            // FSPEC_USER_DIR.
            .env("FSPEC_USER_DIR", home.home().join(".fspec"))
            .env("XDG_RUNTIME_DIR", home.runtime_dir());
        command
    }

    pub fn arg(&mut self, arg: impl AsRef<OsStr>) -> &mut Self {
        self.command.arg(arg);
        self
    }

    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command.args(args);
        self
    }

    pub fn env(&mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> &mut Self {
        self.command.env(key, value);
        self
    }

    pub fn env_remove(&mut self, key: impl AsRef<OsStr>) -> &mut Self {
        self.command.env_remove(key);
        self
    }

    pub fn current_dir(&mut self, dir: impl AsRef<Path>) -> &mut Self {
        self.command.current_dir(dir);
        self
    }

    pub fn stdin(&mut self, cfg: impl Into<Stdio>) -> &mut Self {
        self.command.stdin(cfg);
        self
    }

    pub fn stdout(&mut self, cfg: impl Into<Stdio>) -> &mut Self {
        self.command.stdout(cfg);
        self
    }

    pub fn stderr(&mut self, cfg: impl Into<Stdio>) -> &mut Self {
        self.command.stderr(cfg);
        self
    }

    /// The arguments given so far.
    pub fn get_args(&self) -> Vec<OsString> {
        self.command.get_args().map(OsStr::to_os_string).collect()
    }

    /// [`check_isolated`] for this command.
    pub fn check(&self) -> Result<(), String> {
        check_isolated(&self.command)
    }

    fn refused(&self) -> std::io::Result<()> {
        self.check()
            .map_err(|refusal| std::io::Error::new(std::io::ErrorKind::PermissionDenied, refusal))
    }

    /// Start the process; a command that is not isolated is refused and
    /// nothing starts.
    pub fn spawn(&mut self) -> std::io::Result<Child> {
        self.refused()?;
        self.command.spawn()
    }

    /// Run the process to completion (refused like [`Self::spawn`]).
    pub fn output(&mut self) -> std::io::Result<Output> {
        self.refused()?;
        self.command.output()
    }

    /// Run the process to completion (refused like [`Self::spawn`]).
    pub fn status(&mut self) -> std::io::Result<ExitStatus> {
        self.refused()?;
        self.command.status()
    }
}
