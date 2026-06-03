//! Feature: spec/features/handle-impl-redundant-clone-regression.feature
//!
//! RPC-077: pin the absence of the `clippy::redundant_clone` violations
//! that originally appeared at `codelet/sessions/src/handle_impl.rs:1321`
//! (`id: id.clone(),`) and `:1323` (`prompt: prompt.clone(),`). Both
//! patterns were eliminated implicitly during the workspace clippy
//! sweep (RPC-082/083/084/086 cleanup). This test fails in milliseconds
//! if either pattern ever returns — without paying the 30-second
//! clippy compile cost on every CI run.
//!
//! The runtime-equivalent contract (`cargo clippy -p codelet-sessions
//! --all-targets -- -D warnings` exit code 0) is already pinned by
//! `tests/skeleton_invariants.rs::scenario_workspace_lints_are_inherited_and_clippy_passes`.
//! This file is the fast structural complement.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;

/// Workspace root resolved from `CARGO_MANIFEST_DIR` (= `codelet/sessions`).
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR must have a parent")
        .to_path_buf()
}

fn read_source(rel_from_workspace: &str) -> String {
    let path = workspace_root().join(rel_from_workspace);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
}

// ===========================================================================
// Scenario: handle_impl.rs has no redundant `id: id.clone(),` struct literal
// ===========================================================================

#[test]
fn handle_impl_has_no_redundant_id_clone() {
    // @step Given the source of `codelet/sessions/src/handle_impl.rs`
    let src = read_source("sessions/src/handle_impl.rs");

    // @step When I scan the source for the substring `id: id.clone(),`
    let occurrences = src.matches("id: id.clone(),").count();

    // @step Then zero matches are found
    assert_eq!(
        occurrences, 0,
        "codelet/sessions/src/handle_impl.rs must NOT contain \
         `id: id.clone(),` — this pattern triggers \
         `clippy::redundant_clone` because the owned `id` binding is \
         dropped at the end of the surrounding block. Original RPC-077 \
         bug fix landed via the workspace clippy sweep (RPC-082/083/084/086)."
    );
}

// ===========================================================================
// Scenario: handle_impl.rs has no redundant `prompt: prompt.clone(),` struct literal
// ===========================================================================

#[test]
fn handle_impl_has_no_redundant_prompt_clone() {
    // @step Given the source of `codelet/sessions/src/handle_impl.rs`
    let src = read_source("sessions/src/handle_impl.rs");

    // @step When I scan the source for the substring `prompt: prompt.clone(),`
    let occurrences = src.matches("prompt: prompt.clone(),").count();

    // @step Then zero matches are found
    assert_eq!(
        occurrences, 0,
        "codelet/sessions/src/handle_impl.rs must NOT contain \
         `prompt: prompt.clone(),` — this pattern triggers \
         `clippy::redundant_clone` because the owned `prompt` binding is \
         dropped at the end of the surrounding block. Original RPC-077 \
         bug fix landed via the workspace clippy sweep (RPC-082/083/084/086)."
    );
}

// ===========================================================================
// Scenario: codelet/sessions/Cargo.toml inherits workspace lints
// ===========================================================================

#[test]
fn sessions_cargo_toml_inherits_workspace_lints() {
    // @step Given the source of `codelet/sessions/Cargo.toml`
    let src = read_source("sessions/Cargo.toml");

    // @step When I inspect the `[lints]` section
    assert!(
        src.contains("[lints]"),
        "codelet/sessions/Cargo.toml must declare a `[lints]` section so \
         the workspace lint policy (deny redundant_clone et al.) applies \
         when running `cargo clippy -p codelet-sessions`"
    );

    // @step Then the section declares `workspace = true`
    //
    // Walk the `[lints]` section body and assert the next non-comment,
    // non-empty line contains `workspace = true`. This is the canonical
    // shape of an inheriting `[lints]` table in Cargo.toml.
    let lints_pos = src
        .find("[lints]")
        .expect("`[lints]` exists (checked above)");
    let body = &src[lints_pos + "[lints]".len()..];
    let has_workspace_true = body
        .lines()
        .take_while(|l| !l.trim_start().starts_with('['))
        .any(|l| {
            let t = l.trim();
            t.starts_with("workspace") && t.contains("= true")
        });
    assert!(
        has_workspace_true,
        "codelet/sessions/Cargo.toml `[lints]` section must inherit \
         workspace lints via `workspace = true`"
    );
}
