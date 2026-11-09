#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/completion-goal-teardown-layout-guards.feature
//!
//! CONT-011 guard: goal teardown keeps a single reminder owner. Its own guard
//! test file per the product owner (2026-11-05, COV-075); the checks are the
//! same source-shape assertions as CONT-006's
//! stream_loop_wiring_pins_goal_mode_early_exit_and_single_teardown_invariants
//! in cont006_goal_immediate_termination.rs.

#[path = "support/stream_loop_source.rs"]
mod stream_loop_source_tree;

fn stream_loop_source() -> String {
    stream_loop_source_tree::stream_loop_source()
}

/// The `StreamUserItem(ToolResult)` arm region of stream_loop.rs — from the
/// arm's pattern to the start of the next (`Usage`) arm.
fn tool_result_arm_region(source: &str) -> &str {
    let start = source
        .find("StreamedUserContent::ToolResult")
        .expect("stream_loop.rs must have a StreamUserItem(ToolResult) arm");
    let end = source[start..]
        .find("MultiTurnStreamItem::Usage")
        .expect("Usage arm must follow the ToolResult arm")
        + start;
    &source[start..end]
}

#[test]
fn goal_teardown_guards_preserve_a_single_reminder_owner() {
    // @step Given the source file rust/cli/src/interactive/stream_loop.rs
    let source = stream_loop_source();

    // @step Then the ToolResult arm consults the early-exit decision without any goal gate
    let arm = tool_result_arm_region(&source);
    let consult = arm
        .find("decide_tool_result_early_exit")
        .expect("the ToolResult arm must consult the early-exit decision");
    let consult_block = &arm[consult..];
    let take = consult_block
        .find("take_done_acceptance")
        .expect("the consult must take from the DONE_ACCEPTANCE registry");
    assert!(
        !consult_block[..take].contains("goal"),
        "the early-exit consult must not gate on goal state — the CONT-005 \
         goal_active argument is lifted by CONT-006"
    );

    // @step And the FinalResponse fallback routes through the same shared teardown helper as the early exit
    let call_sites = source.matches("apply_finish_with_summary").count();
    assert_eq!(
        call_sites, 2,
        "exactly two apply_finish_with_summary call sites: the ToolResult-arm \
         early exit and the FinalResponse FinishWithSummary fallback"
    );
    let finish_arm = source
        .find("ContinueDecision::FinishWithSummary")
        .expect("stream_loop.rs must match on ContinueDecision::FinishWithSummary");
    let arm_end = source[finish_arm..]
        .find("ContinueDecision::FinishWithWarning")
        .expect("FinishWithWarning arm must follow FinishWithSummary")
        + finish_arm;
    assert!(
        source[finish_arm..arm_end].contains("apply_finish_with_summary"),
        "the FinishWithSummary arm body must call the shared teardown helper"
    );

    // @step And the goal announcement formatting lives only in the goal acceptance helper
    assert!(
        !source.contains("🎯 goal satisfied"),
        "stream_loop.rs must not format the goal announcement inline"
    );
    let helper_source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/interactive/done_early_exit.rs"
    ))
    .expect("done_early_exit.rs must exist");
    assert!(
        !helper_source.contains("🎯 goal satisfied"),
        "done_early_exit.rs must delegate the announcement to goal::apply_goal_acceptance"
    );
    let goal_source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/interactive/goal.rs"
    ))
    .expect("goal.rs must exist");
    assert!(
        goal_source.contains("🎯 goal satisfied"),
        "goal::apply_goal_acceptance must own the announcement formatting"
    );

    // @step And CompletionContract reminder removal lives only in the session goal clearing method
    assert!(
        !source.contains("remove_system_reminders_of_type"),
        "stream_loop.rs must not remove the CompletionContract reminder inline"
    );
    assert!(
        !helper_source.contains("remove_system_reminders_of_type"),
        "done_early_exit.rs must not remove the CompletionContract reminder inline"
    );
    let session_source =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/session/mod.rs"))
            .expect("session/mod.rs must exist");
    assert!(session_source.contains("mod reminders;"));
    let session_source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/session/reminders.rs"
    ))
    .expect("session/reminders.rs must own the registered reminder implementation");
    let clear_goal = session_source
        .find("pub fn clear_goal")
        .expect("Session::clear_goal must exist");
    let after_sig = clear_goal + "pub fn clear_goal".len();
    let clear_goal_end = session_source[after_sig..]
        .find("pub fn ")
        .map_or(session_source.len(), |n| after_sig + n);
    assert!(
        session_source[clear_goal..clear_goal_end].contains("CompletionContract"),
        "Session::clear_goal must own the CompletionContract reminder removal"
    );
}
