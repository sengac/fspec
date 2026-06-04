#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/agent-loop-error-classification-recovery-wiring-shape.feature
//!
//! RPC-087 regression-shape coverage for the error classification +
//! recovery helper wiring in `codelet/cli/src/interactive/`. Pins the
//! module declarations in `mod.rs`, the public re-exports of the
//! classifier + recovery surface, and the call sites in `stream_loop.rs`
//! that compose them.
//!
//! These tests intentionally use source-string substring assertions
//! (sub-millisecond, no HTTP, no stub provider) — see
//! `spec/attachments/RPC-087/ast-research-error-classification-recovery.md`
//! for the rationale.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

fn workspace_root() -> PathBuf {
    // tests run with CWD = codelet/cli — walk up two levels to repo root.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root above codelet/cli")
        .to_path_buf()
}

fn read_source(rel: &str) -> String {
    let path = workspace_root().join(rel);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
}

// ====================================================================
// Scenario: interactive/mod.rs declares all six recovery modules and the
// error_classifiers module
// ====================================================================
#[test]
fn interactive_mod_declares_classifier_and_six_recovery_modules() {
    // @step Given the source file codelet/cli/src/interactive/mod.rs
    let body = read_source("codelet/cli/src/interactive/mod.rs");

    // @step When I read the file as a string
    // (already read into `body` above)

    // @step Then the body contains the substring "mod error_classifiers;"
    assert!(body.contains("mod error_classifiers;"));

    // @step And the body contains the substring "mod recovery_compaction;"
    assert!(body.contains("mod recovery_compaction;"));

    // @step And the body contains the substring "mod recovery_image;"
    assert!(body.contains("mod recovery_image;"));

    // @step And the body contains the substring "mod recovery_network;"
    assert!(body.contains("mod recovery_network;"));

    // @step And the body contains the substring "mod recovery_stall;"
    assert!(body.contains("mod recovery_stall;"));

    // @step And the body contains the substring "mod recovery_thinking;"
    assert!(body.contains("mod recovery_thinking;"));

    // @step And the body contains the substring "mod recovery_truncation;"
    assert!(body.contains("mod recovery_truncation;"));
}

// ====================================================================
// Scenario: interactive crate re-exports the recovery + classifier
// public surface
// ====================================================================
#[test]
fn interactive_mod_reexports_recovery_and_classifier_surface() {
    // @step Given the source file codelet/cli/src/interactive/mod.rs
    let body = read_source("codelet/cli/src/interactive/mod.rs");

    // @step When I read the file as a string
    // (already read above)

    // @step Then the body contains the substring "pub use error_classifiers::{"
    assert!(body.contains("pub use error_classifiers::{"));

    // @step And the body contains the substring "is_transient_network_error"
    assert!(body.contains("is_transient_network_error"));

    // @step And the body contains the substring "is_stall_timeout_error"
    assert!(body.contains("is_stall_timeout_error"));

    // @step And the body contains the substring "classify_compaction_branch"
    assert!(body.contains("classify_compaction_branch"));

    // @step And the body contains the substring "pub use recovery_network::{"
    assert!(body.contains("pub use recovery_network::{"));

    // @step And the body contains the substring "MAX_NETWORK_RETRIES"
    assert!(body.contains("MAX_NETWORK_RETRIES"));

    // @step And the body contains the substring "network_retry_delay"
    assert!(body.contains("network_retry_delay"));

    // @step And the body contains the substring "pub use recovery_image::sanitize_image_content;"
    assert!(body.contains("pub use recovery_image::sanitize_image_content;"));

    // @step And the body contains the substring "STALL_TIMEOUT_ERROR_PREFIX"
    assert!(body.contains("STALL_TIMEOUT_ERROR_PREFIX"));

    // @step And the body contains the substring "MAX_TRUNCATION_RETRIES"
    assert!(body.contains("MAX_TRUNCATION_RETRIES"));
}

// ====================================================================
// Scenario: MAX_NETWORK_RETRIES constant is reachable via the public
// surface and equals 3
// ====================================================================
#[test]
fn max_network_retries_is_reachable_and_equals_three() {
    // @step Given the re-exported constant codelet_cli::interactive::MAX_NETWORK_RETRIES
    let value = codelet_cli::interactive::MAX_NETWORK_RETRIES;

    // @step When I read its value
    // (binding above)

    // @step Then it equals 3
    assert_eq!(value, 3);
}

// ====================================================================
// Scenario: network_retry_delay implements exponential backoff with 1s base
// ====================================================================
#[test]
fn network_retry_delay_implements_exponential_backoff() {
    use codelet_cli::interactive::network_retry_delay;

    // @step Given the re-exported function codelet_cli::interactive::network_retry_delay
    // (imported above)

    // @step When I call it with attempt 1
    // @step Then it returns Duration::from_millis(1000)
    assert_eq!(network_retry_delay(1), Duration::from_millis(1000));

    // @step When I call it with attempt 2
    // @step Then it returns Duration::from_millis(2000)
    assert_eq!(network_retry_delay(2), Duration::from_millis(2000));

    // @step When I call it with attempt 3
    // @step Then it returns Duration::from_millis(4000)
    assert_eq!(network_retry_delay(3), Duration::from_millis(4000));

    // @step When I call it with attempt 0
    // @step Then it returns Duration::from_millis(1000)
    assert_eq!(network_retry_delay(0), Duration::from_millis(1000));
}

// ====================================================================
// Scenario: is_transient_network_error recognises common HTTP/connection
// failures while stall classifier does not
// ====================================================================
#[test]
fn transient_network_classifier_matches_common_failures_stall_does_not() {
    use codelet_cli::interactive::{is_stall_timeout_error, is_transient_network_error};

    // @step Given the re-exported predicate codelet_cli::interactive::is_transient_network_error
    // (imported above)

    // @step When I call it with "error sending request for url"
    // @step Then it returns true
    assert!(is_transient_network_error("error sending request for url"));

    // @step When I call it with "connection reset by peer"
    // @step Then it returns true
    assert!(is_transient_network_error("connection reset by peer"));

    // @step When I call it with "connection refused"
    // @step Then it returns true
    assert!(is_transient_network_error("connection refused"));

    // @step When I call it with "operation timed out"
    // @step Then it returns true
    assert!(is_transient_network_error("operation timed out"));

    // @step And calling codelet_cli::interactive::is_stall_timeout_error with "connection reset by peer" returns false
    assert!(!is_stall_timeout_error("connection reset by peer"));
}

// ====================================================================
// Scenario: is_stall_timeout_error uses STALL_TIMEOUT_ERROR_PREFIX as
// single source of truth
// ====================================================================
#[test]
fn stall_classifier_uses_prefix_as_single_source_of_truth() {
    use codelet_cli::interactive::{is_stall_timeout_error, STALL_TIMEOUT_ERROR_PREFIX};

    // @step Given the re-exported predicate codelet_cli::interactive::is_stall_timeout_error
    // @step And the re-exported constant codelet_cli::interactive::STALL_TIMEOUT_ERROR_PREFIX
    // (both imported above)

    // @step When I call the predicate with the constant value
    // @step Then it returns true
    assert!(is_stall_timeout_error(STALL_TIMEOUT_ERROR_PREFIX));

    // @step And the source file codelet/cli/src/interactive/error_classifiers.rs body contains the substring "super::recovery_stall::STALL_TIMEOUT_ERROR_PREFIX"
    let body = read_source("codelet/cli/src/interactive/error_classifiers.rs");
    assert!(
        body.contains("super::recovery_stall::STALL_TIMEOUT_ERROR_PREFIX"),
        "error_classifiers.rs must reference the canonical prefix \
         via super::recovery_stall, not a duplicated literal"
    );
}

// ====================================================================
// Scenario: stream_loop.rs wires every classifier predicate and has at
// least one call site each
// ====================================================================
#[test]
fn stream_loop_wires_every_classifier_predicate() {
    // @step Given the source file codelet/cli/src/interactive/stream_loop.rs
    let body = read_source("codelet/cli/src/interactive/stream_loop.rs");

    // @step When I read the file as a string
    // (already read above)

    // @step Then the body contains the substring "use super::error_classifiers::{"
    assert!(body.contains("use super::error_classifiers::{"));

    // @step And the body contains the substring "is_stall_timeout_error("
    assert!(body.contains("is_stall_timeout_error("));

    // @step And the body contains the substring "is_prompt_too_long_error("
    assert!(body.contains("is_prompt_too_long_error("));

    // @step And the body contains the substring "is_image_content_error("
    assert!(body.contains("is_image_content_error("));

    // @step And the body contains the substring "is_truncated_tool_call_error("
    assert!(body.contains("is_truncated_tool_call_error("));

    // @step And the body contains the substring "is_transient_network_error("
    assert!(body.contains("is_transient_network_error("));

    // @step And the body contains the substring "classify_compaction_branch("
    assert!(body.contains("classify_compaction_branch("));
}

// ====================================================================
// Scenario: stream_loop.rs guards retry with MAX_NETWORK_RETRIES and
// uses network_retry_delay
// ====================================================================
#[test]
fn stream_loop_guards_retry_with_max_network_retries() {
    // @step Given the source file codelet/cli/src/interactive/stream_loop.rs
    let body = read_source("codelet/cli/src/interactive/stream_loop.rs");

    // @step When I read the file as a string
    // (already read above)

    // @step Then the body contains the substring "use super::recovery_network::{MAX_NETWORK_RETRIES, network_retry_delay}"
    assert!(body.contains("use super::recovery_network::{MAX_NETWORK_RETRIES, network_retry_delay}"));

    // @step And the body contains the substring "network_retry_count <= MAX_NETWORK_RETRIES"
    assert!(body.contains("network_retry_count <= MAX_NETWORK_RETRIES"));

    // @step And the body contains the substring "network_retry_delay(network_retry_count)"
    assert!(body.contains("network_retry_delay(network_retry_count)"));
}

// ====================================================================
// Scenario: stream_loop.rs sanitises image content after classifying an
// image-content rejection
// ====================================================================
#[test]
fn stream_loop_sanitises_image_content_after_classifier_check() {
    // @step Given the source file codelet/cli/src/interactive/stream_loop.rs
    let body = read_source("codelet/cli/src/interactive/stream_loop.rs");

    // @step When I read the file as a string
    // (already read above)

    // @step Then the body contains the substring "sanitize_image_content(&mut session.messages)"
    assert!(body.contains("sanitize_image_content(&mut session.messages)"));

    // @step And the body contains the substring "is_image_content_error(&error_str)"
    assert!(body.contains("is_image_content_error(&error_str)"));

    // @step And the byte offset of "is_image_content_error(&error_str)" is less than the byte offset of "sanitize_image_content(&mut session.messages)"
    let classifier_offset = body
        .find("is_image_content_error(&error_str)")
        .expect("classifier call must exist");
    let sanitize_offset = body
        .find("sanitize_image_content(&mut session.messages)")
        .expect("sanitize call must exist");
    assert!(
        classifier_offset < sanitize_offset,
        "is_image_content_error(&error_str) must precede \
         sanitize_image_content(&mut session.messages) in source order \
         (classify first, sanitize second)"
    );
}
