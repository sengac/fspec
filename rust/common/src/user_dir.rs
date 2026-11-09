//! BUG-640: the one resolver of the fspec user directory.
//!
//! Feature: spec/features/fspec-config-writes-are-atomic-and-tests-never-touch-the-real-user-config.feature
//!
//! The user directory holds `fspec-config.json` and the other per-user
//! files. It is `FSPEC_USER_DIR` when that is set and not empty, else
//! `<home>/.fspec`. A build with this crate's `test-support` feature (every
//! test build: codelet-test-helpers turns it on) refuses to fall back to
//! `<home>/.fspec`, so no test can read or write the user's real config
//! (product owner, 2026-11-08).

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The environment variable that names the fspec user directory.
pub const FSPEC_USER_DIR: &str = "FSPEC_USER_DIR";

/// The error of a test-support build that resolves the user directory
/// before isolating it.
pub const NOT_ISOLATED: &str = "FSPEC_USER_DIR is not set in a test-support build; isolate the \
     user directory with codelet_test_helpers::isolated_user_directory() first";

/// The fspec user directory: `FSPEC_USER_DIR` when set and not empty, else
/// `<home>/.fspec`. A test-support build fails with [`NOT_ISOLATED`]
/// instead of using the home directory; no home directory is an error.
pub fn fspec_user_dir() -> Result<PathBuf, String> {
    match std::env::var_os(FSPEC_USER_DIR).filter(|value| !value.is_empty()) {
        Some(directory) => Ok(PathBuf::from(directory)),
        None => home_user_dir(),
    }
}

/// The same rule over injected values, for a caller that resolves from an
/// injected environment rather than the process's (fspec init's `HomeEnv`,
/// BUG-533): `fspec_user_dir_var` when set and not empty, else
/// `<home>/.fspec`. It never reads the process environment, so the
/// test-support guard does not apply.
pub fn user_dir_from(
    fspec_user_dir_var: Option<OsString>,
    home: Option<&Path>,
) -> Result<PathBuf, String> {
    match fspec_user_dir_var.filter(|value| !value.is_empty()) {
        Some(directory) => Ok(PathBuf::from(directory)),
        None => home
            .map(|home| home.join(".fspec"))
            .ok_or_else(|| NO_HOME.to_string()),
    }
}

const NO_HOME: &str = "cannot resolve the fspec user directory: no home directory";

#[cfg(feature = "test-support")]
fn home_user_dir() -> Result<PathBuf, String> {
    Err(NOT_ISOLATED.to_string())
}

#[cfg(not(feature = "test-support"))]
fn home_user_dir() -> Result<PathBuf, String> {
    user_dir_from(None, dirs::home_dir().as_deref())
}
