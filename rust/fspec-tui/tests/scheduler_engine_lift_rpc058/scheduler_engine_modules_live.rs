//! Feature: spec/features/rpc058-scheduler-engine-lift.feature
use super::*;
use codelet_test_helpers::{OrFail, TestResult};

/// Scenario: Scheduler engine modules live under rust/core/src/scheduler/
#[test]
fn scheduler_engine_modules_live_under_codelet_core() -> TestResult {
    // @step Given the directory rust/core/src/scheduler/ exists
    let dir = scheduler_core_dir()?;
    assert!(
        dir.is_dir(),
        "rust/core/src/scheduler/ directory should exist"
    );

    // @step Then it contains a file named "mod.rs"
    // @step And it contains a file named "engine.rs"
    // @step And it contains a file named "state.rs"
    // @step And it contains a file named "cron_utils.rs"
    // @step And it contains a file named "types.rs"
    // @step And it contains a file named "trigger.rs"
    // @step And it contains a file named "agent_job.rs"
    // @step And it contains a file named "shell_job.rs"
    // @step And it contains a file named "catch_up.rs"
    // @step And it contains a file named "job_log.rs"
    // @step And it contains a file named "crud.rs"
    for file in [
        "mod.rs",
        "engine.rs",
        "state.rs",
        "cron_utils.rs",
        "types.rs",
        "trigger.rs",
        "agent_job.rs",
        "shell_job.rs",
        "catch_up.rs",
        "job_log.rs",
        "crud.rs",
    ] {
        let path = dir.join(file);
        assert!(
            path.is_file(),
            "rust/core/src/scheduler/{file} should exist"
        );
    }
    Ok(())
}

/// Scenario: codelet-core declares a SchedulerHooks trait
#[test]
fn codelet_core_declares_scheduler_hooks_trait() -> TestResult {
    // @step Given the file rust/core/src/scheduler/mod.rs is compiled
    let path = scheduler_core_dir()?.join("mod.rs");
    let source = codelet_test_helpers::source_composition::read_tree_or_file(&path)
        .or_fail("read core/scheduler/mod.rs")?;

    // @step Then it declares a public trait named "SchedulerHooks"
    assert!(
        source.contains("pub trait SchedulerHooks"),
        "scheduler/mod.rs should declare pub trait SchedulerHooks"
    );

    // @step And SchedulerHooks declares a method named "get_session_count" returning usize
    // @step And SchedulerHooks declares a method named "get_live_session_ids" returning Vec<Uuid>
    // @step And SchedulerHooks declares a method named "spawn_scheduled_session" returning Result<(), String>
    // @step And SchedulerHooks declares a method named "default_model" returning String
    for method in [
        "get_session_count",
        "get_live_session_ids",
        "spawn_scheduled_session",
        "default_model",
    ] {
        let needle = format!("fn {method}(");
        assert!(
            source.contains(&needle),
            "SchedulerHooks should declare fn {method}"
        );
    }
    Ok(())
}

/// Scenario: spawn_scheduler accepts an Arc<dyn SchedulerHooks>
#[test]
fn spawn_scheduler_takes_arc_dyn_scheduler_hooks() -> TestResult {
    // @step Given the file rust/core/src/scheduler/engine.rs is compiled
    let path = scheduler_core_dir()?.join("engine.rs");
    let source = codelet_test_helpers::source_composition::read_tree_or_file(&path)
        .or_fail("read core/scheduler/engine.rs")?;
    let normalised = normalise(&source);

    // @step Then it declares a public fn named "spawn_scheduler" whose last parameter has type "Arc<dyn SchedulerHooks>"
    assert!(
        source.contains("pub fn spawn_scheduler("),
        "engine.rs should declare pub fn spawn_scheduler"
    );
    assert!(
        normalised.contains("Arc<dyn SchedulerHooks>"),
        "spawn_scheduler should accept Arc<dyn SchedulerHooks>"
    );
    Ok(())
}

/// Scenario: The lifted engine has no crate::session_bindings references
#[test]
fn lifted_engine_has_no_session_bindings_references() -> TestResult {
    // @step Given the directory rust/core/src/scheduler/ exists
    let dir = scheduler_core_dir()?;
    assert!(dir.is_dir(), "scheduler core dir should exist");

    // @step Then no file under rust/core/src/scheduler/ contains the text "crate::session_bindings"
    // @step And no file under rust/core/src/scheduler/ contains the text "session_bindings::SessionManager"
    for (path, content) in read_all_rust_files(&dir) {
        assert!(
            !content.contains("crate::session_bindings"),
            "{} must not contain 'crate::session_bindings' — the lift requires hooks not direct references",
            path.display()
        );
        assert!(
            !content.contains("session_bindings::SessionManager"),
            "{} must not contain 'session_bindings::SessionManager'",
            path.display()
        );
    }
    Ok(())
}

/// Scenario: The pure CRUD helpers live in codelet-core::scheduler::crud
#[test]
fn crud_helpers_live_in_codelet_core_scheduler_crud() -> TestResult {
    // @step Given the file rust/core/src/scheduler/crud.rs is compiled
    let path = scheduler_core_dir()?.join("crud.rs");
    let source = codelet_test_helpers::source_composition::read_tree_or_file(&path)
        .or_fail("read core/scheduler/crud.rs")?;

    // @step Then it declares a public fn named "schedule_add"
    // @step And it declares a public fn named "schedule_list"
    // @step And it declares a public fn named "schedule_pause"
    // @step And it declares a public fn named "schedule_resume"
    // @step And it declares a public fn named "schedule_remove"
    for fname in [
        "schedule_add",
        "schedule_list",
        "schedule_pause",
        "schedule_resume",
        "schedule_remove",
    ] {
        let needle = format!("pub fn {fname}(");
        assert!(
            source.contains(&needle),
            "crud.rs should declare pub fn {fname}"
        );
    }

    // @step And the file does not contain the text "use napi"
    assert!(
        !source.contains("use napi"),
        "crud.rs must not contain 'use napi' — it lives in codelet-core (NAPI-free)"
    );
    Ok(())
}

/// Scenario: codelet-sessions handle_impl wires the five new methods to crud.rs
#[test]
fn sessions_handle_impl_wires_schedule_methods_to_crud() -> TestResult {
    // @step Given the file rust/sessions/src/handle_impl.rs is compiled
    let path = workspace_root()?.join("rust/sessions/src/handle_impl.rs");
    let source = codelet_test_helpers::source_composition::read_tree_or_file(&path)
        .or_fail("read sessions/src/handle_impl.rs")?;

    // @step Then it implements "schedule_add" by delegating to codelet_core::scheduler::crud
    // @step And it implements "schedule_list" by delegating to codelet_core::scheduler::crud
    // @step And it implements "schedule_pause" by delegating to codelet_core::scheduler::crud
    // @step And it implements "schedule_resume" by delegating to codelet_core::scheduler::crud
    // @step And it implements "schedule_remove" by delegating to codelet_core::scheduler::crud
    for fname in [
        "schedule_add",
        "schedule_list",
        "schedule_pause",
        "schedule_resume",
        "schedule_remove",
    ] {
        let impl_needle = format!("fn {fname}(");
        assert!(
            source.contains(&impl_needle),
            "handle_impl.rs should implement fn {fname}"
        );
    }
    // The delegation goes through codelet_core::scheduler::crud.
    assert!(
        source.contains("codelet_core::scheduler::crud") || source.contains("scheduler::crud::"),
        "handle_impl.rs should delegate to codelet_core::scheduler::crud"
    );
    Ok(())
}
