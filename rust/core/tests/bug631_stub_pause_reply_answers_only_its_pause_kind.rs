//! Feature: spec/features/a-pause-reply-answers-only-the-pause-kind-it-is-for.feature
//!
//! BUG-631: the in-process stub handle checks a pause reply against the
//! pending pause's kind as the daemon does: pause_human_action (AMGR-023)
//! answers a human action only, pause_triple a three-choice approval only.

use codelet_core::session_manager_handle::{SessionManagerHandle, StubSessionManagerHandle};
use codelet_rpc_types::{
    ApprovalChoice, HumanActionReply, PauseKind, PauseState, PendingRequest, RequestHolder,
    RequestKind, RequestRoute, SessionId,
};

fn pending(
    request_id: &str,
    session: &SessionId,
    kind: RequestKind,
    pause: PauseKind,
) -> PendingRequest<PauseState> {
    PendingRequest {
        route: RequestRoute {
            request_id: request_id.into(),
            origin: session.clone(),
            kind,
            chain: vec![session.clone()],
            holder: RequestHolder::User,
        },
        payload: PauseState {
            kind: pause,
            prompt: "bug631".into(),
            details: None,
            tool_call_id: None,
        },
    }
}

#[test]
fn scenario_the_stub_handle_checks_the_same_pause_kinds() -> Result<(), String> {
    // @step Given the in-process stub handle with a pending three-choice approval and a pending human action
    let stub = StubSessionManagerHandle::new();
    let approval = SessionId::from("bug631-approval".to_string());
    let human = SessionId::from("bug631-human-action".to_string());
    let approval_request = pending(
        "approval-1",
        &approval,
        RequestKind::Approval,
        PauseKind::Triple,
    );
    let human_request = pending(
        "human-1",
        &human,
        RequestKind::HumanAction,
        PauseKind::HumanAction,
    );
    stub.seed_pause_state(approval.clone(), approval_request.clone());
    stub.seed_pause_state(human.clone(), human_request.clone());
    // @step When pause_human_action names the approval's id and pause_triple names the human action's id
    let resume = stub.pause_human_action(&approval, "approval-1", HumanActionReply::Done);
    let triple = stub.pause_triple(&human, "human-1", ApprovalChoice::Approve);
    // @step Then both are rejected and both requests are still pending
    assert!(
        resume.is_err(),
        "pause_human_action answered a three-choice approval"
    );
    assert!(triple.is_err(), "pause_triple answered a human action");
    assert_eq!(stub.get_pause_state(&approval)?, Some(approval_request));
    assert_eq!(stub.get_pause_state(&human)?, Some(human_request));
    Ok(())
}
