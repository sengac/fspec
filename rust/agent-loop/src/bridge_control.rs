use codelet_rpc_types::{ApprovalChoice, RequestStatus, StreamChunk};
use codelet_sessions::background_session::{BackgroundSession, PauseReply};
use codelet_tools::request_user_input::HitlResponse;
use std::sync::Arc;

/// The bridge (remote dashboard) control actions for `session`: interrupt,
/// clear, (AMGR-020) a `pause_response` that names the request id of the
/// approval it answers, (BRIDGE-022) a `question_response` that names
/// the question's request id, and (AMGR-023) a `human_action_response`
/// (done or cancel) that names the human action's request id; a reply without the id, with another id or
/// with an unknown response answers nothing.
pub fn control_handler(session: Arc<BackgroundSession>) -> codelet_tools::ControlHandler {
    Arc::new(
        move |action: &str, response: Option<&str>, request_id: Option<&str>| match action {
            "interrupt" => {
                tracing::info!("Bridge control: interrupting session");
                session.interrupt();
            }
            "clear" => {
                tracing::info!("Bridge control: clearing session");
                tokio::task::block_in_place(|| clear_history(&session));
            }
            "pause_response" => {
                if let Err(error) = answer_pause(&session, response, request_id) {
                    tracing::warn!(%error, "Bridge control: pause_response rejected");
                }
            }
            "human_action_response" => {
                if let Err(error) = answer_human_action(&session, response, request_id) {
                    tracing::warn!(%error, "Bridge control: human_action_response rejected");
                }
            }
            "question_response" => {
                if let Err(error) = answer_question(&session, response, request_id) {
                    tracing::warn!(%error, "Bridge control: question_response rejected");
                }
            }
            _ => {
                tracing::warn!("Bridge control: unknown action '{}'", action);
            }
        },
    )
}

fn answer_pause(
    session: &BackgroundSession,
    response: Option<&str>,
    request_id: Option<&str>,
) -> Result<(), String> {
    let request_id = request_id.ok_or("pause_response names no request_id")?;
    // BUG-631: the bridge speaks the three-choice vocabulary, so it answers
    // a three-choice approval only.
    let choice = match response.ok_or("pause_response has no response")? {
        "allow_once" => ApprovalChoice::Approve,
        "allow_session" => ApprovalChoice::ApproveSession,
        "deny" => ApprovalChoice::Deny,
        other => return Err(format!("unknown pause response '{other}'")),
    };
    session.answer_pause(
        request_id,
        &codelet_rpc_types::RequestHolder::User,
        PauseReply::Triple(choice),
    )
}

/// AMGR-023: do ("done") or cancel ("cancel") the human action named
/// `request_id`, as the user.
fn answer_human_action(
    session: &BackgroundSession,
    response: Option<&str>,
    request_id: Option<&str>,
) -> Result<(), String> {
    let request_id = request_id.ok_or("human_action_response names no request_id")?;
    let reply = match response.ok_or("human_action_response has no response")? {
        "done" => codelet_rpc_types::HumanActionReply::Done,
        "cancel" => codelet_rpc_types::HumanActionReply::Cancel,
        other => return Err(format!("unknown human_action_response '{other}'")),
    };
    session.answer_pause(
        request_id,
        &codelet_rpc_types::RequestHolder::User,
        PauseReply::HumanAction(reply),
    )
}

/// BRIDGE-022: answer the question named `request_id` as the user (taking
/// it over from a session that holds it) with the HitlResponse JSON text.
fn answer_question(
    session: &BackgroundSession,
    response: Option<&str>,
    request_id: Option<&str>,
) -> Result<(), String> {
    let request_id = request_id.ok_or("question_response names no request_id")?;
    let response = response.ok_or("question_response has no response")?;
    let response: HitlResponse = serde_json::from_str(response)
        .map_err(|error| format!("question_response response does not parse: {error}"))?;
    session.answer_question(
        request_id,
        &codelet_rpc_types::RequestHolder::User,
        response,
    )
}

/// BRIDGE-022: the JSON the bridge forwards for `chunk` from `session`. The
/// Pending event of the session's own question also carries the question
/// (`request`), so the remote side can show and answer it.
pub fn bridge_event_json(session: &BackgroundSession, chunk: &StreamChunk) -> serde_json::Value {
    let mut json = crate::stream_chunk_json::stream_chunk_to_json_value(chunk);
    if let StreamChunk::InteractionRequestChanged {
        route,
        status: RequestStatus::Pending,
    } = chunk
    {
        let own = session
            .pending_question()
            .filter(|(pending, _)| pending.request_id == route.request_id);
        if let Some((_, request)) = own {
            match serde_json::to_value(request) {
                Ok(request) => json["request"] = request,
                Err(error) => tracing::error!(%error, "Cannot encode the question for the bridge"),
            }
        }
    }
    json
}

fn clear_history(session: &BackgroundSession) {
    if let Err(error) = session.clear_history() {
        tracing::warn!(%error, "Bridge conversation clear failed");
        session.handle_output(StreamChunk::Error {
            error: format!("Failed to clear conversation: {error}"),
        });
    }
}
