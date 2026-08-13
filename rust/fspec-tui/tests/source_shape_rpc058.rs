//! RPC-058 — Source-shape assertions for the /schedule RPC surface.
//!
//! Feature: spec/features/rpc058-schedule-source-shape.feature
//!
//! These tests scan source files at compile time to pin the layering
//! contract for the FIVE new RPC methods (schedule_add, schedule_list,
//! schedule_pause, schedule_resume, schedule_remove), the NEW wire
//! type (ScheduledJob), the new ScheduleSubcommand parser, and the
//! `/schedule` slash-command dispatch routing in
//! `dispatch_slash_schedule.rs`. Mirrors the source_shape_rpc054 /
//! source_shape_rpc055 / source_shape_rpc056 / source_shape_rpc057
//! patterns.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root above rust/fspec-tui")
        .to_path_buf()
}

fn normalise(source: &str) -> String {
    source
        .replace(['\n', '\r'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Scenario: ScheduledJob wire type is exported from codelet-rpc-types
#[test]
fn rpc_types_exports_scheduled_job_wire_type() {
    // @step Given the file rust/rpc-types/src/lib.rs is compiled
    let path = workspace_root().join("rust/rpc-types/src/lib.rs");
    let source = fs::read_to_string(&path).expect("read rpc-types/src/lib.rs");
    let normalised = normalise(&source);

    // @step Then it declares a public struct named "ScheduledJob"
    assert!(
        source.contains("pub struct ScheduledJob"),
        "rpc-types/src/lib.rs should declare pub struct ScheduledJob"
    );

    // @step And ScheduledJob has fields named name, cron, timezone, job_type, status
    // @step And ScheduledJob has fields named created_at, last_run_at, last_run_status
    // @step And ScheduledJob has fields named role, prompt, command, overlap_policy
    for field in [
        "pub name:",
        "pub cron:",
        "pub timezone:",
        "pub job_type:",
        "pub status:",
        "pub created_at:",
        "pub last_run_at:",
        "pub last_run_status:",
        "pub role:",
        "pub prompt:",
        "pub command:",
        "pub overlap_policy:",
    ] {
        assert!(
            normalised.contains(field),
            "ScheduledJob should declare field {field:?}"
        );
    }
}

/// Scenario: SessionManagerHandle declares the five new schedule methods
#[test]
fn session_manager_handle_declares_schedule_methods() {
    // @step Given the file rust/core/src/session_manager_handle.rs is compiled
    let path = workspace_root().join("rust/core/src/session_manager_handle.rs");
    let source = fs::read_to_string(&path).expect("read session_manager_handle.rs");
    let normalised = normalise(&source);

    // @step Then it declares a trait method named "schedule_add" returning Result<ScheduledJob, String>
    assert!(
        source.contains("fn schedule_add("),
        "session_manager_handle.rs should declare fn schedule_add"
    );
    assert!(
        normalised.contains("-> Result<ScheduledJob, String>"),
        "schedule_add/pause/resume should return Result<ScheduledJob, String>"
    );

    // @step And it declares a trait method named "schedule_list" returning Vec<ScheduledJob>
    assert!(
        source.contains("fn schedule_list("),
        "session_manager_handle.rs should declare fn schedule_list"
    );
    assert!(
        normalised.contains("-> Vec<ScheduledJob>"),
        "schedule_list should return Vec<ScheduledJob>"
    );

    // @step And it declares a trait method named "schedule_pause" returning Result<ScheduledJob, String>
    assert!(
        source.contains("fn schedule_pause("),
        "session_manager_handle.rs should declare fn schedule_pause"
    );

    // @step And it declares a trait method named "schedule_resume" returning Result<ScheduledJob, String>
    assert!(
        source.contains("fn schedule_resume("),
        "session_manager_handle.rs should declare fn schedule_resume"
    );

    // @step And it declares a trait method named "schedule_remove" returning Result<(), String>
    assert!(
        source.contains("fn schedule_remove("),
        "session_manager_handle.rs should declare fn schedule_remove"
    );
}

/// Scenario: StubSessionManagerHandle exposes per-call counters for all five schedule methods
#[test]
fn stub_exposes_per_call_counters_for_schedule() {
    // @step Given the file rust/core/src/session_manager_handle.rs is compiled
    let path = workspace_root().join("rust/core/src/session_manager_handle.rs");
    let source = fs::read_to_string(&path).expect("read session_manager_handle.rs");
    let normalised = normalise(&source);

    // @step Then StubSessionManagerHandle declares a method named "schedule_add_calls" returning u64
    // @step And StubSessionManagerHandle declares a method named "schedule_list_calls" returning u64
    // @step And StubSessionManagerHandle declares a method named "schedule_pause_calls" returning u64
    // @step And StubSessionManagerHandle declares a method named "schedule_resume_calls" returning u64
    // @step And StubSessionManagerHandle declares a method named "schedule_remove_calls" returning u64
    for counter in [
        "schedule_add_calls",
        "schedule_list_calls",
        "schedule_pause_calls",
        "schedule_resume_calls",
        "schedule_remove_calls",
    ] {
        let needle = format!("pub fn {counter}(");
        assert!(
            source.contains(&needle),
            "StubSessionManagerHandle should declare pub fn {counter}"
        );
        let sig = format!("pub fn {counter}(&self) -> u64");
        assert!(
            normalised.contains(&sig),
            "StubSessionManagerHandle should declare {counter}(&self) -> u64"
        );
    }
}

/// Scenario: FspecService declares the five new RPC methods
#[test]
fn fspec_service_declares_schedule_methods() {
    // @step Given the file rust/rpc/src/lib.rs is compiled
    let path = workspace_root().join("rust/rpc/src/lib.rs");
    let source = fs::read_to_string(&path).expect("read rpc/src/lib.rs");
    let normalised = normalise(&source);

    // @step Then it declares an async fn named "schedule_add" with return type Result<ScheduledJob, String>
    // @step And it declares an async fn named "schedule_list" with return type Vec<ScheduledJob>
    // @step And it declares an async fn named "schedule_pause" with return type Result<ScheduledJob, String>
    // @step And it declares an async fn named "schedule_resume" with return type Result<ScheduledJob, String>
    // @step And it declares an async fn named "schedule_remove" with return type Result<(), String>
    for method in [
        "schedule_add",
        "schedule_list",
        "schedule_pause",
        "schedule_resume",
        "schedule_remove",
    ] {
        let needle = format!("async fn {method}(");
        assert!(
            source.contains(&needle),
            "rpc/src/lib.rs should declare async fn {method}"
        );
    }

    // Spot-check documented return shapes appear in the service surface.
    assert!(
        normalised.contains("-> Result<ScheduledJob, String>"),
        "schedule_add/pause/resume should return Result<ScheduledJob, String>"
    );
    assert!(
        normalised.contains("-> Vec<ScheduledJob>"),
        "schedule_list should return Vec<ScheduledJob>"
    );
}

/// Scenario: FspecBackend declares the five new methods
#[test]
fn fspec_backend_declares_schedule_methods() {
    // @step Given the file rust/fspec-tui/src/transport/mod.rs is compiled
    let path = workspace_root().join("rust/fspec-tui/src/transport/mod.rs");
    let source = fs::read_to_string(&path).expect("read transport/mod.rs");
    let normalised = normalise(&source);

    // @step Then it declares an async fn named "schedule_add" on the FspecBackend trait returning Result<ScheduledJob>
    // @step And it declares an async fn named "schedule_list" on the FspecBackend trait returning Result<Vec<ScheduledJob>>
    // @step And it declares an async fn named "schedule_pause" on the FspecBackend trait returning Result<ScheduledJob>
    // @step And it declares an async fn named "schedule_resume" on the FspecBackend trait returning Result<ScheduledJob>
    // @step And it declares an async fn named "schedule_remove" on the FspecBackend trait returning Result<()>
    for method in [
        "schedule_add",
        "schedule_list",
        "schedule_pause",
        "schedule_resume",
        "schedule_remove",
    ] {
        let needle = format!("async fn {method}(");
        assert!(
            source.contains(&needle),
            "transport/mod.rs should declare async fn {method} on FspecBackend"
        );
    }

    // Trait surface uses anyhow-style Result<...>.
    assert!(
        normalised.contains("-> Result<ScheduledJob>"),
        "FspecBackend::schedule_add/pause/resume should return Result<ScheduledJob>"
    );
    assert!(
        normalised.contains("-> Result<Vec<ScheduledJob>>"),
        "FspecBackend::schedule_list should return Result<Vec<ScheduledJob>>"
    );
}

/// Scenario: Both transports implement the five new methods
#[test]
fn both_transports_implement_schedule_methods() {
    // @step Given the files rust/fspec-tui/src/transport/embedded.rs and rust/fspec-tui/src/transport/websocket.rs are compiled
    let embedded =
        fs::read_to_string(workspace_root().join("rust/fspec-tui/src/transport/embedded.rs"))
            .expect("read transport/embedded.rs");
    let websocket =
        fs::read_to_string(workspace_root().join("rust/fspec-tui/src/transport/websocket.rs"))
            .expect("read transport/websocket.rs");

    // @step Then each file contains an impl of "schedule_add" that calls the corresponding tarpc client method
    // @step And each file contains an impl of "schedule_list" that calls the corresponding tarpc client method
    // @step And each file contains an impl of "schedule_pause" that calls the corresponding tarpc client method
    // @step And each file contains an impl of "schedule_resume" that calls the corresponding tarpc client method
    // @step And each file contains an impl of "schedule_remove" that calls the corresponding tarpc client method
    for method in [
        "schedule_add",
        "schedule_list",
        "schedule_pause",
        "schedule_resume",
        "schedule_remove",
    ] {
        let impl_needle = format!("async fn {method}(");
        let forward_needle = format!(".{method}(");
        assert!(
            embedded.contains(&impl_needle),
            "embedded.rs should impl {method}"
        );
        assert!(
            embedded.contains(&forward_needle),
            "embedded.rs should forward to the tarpc client's {method}"
        );
        assert!(
            websocket.contains(&impl_needle),
            "websocket.rs should impl {method}"
        );
        assert!(
            websocket.contains(&forward_needle),
            "websocket.rs should forward to the tarpc client's {method}"
        );
    }
}

/// Scenario: schedule_parser module exists with the documented entry points
#[test]
fn schedule_parser_module_exists() {
    // @step Given the file rust/fspec-tui/src/app/schedule_parser.rs exists
    let path = workspace_root().join("rust/fspec-tui/src/app/schedule_parser.rs");
    let source = fs::read_to_string(&path).expect("read app/schedule_parser.rs");

    // @step Then it declares a public enum named "ScheduleSubcommand"
    assert!(
        source.contains("pub enum ScheduleSubcommand"),
        "schedule_parser.rs should declare pub enum ScheduleSubcommand"
    );

    // @step And ScheduleSubcommand has variants named Add, List, Pause, Resume, Remove, Help
    for variant in ["Add", "List", "Pause", "Resume", "Remove", "Help"] {
        // Variants may be bare (`List,`) or struct-form (`Add { ... }`).
        assert!(
            source.contains(&format!("{variant},"))
                || source.contains(&format!("{variant} {{"))
                || source.contains(&format!("{variant}(")),
            "ScheduleSubcommand should declare variant {variant}"
        );
    }

    // @step And it declares a public fn named "parse_schedule_command" taking &str and returning ScheduleSubcommand
    assert!(
        source.contains("pub fn parse_schedule_command("),
        "schedule_parser.rs should declare pub fn parse_schedule_command"
    );
}
