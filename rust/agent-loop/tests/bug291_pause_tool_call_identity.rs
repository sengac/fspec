//! BUG-291: the pause state carries its details and the gated tool call's
//! id in separate fields.
//!
//! Feature: spec/features/pause-state-tool-call-identity.feature

#[path = "rig016_interrupt_steer_queue/harness.rs"]
pub mod harness;

use codelet_tools::tool_pause::{
    active_tool_call, set_active_tool_call, PauseKind, PauseRequest, PauseState,
};
use harness::{Harness, Reply, TestResult};

fn request(details: &str) -> PauseRequest {
    PauseRequest {
        kind: PauseKind::Triple,
        tool_name: "Bash".into(),
        message: "Approve this command?".into(),
        details: Some(details.into()),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn the_stream_loop_records_the_running_tool_call_for_the_session() -> TestResult {
    // BUG-447: this test drives tools without a session manager callback.
    codelet_tools::facade::declare_unrestricted_tool_access(
        "tests drive tools without a session manager",
    );
    // @step Given a session whose turn is running a Bash tool call "rig016-call"
    let mut harness = Harness::new(vec![
        Reply::Bash("sleep 2; echo tool-finished"),
        Reply::Text("done", 0),
    ])
    .await;
    harness.session.send_input("run the build".into(), None)?;
    harness.until_requests(1).await;

    // @step When the tool is executing
    // TEST-042: wait for the running call to be recorded (between yields,
    // under the deadlock guard), not a fixed time.
    let session = harness.session.id;
    codelet_test_helpers::wait_until("the tool call is recorded as running", || {
        std::future::ready(active_tool_call(session).is_some())
    })
    .await?;

    // @step Then the session's active tool call is "rig016-call"
    assert_eq!(
        active_tool_call(harness.session.id).as_deref(),
        Some("rig016-call")
    );

    // @step And once the tool result arrives the session has no active tool call
    harness.until_done(1).await;
    assert_eq!(active_tool_call(harness.session.id), None);
    Ok(())
}

#[test]
fn a_pause_raised_during_a_tool_call_carries_that_calls_id() {
    // @step Given a session whose active tool call is "call-7"
    let session = uuid::Uuid::new_v4();
    set_active_tool_call(session, Some("call-7".into()));

    // @step When a tool requests a pause with details "rm -rf build"
    let state = PauseState::for_session(session, request("rm -rf build"));
    set_active_tool_call(session, None);

    // @step Then the pause state's tool_call_id is "call-7"
    assert_eq!(state.tool_call_id.as_deref(), Some("call-7"));

    // @step And its details are "rm -rf build"
    assert_eq!(state.details.as_deref(), Some("rm -rf build"));
}

#[test]
fn the_rpc_pause_state_keeps_details_and_the_tool_call_id_apart() {
    // @step Given an internal pause state with details "rm -rf build" and tool call id "call-7"
    let session = uuid::Uuid::new_v4();
    set_active_tool_call(session, Some("call-7".into()));
    let internal = PauseState::for_session(session, request("rm -rf build"));
    set_active_tool_call(session, None);

    // @step When it is converted for get_pause_state
    let rpc = codelet_sessions::conversions::pause_state_to_rpc(internal);

    // @step Then the RPC pause state's details are "rm -rf build"
    assert_eq!(rpc.details.as_deref(), Some("rm -rf build"));

    // @step And its tool_call_id is "call-7"
    assert_eq!(rpc.tool_call_id.as_deref(), Some("call-7"));
}
