//! Feature: spec/features/scripted-provider-dispatch-identity.feature

#[path = "support/bug267_environment.rs"]
mod environment;
#[path = "support/bug267.rs"]
mod support;
#[path = "support/bug267_wire.rs"]
mod wire;

use environment::TestResult;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn built_in_name_scripts_receive_real_shared_loop_requests() -> TestResult {
    // @step Given registered scripts collide with each supported native provider name
    for name in [
        "claude",
        "openai",
        "codex",
        "gemini",
        "zai",
        "github-copilot",
    ] {
        // @step When their selected custom implementations run through the shared loop
        // @step Then each script receives its own dialect and resolved model with armed native Done
        // @step And attributed Done completes each task with exact physical usage and no nudge
        support::run_case(name, true, false, None).await?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn alias_named_scripts_retain_selected_identity() -> TestResult {
    // @step Given registered scripts use native provider aliases
    for name in ["anthropic", "google", "z-ai", "copilot"] {
        // @step When their selected custom implementations run through the shared loop
        // @step Then each script receives its own dialect and resolved model with armed native Done
        // @step And attributed Done completes each task with exact physical usage and no nudge
        support::run_case(name, true, false, None).await?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn noncolliding_custom_dispatch_remains_compatible() -> TestResult {
    // @step Given a registered script has a noncolliding provider name
    // @step When its selected custom implementation runs through the shared loop
    // @step Then the script receives its own dialect and resolved model with armed native Done
    // @step And attributed Done completes the task with exact physical usage and no nudge
    support::run_case("script-owned", true, false, None).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn native_selection_and_disabled_shadowing_retain_native_dispatch() -> TestResult {
    // @step Given a genuine native OpenAI selection or a colliding script with shadowing disabled
    for (register, disabled, facade) in [
        (false, false, None),
        (true, true, None),
        (false, false, Some("openai")),
    ] {
        // @step When the selected implementation runs through the shared loop
        // @step Then only the native dialect reaches the local native endpoint with armed Done
        // @step And attributed Done completes the task with exact physical usage and no nudge
        support::run_case("openai", register, disabled, facade).await?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn explicit_incompatible_facade_remains_fail_closed() -> TestResult {
    // @step Given a selected Custom claude has an explicit openai facade override
    // @step When the selected implementation runs through the shared loop
    // @step Then the incompatible native getter rejects the turn without any HTTP request
    support::run_case("claude", true, false, Some("openai")).await
}
