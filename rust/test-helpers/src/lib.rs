//! Shared test-only helpers for the codelet workspace.
//!
//! This crate is the home for utilities that multiple test binaries need
//! to share — most importantly the architectural-invariant assertions
//! that pin the workspace's forbidden-arrow rules (RPC-002, RPC-006,
//! RPC-044, RPC-067).
//!
//! The crate's only workspace-internal dependency is the leaf crate
//! codelet-common, with its `test-support` feature (BUG-640: every test
//! build's user-directory resolver refuses the real `~/.fspec`): every test
//! binary in the workspace must be able to consume it without transitively
//! pulling forbidden crates like `codelet-napi`.
//!
//! # Module map
//!
//! - [`dependency_rules`] — generic dependency-rule helpers that walk
//!   `cargo metadata` and source trees to enforce arrows like
//!   `codelet-fspec → codelet-napi` are absent. Used by every
//!   `tests/no_napi_dependency.rs` regression test in the workspace.

pub mod coverage_steps;
pub mod or_fail;
pub use or_fail::{Failure, OrFail, OrFailErr, TestError, TestResult};
pub mod dependency_rules;
pub mod source_composition;
// BUG-642: range slices with a fixed bound (byte cuts of text).
pub mod fixed_length_cuts;
// TEST-042: the one allowed timed wait, and the check that tests use no other.
pub mod deadlock_guard;
pub mod wall_clock_waits;
pub use deadlock_guard::{
    deadlock_guard, deadlock_guard_blocking, pending_until, wait_until, wait_until_blocking,
    wait_until_here, yield_now, DEADLOCK_GUARD,
};

pub fn setup_temp_directory() -> std::io::Result<tempfile::TempDir> {
    tempfile::tempdir()
}

/// BUG-406: one temp directory per test process, alive for the whole
/// process. Point process-global state (e.g. `codelet_common`'s data
/// directory) here instead of at a per-test tempdir that is deleted while
/// other tests in the same binary still use it.
pub fn shared_temp_directory() -> Result<&'static std::path::Path, String> {
    static SHARED: std::sync::OnceLock<Result<tempfile::TempDir, String>> =
        std::sync::OnceLock::new();
    SHARED
        .get_or_init(|| {
            tempfile::tempdir()
                .map_err(|error| format!("cannot create the shared temp directory: {error}"))
        })
        .as_ref()
        .map(tempfile::TempDir::path)
        .map_err(Clone::clone)
}

/// BUG-406: a fresh, empty directory inside [`shared_temp_directory`] for a
/// test that needs its own data directory. It lives until the process
/// exits, so global state pointed at it never dangles after the test.
pub fn shared_temp_subdirectory() -> Result<std::path::PathBuf, String> {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let index = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let directory = shared_temp_directory()?.join(format!("test-{index}"));
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
    Ok(directory)
}

/// BUG-347: point `FSPEC_USER_DIR` at a per-process temp user directory
/// whose `fspec-config.json` disables RLCD, so tests never read the host's
/// `~/.fspec` or reach a live RLCD service. Idempotent; call it first in
/// every test that runs file tools. BUG-656: the directory is the `.fspec`
/// of [`isolated_home`]'s home, so both helpers name one user directory.
/// BUG-640: a test build cannot resolve the user directory at all until
/// this (or its own `FSPEC_USER_DIR`) is set.
pub fn isolated_user_directory() -> Result<&'static std::path::Path, String> {
    static USER: std::sync::OnceLock<Result<std::path::PathBuf, String>> =
        std::sync::OnceLock::new();
    let user = USER
        .get_or_init(|| {
            let user = shared_temp_directory()?.join("home").join(".fspec");
            std::fs::create_dir_all(&user)
                .map_err(|error| format!("cannot create {}: {error}", user.display()))?;
            std::fs::write(
                user.join("fspec-config.json"),
                "{\"rlcd\": {\"enabled\": false}}\n",
            )
            .map_err(|error| format!("cannot write the isolated fspec-config.json: {error}"))?;
            Ok(user)
        })
        .as_ref()
        .map_err(Clone::clone)?;
    std::env::set_var("FSPEC_USER_DIR", user);
    Ok(user.as_path())
}

/// BUG-656: point `HOME` at a per-process temp home and `FSPEC_USER_DIR` at
/// its `.fspec` ([`isolated_user_directory`]), which holds an
/// `fspec-config.json` that disables RLCD and a system `blocklist.json`
/// with no rules. A test that loads the system blocklist then reads only
/// its own rules, whether the path comes from `HOME` or from
/// `FSPEC_USER_DIR`, and the default template is never installed into the
/// real home. Idempotent; call it first in every test that reaches the
/// blocklist.
pub fn isolated_home() -> Result<&'static std::path::Path, String> {
    static HOME: std::sync::OnceLock<Result<std::path::PathBuf, String>> =
        std::sync::OnceLock::new();
    let home = HOME
        .get_or_init(|| {
            let user = isolated_user_directory()?;
            std::fs::write(
                user.join("blocklist.json"),
                "{\"version\": \"1.0.0\", \"rules\": []}\n",
            )
            .map_err(|error| format!("cannot write the isolated blocklist.json: {error}"))?;
            user.parent()
                .map(std::path::Path::to_path_buf)
                .ok_or_else(|| format!("{} has no parent", user.display()))
        })
        .as_ref()
        .map_err(Clone::clone)?;
    std::env::set_var("HOME", home);
    isolated_user_directory()?;
    Ok(home.as_path())
}
