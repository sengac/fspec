//! AMGR-021: a parent's view of, and replies to, the questions and
//! approvals of the sub-agents below it.
//!
//! Feature: spec/features/a-parent-answers-approves-or-escalates-its-sub-agents-request.feature

use codelet_rpc_types::{ApprovalChoice, RequestHolder, RequestKind, RequestRoute, SessionId};
use codelet_sessions::background_session::{BackgroundSession, PauseReply, PendingInteraction};
use codelet_sessions::session_manager::SessionManager;
use codelet_tools::agent_manager::{
    AgentManagerResult, ApprovalView, ApproveChoice, AwaitOutcome, PendingRequestView,
    RequestAnswer,
};
use codelet_tools::request_user_input::{HitlAnswer, HitlResponse};
use codelet_tools::tool_pause::PauseKind;
use std::sync::Arc;
use uuid::Uuid;

fn kind_name(kind: RequestKind) -> &'static str {
    match kind {
        RequestKind::Question => "question",
        RequestKind::Approval => "approval",
        RequestKind::HumanAction => "human_action",
    }
}

fn holder_name(holder: &RequestHolder) -> String {
    match holder {
        RequestHolder::User => "user".to_string(),
        RequestHolder::Session(session) => session.value.clone(),
    }
}

/// The request as AgentManager reports it.
pub(super) fn view(interaction: &PendingInteraction) -> PendingRequestView {
    let route = interaction.route();
    let (questions, approval) = match interaction {
        PendingInteraction::Question(_, request) => (Some(request.questions.clone()), None),
        PendingInteraction::Pause(_, state) => (
            None,
            Some(ApprovalView {
                tool_name: state.tool_name.clone(),
                message: state.message.clone(),
                details: state.details.clone(),
            }),
        ),
    };
    PendingRequestView {
        request_id: route.request_id.clone(),
        origin_session_id: route.origin.value.clone(),
        kind: kind_name(route.kind).to_string(),
        holder: holder_name(&route.holder),
        questions,
        approval,
    }
}

/// Every live session's pending question and approval.
pub(super) fn pending_all(manager: &SessionManager) -> Vec<PendingInteraction> {
    manager
        .all_in_memory_session_ids()
        .iter()
        .filter_map(|id| manager.get_session(id).ok())
        .flat_map(|session| session.pending_interactions())
        .collect()
}

/// The session waiting on request `request_id`, with the request.
fn find(
    manager: &SessionManager,
    request_id: &str,
) -> Option<(Arc<BackgroundSession>, PendingInteraction)> {
    manager
        .all_in_memory_session_ids()
        .iter()
        .filter_map(|id| manager.get_session(id).ok())
        .find_map(|session| {
            session
                .pending_interactions()
                .into_iter()
                .find(|interaction| interaction.route().request_id == request_id)
                .map(|interaction| (session.clone(), interaction))
        })
}

fn caller_holder(caller: Uuid) -> RequestHolder {
    RequestHolder::Session(SessionId::from(caller.to_string()))
}

/// What await_idle reports for awaited session `awaited` when `route`
/// (a request at or below it) concerns the caller: needs_input when the
/// caller holds it; waiting_on_user, once, when it is a human action of
/// the caller's own sub-agent (only when `report_human`; reporting marks it).
pub(super) fn concerns(
    route: &RequestRoute,
    awaited: &str,
    caller: &BackgroundSession,
    report_human: bool,
) -> Option<AwaitOutcome> {
    if !route.chain.iter().any(|member| member.value == awaited) {
        return None;
    }
    let caller_id = caller.id.to_string();
    match &route.holder {
        RequestHolder::Session(holder) if holder.value == caller_id => {
            Some(AwaitOutcome::NeedsInput)
        }
        _ if report_human
            && route.kind == RequestKind::HumanAction
            && route
                .chain
                .get(1)
                .is_some_and(|parent| parent.value == caller_id)
            && caller.first_report_of(&route.request_id) =>
        {
            Some(AwaitOutcome::WaitingOnUser)
        }
        _ => None,
    }
}

/// The first pending request at or below `awaited` that concerns the
/// caller, from the live sessions now.
pub(super) fn held_below(
    manager: &SessionManager,
    awaited: &str,
    caller: &BackgroundSession,
    report_human: bool,
) -> Option<(AwaitOutcome, PendingInteraction)> {
    pending_all(manager).into_iter().find_map(|interaction| {
        concerns(interaction.route(), awaited, caller, report_human)
            .map(|outcome| (outcome, interaction))
    })
}

/// The pending request named by `route`, if it is still pending.
pub(super) fn still_pending(
    manager: &SessionManager,
    route: &RequestRoute,
) -> Option<PendingRequestView> {
    let session = manager.get_session(&route.origin.value).ok()?;
    session
        .pending_interactions()
        .iter()
        .find(|interaction| interaction.route().request_id == route.request_id)
        .map(view)
}

/// AgentManager `answer`: answer a question the caller holds.
pub(super) fn handle_answer(
    manager: &SessionManager,
    caller: Uuid,
    request_id: &str,
    answers: Option<Vec<RequestAnswer>>,
    cancelled: Option<bool>,
) -> AgentManagerResult {
    let Some((session, interaction)) = find(manager, request_id) else {
        return AgentManagerResult::request_not_found(request_id);
    };
    if let PendingInteraction::Pause(route, _) = &interaction {
        return AgentManagerResult::request_rejected(&format!(
            "Request {request_id} is {}, not a question; use approve for an approval",
            kind_name(route.kind)
        ));
    }
    let response = if cancelled == Some(true) {
        HitlResponse::Cancelled { cancelled: true }
    } else {
        match answers {
            Some(answers) if !answers.is_empty() => HitlResponse::Answered {
                answers: answers
                    .into_iter()
                    .map(|answer| {
                        (
                            answer.id,
                            HitlAnswer {
                                selected: answer.selected,
                                other: answer.other,
                            },
                        )
                    })
                    .collect(),
            },
            _ => {
                return AgentManagerResult::invalid_parameter(
                    "answer needs answers [{id, selected, other}] or cancelled: true",
                )
            }
        }
    };
    match session.answer_question(request_id, &caller_holder(caller), response) {
        Ok(()) => AgentManagerResult::RequestAnswered {
            answered: true,
            request_id: request_id.to_string(),
        },
        Err(error) => AgentManagerResult::request_rejected(&error),
    }
}

/// AgentManager `approve`: decide an approval the caller holds.
pub(super) fn handle_approve(
    manager: &SessionManager,
    caller: Uuid,
    request_id: &str,
    choice: ApproveChoice,
) -> AgentManagerResult {
    let Some((session, interaction)) = find(manager, request_id) else {
        return AgentManagerResult::request_not_found(request_id);
    };
    let PendingInteraction::Pause(_, state) = &interaction else {
        return AgentManagerResult::request_rejected(&format!(
            "Request {request_id} is a question, not an approval; use answer"
        ));
    };
    let reply = match (state.kind, choice) {
        (PauseKind::Continue, _) => {
            return AgentManagerResult::request_rejected(&format!(
                "Request {request_id} is a human action; only the user can do it"
            ))
        }
        (PauseKind::Confirm, ApproveChoice::Approve) => PauseReply::Confirm { accept: true },
        (PauseKind::Confirm, ApproveChoice::Deny) => PauseReply::Confirm { accept: false },
        (PauseKind::Confirm, ApproveChoice::ApproveSession) => {
            return AgentManagerResult::request_rejected(&format!(
                "Request {request_id} is a two-choice confirm; choose approve or deny"
            ))
        }
        (PauseKind::Triple, ApproveChoice::Approve) => PauseReply::Triple(ApprovalChoice::Approve),
        (PauseKind::Triple, ApproveChoice::ApproveSession) => {
            PauseReply::Triple(ApprovalChoice::ApproveSession)
        }
        (PauseKind::Triple, ApproveChoice::Deny) => PauseReply::Triple(ApprovalChoice::Deny),
    };
    match session.answer_pause(request_id, &caller_holder(caller), reply) {
        Ok(()) => AgentManagerResult::RequestAnswered {
            answered: true,
            request_id: request_id.to_string(),
        },
        Err(error) => AgentManagerResult::request_rejected(&error),
    }
}

/// AgentManager `escalate`: pass a request the caller holds up its chain.
pub(super) fn handle_escalate(
    manager: &SessionManager,
    caller: Uuid,
    request_id: &str,
) -> AgentManagerResult {
    let Some((session, _)) = find(manager, request_id) else {
        return AgentManagerResult::request_not_found(request_id);
    };
    match session.escalate_request(request_id, caller) {
        Ok(route) => AgentManagerResult::RequestEscalated {
            request_id: request_id.to_string(),
            holder: holder_name(&route.holder),
        },
        Err(error) => AgentManagerResult::request_rejected(&error),
    }
}
