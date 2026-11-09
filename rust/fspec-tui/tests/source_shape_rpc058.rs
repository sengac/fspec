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

use codelet_test_helpers::source_composition::read_source;
use codelet_test_helpers::{OrFail, TestResult};
use std::path::PathBuf;

fn workspace_root() -> Result<PathBuf, codelet_test_helpers::TestError> {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    Ok(crate_dir
        .parent()
        .and_then(|p| p.parent())
        .or_fail("workspace root above rust/fspec-tui")?
        .to_path_buf())
}

fn normalise(source: &str) -> String {
    source
        .replace(['\n', '\r'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

include!("source_shape_rpc058/scheduled_job.rs");

/// Scenario: SessionManagerHandle declares the five new schedule methods
#[test]
fn session_manager_handle_declares_schedule_methods() -> TestResult {
    // @step Given the file rust/core/src/session_manager_handle.rs is compiled
    let path = workspace_root()?.join("rust/core/src/session_manager_handle.rs");
    let source = read_tree(&path)?;
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

    // @step And it declares a trait method named "schedule_list" returning Result<Vec<ScheduledJob>, String>
    assert!(
        source.contains("fn schedule_list("),
        "session_manager_handle.rs should declare fn schedule_list"
    );
    assert!(
        normalised.contains("-> Result<Vec<ScheduledJob>, String>"),
        "schedule_list should return Result<Vec<ScheduledJob>, String>"
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
    Ok(())
}

/// Scenario: StubSessionManagerHandle exposes per-call counters for all five schedule methods
#[test]
fn stub_exposes_per_call_counters_for_schedule() -> TestResult {
    // @step Given the file rust/core/src/session_manager_handle.rs is compiled
    let path = workspace_root()?.join("rust/core/src/session_manager_handle.rs");
    let source = read_tree(&path)?;
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
    Ok(())
}

/// Scenario: FspecService declares the five new RPC methods
#[test]
fn fspec_service_declares_schedule_methods() -> TestResult {
    // @step Given the file rust/rpc/src/lib.rs is compiled
    let path = workspace_root()?.join("rust/rpc/src/lib.rs");
    let source = read_source(&path).or_fail("resolve rpc/src/lib.rs")?;
    let normalised = normalise(&source);

    // @step Then it declares an async fn named "schedule_add" with return type Result<ScheduledJob, String>
    // @step And it declares an async fn named "schedule_list" with return type Result<Vec<ScheduledJob>, String>
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
        normalised.contains("-> Result<Vec<ScheduledJob>, String>"),
        "schedule_list should return Result<Vec<ScheduledJob>, String>"
    );
    Ok(())
}

/// Scenario: FspecBackend declares the five new methods
#[test]
fn fspec_backend_declares_schedule_methods() -> TestResult {
    // @step Given the file rust/fspec-tui/src/transport/mod.rs is compiled
    let path = workspace_root()?.join("rust/fspec-tui/src/transport/mod.rs");
    let source = read_source(&path).or_fail("resolve transport/mod.rs")?;
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
    Ok(())
}

/// Scenario: Both transports implement the five new methods
#[test]
fn both_transports_implement_schedule_methods() -> TestResult {
    // @step Given the files rust/fspec-tui/src/transport/embedded.rs and rust/fspec-tui/src/transport/websocket.rs are compiled
    let embedded = read_source(&workspace_root()?.join("rust/fspec-tui/src/transport/embedded.rs"))
        .or_fail("resolve transport/embedded.rs")?;
    let websocket =
        read_source(&workspace_root()?.join("rust/fspec-tui/src/transport/websocket.rs"))
            .or_fail("resolve transport/websocket.rs")?;

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
    Ok(())
}

/// Scenario: schedule_parser module exists with the documented entry points
#[test]
fn schedule_parser_module_exists() -> TestResult {
    // @step Given the file rust/fspec-tui/src/app/schedule_parser.rs exists
    let path = workspace_root()?.join("rust/fspec-tui/src/app/schedule_parser.rs");
    let source = codelet_test_helpers::source_composition::read_tree_or_file(&path)
        .or_fail("read app/schedule_parser.rs")?;

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
    Ok(())
}

/// REFAC-021/022: read a source file together with its include!d fragments
/// Scenario: /schedule slash command wiring lives in dispatch_slash_schedule.rs
#[test]
fn schedule_slash_command_wiring_lives_in_dispatch_slash_schedule() -> TestResult {
    // @step Given the file rust/fspec-tui/src/app/dispatch_slash_schedule.rs exists
    let source =
        read_tree(&workspace_root()?.join("rust/fspec-tui/src/app/dispatch_slash_schedule.rs"))?;
    // @step Then it declares a method named "handle_slash_schedule_help"
    assert!(source.contains("fn handle_slash_schedule_help("));
    // @step And it declares a method named "handle_schedule_subcommand"
    assert!(source.contains("fn handle_schedule_subcommand("));
    // @step And it declares a method named "handle_schedule_add"
    assert!(source.contains("fn handle_schedule_add("));
    // @step And it declares a method named "handle_schedule_list"
    assert!(source.contains("fn handle_schedule_list("));
    // @step And it declares a method named "handle_schedule_pause"
    assert!(source.contains("fn handle_schedule_pause("));
    // @step And it declares a method named "handle_schedule_resume"
    assert!(source.contains("fn handle_schedule_resume("));
    // @step And it declares a method named "handle_schedule_remove"
    assert!(source.contains("fn handle_schedule_remove("));
    // @step And it declares a method named "try_dispatch_slash_schedule"
    assert!(source.contains("fn try_dispatch_slash_schedule("));
    Ok(())
}

/// and file-backed child modules (split modules read as one text).
fn read_tree(path: &std::path::Path) -> Result<String, codelet_test_helpers::TestError> {
    Ok(
        codelet_test_helpers::source_composition::read_module_tree(path)
            .map_err(|error| format!("read {}: {error}", path.display()))?,
    )
}
