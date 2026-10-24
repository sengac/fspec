use std::time::Duration;

use codelet_rpc_types::{SessionId, TranscriptAssembler, TranscriptCursor};
use ratatui::style::Color;

use super::state::App;
use super::transcript_sync::TranscriptAction;
use crate::components::Action;
use crate::transport::FspecBackend;
use crate::views::agent::{ChunkKind, ChunkSource};

impl App {
    pub(crate) fn handle_archive_command(&mut self, session: &SessionId, text: &str) -> bool {
        let mut words = text.split_whitespace();
        if words.next() != Some("/archive") {
            return false;
        }
        let sequence = words.next().and_then(|word| word.parse::<u64>().ok());
        let Some(sequence) = sequence.filter(|_| words.next().is_none()) else {
            if let Some(context) = self.agent_view_store.session_context_mut_for(session) {
                context.push_notice_line(
                    "[error] Usage: /archive <event-sequence> (starting at 0)",
                    Color::Yellow,
                );
            }
            return true;
        };
        if tokio::runtime::Handle::try_current().is_err() {
            return true;
        }
        let Some(sync) = self.transcript_sync_for(session) else {
            return true;
        };
        if let Some(task) = sync.inspector.take() {
            task.abort();
        }
        sync.inspection_id = sync.inspection_id.wrapping_add(1);
        let inspection_id = sync.inspection_id;
        let nonce = sync.nonce;
        let generation = sync
            .assembler
            .cursor()
            .map(|cursor| cursor.generation.clone());
        let backend = self.backend.clone();
        let sender = self.action_tx.clone();
        let session = session.clone();
        let task_session = session.clone();
        let task = tokio::spawn(async move {
            let result = match tokio::time::timeout(
                Duration::from_secs(30),
                inspect_event(backend.as_ref(), &task_session, sequence, generation),
            )
            .await
            {
                Ok(result) => result,
                Err(error) => Err(format!("archive inspection timed out: {error}")),
            };
            let _ = sender.send(Action::Transcript(TranscriptAction::Inspected {
                session: task_session,
                nonce,
                inspection_id,
                result,
            }));
        });
        if let Some(sync) = self.transcript_sync.get_mut(&session) {
            sync.inspector = Some(task.abort_handle());
        }
        self.pending_tasks.push(task);
        true
    }

    pub(crate) fn apply_archive_inspection(
        &mut self,
        session: &SessionId,
        nonce: u64,
        inspection_id: u64,
        result: &Result<String, String>,
    ) {
        if !self.transcript_response_is_current(session, nonce) {
            return;
        }
        let Some(sync) = self.transcript_sync.get_mut(session) else {
            return;
        };
        if sync.inspection_id != inspection_id {
            return;
        }
        sync.inspector = None;
        let Some(context) = self.agent_view_store.session_context_mut_for(session) else {
            return;
        };
        match result {
            Ok(full_text) => {
                let Some((title, _payload)) = full_text.split_once('\n') else {
                    context.push_notice_line(
                        "[error] Archive incomplete: invalid inspection payload",
                        Color::Yellow,
                    );
                    return;
                };
                context.push_chunk(ChunkSource {
                    text: format!(
                        "{title} — select this turn to inspect the complete original payload"
                    ),
                    color: Color::Cyan,
                    kind: ChunkKind::Notification,
                    is_streaming: false,
                    full_text: Some(full_text.clone()),
                });
            }
            Err(error) => {
                context.push_notice_line(
                    format!("[error] Archive incomplete: {error}; retry /archive <event-sequence>"),
                    Color::Yellow,
                );
            }
        }
    }
}

async fn inspect_event(
    backend: &dyn FspecBackend,
    session: &SessionId,
    sequence: u64,
    generation: Option<String>,
) -> Result<String, String> {
    let initial = backend
        .get_transcript_page(session.clone(), None, None, 61_440)
        .await
        .map_err(|error| error.to_string())?;
    if generation
        .as_ref()
        .is_some_and(|generation| generation != &initial.generation)
    {
        return Err("archive generation changed; reopen the tab before inspecting it".into());
    }
    if sequence >= initial.snapshot_end {
        return Err(format!(
            "event {sequence} is outside the archive ({} events)",
            initial.snapshot_end
        ));
    }
    let mut assembler = TranscriptAssembler::from_cursor(TranscriptCursor {
        generation: initial.generation,
        sequence,
        byte_offset: 0,
    })
    .map_err(|error| error.to_string())?;
    loop {
        let page = backend
            .get_transcript_page(
                session.clone(),
                assembler.cursor().cloned(),
                Some(initial.snapshot_end),
                61_440,
            )
            .await
            .map_err(|error| error.to_string())?;
        if let Some(chunk) = assembler.accept(&page).map_err(|error| error.to_string())? {
            let payload =
                serde_json::to_string_pretty(&chunk).map_err(|error| error.to_string())?;
            let coordinates = serde_json::to_string_pretty(&assembler.original())
                .map_err(|error| error.to_string())?;
            return Ok(format!(
                "Archive event {sequence}\nOriginal: {coordinates}\n{payload}"
            ));
        }
        if page.complete {
            return Err(format!("event {sequence} is incomplete"));
        }
    }
}
