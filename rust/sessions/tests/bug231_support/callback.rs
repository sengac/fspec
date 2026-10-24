use super::{Fixture, Result};
use codelet_rpc_types::{FspecResult, StreamChunk};
use codelet_tools::execution_context::{scope_invocation, WorkUnitAttribution};
use std::time::Duration;

pub async fn verify(fixture: &Fixture) -> Result {
    let session = fixture.manager.get_session(&fixture.first.value)?;
    codelet_tools::set_fspec_handler_for_session(
        session.id,
        Some(codelet_agent_loop::fspec_handler::create_handler(
            session.clone(),
            true,
        )),
    );
    let receipt = fixture
        .manager
        .admit_tool_invocation(&fixture.first, WorkUnitAttribution::Card("AUTH-001".into()))?;
    let mut chunks = session.subscribe_to_stream();
    let responder = tokio::spawn(async move {
        let request = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let StreamChunk::FspecCommandRequest { fspec_request } = chunks.recv().await? {
                    return Ok::<_, tokio::sync::broadcast::error::RecvError>(fspec_request);
                }
            }
        })
        .await;
        let (request, result) = match request {
            Ok(Ok(request)) => {
                let result =
                    codelet_fspec_core::dispatch_command(codelet_fspec_core::DispatchRequest {
                        command: request.command.clone(),
                        args_json: request.args_json.clone(),
                        project_root: request.project_root.clone().into(),
                    });
                (Some(request), result)
            }
            _ => (
                None,
                codelet_fspec_core::DispatchResult {
                    success: false,
                    data: String::new(),
                    error: Some("callback timeout".into()),
                    system_reminder: None,
                },
            ),
        };
        session.send_fspec_result(FspecResult {
            success: result.success,
            data: result.data,
            error: result.error,
            system_reminder: result.system_reminder,
            tool_call_id: request.map_or_else(String::new, |request| request.tool_call_id),
        });
    });
    let result = scope_invocation(receipt, fixture.status("AUTH-002", "validating")).await;
    responder.await?;
    result?;
    assert_eq!(fixture.focus(&fixture.first)?.as_deref(), Some("AUTH-001"));
    Ok(())
}
