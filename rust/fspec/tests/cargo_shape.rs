//! Cargo-shape / source-shape / CLI-surface / workspace integration tests for RPC-010.
//!
//! Feature: spec/features/fspec-binary-cargo-shape-rpc010.feature
//!
//! Consolidated from source_shape.rs + cli_surface.rs version/help/--pidfile-rejection +
//! workspace_and_reconnect.rs workspace scenarios so that
//! `fspec-binary-cargo-shape-rpc010.feature` maps 1:1 to a single test file
//! (fspec coverage validator design intent — 1 feature = 1 test file).
//!
//! These tests codify cargo / file-layout / source-content invariants
//! statically AND exercise the clap CLI surface dynamically. They MUST
//! FAIL in the testing phase because main.rs is a placeholder.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::fs;
use std::process::{Command, Stdio};

use codelet_fspec_tui::{FspecBackend, WebSocketFspecBackend};
use common::{
    codelet_root, fspec_bin, fspec_crate_root, make_workspace, spawn_fspec_daemon, strip_comments,
    ChildGuard,
};
use url::Url;

// === Helpers ===

fn extract_section(toml: &str, header: &str) -> Option<String> {
    let mut out = String::new();
    let mut in_section = false;
    for line in toml.lines() {
        let trimmed = line.trim();
        if trimmed == header {
            in_section = true;
            continue;
        }
        if in_section && trimmed.starts_with('[') && trimmed.ends_with(']') {
            break;
        }
        if in_section {
            out.push_str(line);
            out.push('\n');
        }
    }
    if in_section {
        Some(out)
    } else {
        None
    }
}

fn extract_members(workspace_section: &str) -> Option<Vec<String>> {
    let start = workspace_section.find("members")?;
    let after = &workspace_section[start..];
    let open = after.find('[')?;
    let close = after.find(']')?;
    let inside = &after[open + 1..close];
    // Strip `# ...` line comments before tokenising so commented entries
    // (e.g. "# RPC-038: ...") don't get fused with the next member token.
    let stripped: String = inside
        .lines()
        .map(|line| match line.find('#') {
            Some(i) => &line[..i],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut members = Vec::new();
    for tok in stripped.split(',') {
        let t = tok.trim().trim_matches('"').trim_matches('\'').trim();
        if !t.is_empty() {
            members.push(t.to_string());
        }
    }
    Some(members)
}

fn collect_rs_files(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack: Vec<std::path::PathBuf> = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                out.push(p);
            }
        }
    }
    out
}

// === Validating-phase attestation stubs ===
// These tests carry the @step comments required by the cargo-shape
// feature file for scenarios that are RUN attestations (cargo build,
// npm scripts, external test suites). The expensive external-invocation
// tests are marked #[ignore] so they only run when explicitly requested
// via `cargo test -- --ignored`. The static-JSON tests run by default.

#[path = "cargo_shape/allowed_files.rs"]
mod allowed_files;
#[path = "cargo_shape/codelet_fspec_is_registered.rs"]
mod codelet_fspec_is_registered;
#[path = "cargo_shape/fspec_version_prints_the.rs"]
mod fspec_version_prints_the;
#[path = "cargo_shape/locked_files.rs"]
mod locked_files;
use allowed_files::ALLOWED_FILES;
use locked_files::LOCKED_FILES;
#[path = "cargo_shape/cargo_build_p_fspec.rs"]
mod cargo_build_p_fspec;
#[path = "cargo_shape/cargo_toml_dependencies_does.rs"]
mod cargo_toml_dependencies_does;
#[path = "cargo_shape/fspec_src_contains_exactly.rs"]
mod fspec_src_contains_exactly;
