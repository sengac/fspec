//! BUG-550: the one FSPEC_HOME redirect the providers lib tests share.
//!
//! The process environment is global to the lib test binary, so every test
//! that changes it runs under serial_test's default `#[serial]` key, the
//! one shared guard (rust/providers/tests/bug550_lib_tests_share_one_env_guard.rs
//! enforces it). A module-local lock would not exclude the other modules.
//!
//! Feature: spec/features/providers-lib-tests-share-one-environment-guard.feature

use std::ffi::OsString;
use std::path::Path;

/// Points `FSPEC_HOME` at a fresh temporary directory and restores the
/// previous value when dropped. Use it only from a `#[serial]` test.
pub struct FspecHome {
    directory: tempfile::TempDir,
    previous: Option<OsString>,
}

impl FspecHome {
    pub fn redirect() -> std::io::Result<Self> {
        let directory = tempfile::tempdir()?;
        let previous = std::env::var_os("FSPEC_HOME");
        std::env::set_var("FSPEC_HOME", directory.path());
        Ok(Self {
            directory,
            previous,
        })
    }

    pub fn path(&self) -> &Path {
        self.directory.path()
    }
}

impl Drop for FspecHome {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(previous) => std::env::set_var("FSPEC_HOME", previous),
            None => std::env::remove_var("FSPEC_HOME"),
        }
    }
}

#[cfg(test)]
#[path = "env_tests.rs"]
mod tests;
