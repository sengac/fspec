use codelet_fspec_tui::App;
use codelet_rpc_types::SessionId;

pub fn rendered(app: &App, id: &SessionId) -> Result<String, Box<dyn std::error::Error>> {
    let context = app
        .agent_view_store()
        .session_context_for(id)
        .ok_or("missing context")?;
    Ok(context
        .scrollback
        .chunks()
        .iter()
        .filter_map(|chunk| context.scrollback.full_text_for_seq(chunk.seq))
        .collect::<Vec<_>>()
        .join("\n"))
}

pub async fn drain(app: &mut App) {
    loop {
        while let Some(task) = app.next_pending_task() {
            let _ = task.await;
        }
        let Some(action) = app.try_recv_action() else {
            break;
        };
        app.dispatch(action);
    }
}
