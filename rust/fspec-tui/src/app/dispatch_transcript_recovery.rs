use std::time::{Duration, Instant};

use codelet_rpc_types::SessionId;
use ratatui::style::Color;

use super::dispatch_transcript::PAGE_TIMEOUT;
use super::state::App;
use super::transcript_sync::TranscriptAction;
use crate::components::Action;

impl App {
    pub(super) fn recover_transcript_generation(&mut self, session: &SessionId) {
        self.transcript_sync.remove(session);
        if let Some(context) = self.agent_view_store.session_context_mut_for(session) {
            context.reset_scrollback();
        }
        self.request_transcript(session);
    }

    pub(crate) fn transcript_failure(&mut self, session: &SessionId, error: &str) {
        let Some(sync) = self.transcript_sync.get_mut(session) else {
            return;
        };
        sync.poll_after = Instant::now() + Duration::from_secs(1);
        let message = format!(
            "[error] Transcript incomplete: {error}; retry with /resume or reopen the tab."
        );
        if let Some(context) = self.agent_view_store.session_context_mut_for(session) {
            match sync.notice {
                Some(sequence) => context.replace_notice_by_seq(sequence, message, Color::Yellow),
                None => sync.notice = Some(context.push_notice_line(message, Color::Yellow)),
            }
        }
    }

    pub(crate) fn spawn_transcript_resume(&mut self, session: &SessionId) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let Some(sync) = self.transcript_sync_for(session) else {
            return;
        };
        sync.suspended = sync.assembler.cursor().is_none();
        if sync.suspended {
            if let Some(task) = sync.in_flight.take() {
                task.abort();
            }
            sync.request_id = sync.request_id.wrapping_add(1);
        }
        let nonce = sync.nonce;
        let backend = self.backend.clone();
        let sender = self.action_tx.clone();
        let session = session.clone();
        self.pending_tasks.push(tokio::spawn(async move {
            let result =
                match tokio::time::timeout(PAGE_TIMEOUT, backend.resume_session(session.clone()))
                    .await
                {
                    Ok(result) => result.map_err(|error| error.to_string()),
                    Err(error) => Err(format!("session resume timed out: {error}")),
                };
            let _ = sender.send(Action::Transcript(TranscriptAction::Resumed {
                session,
                nonce,
                result,
            }));
        }));
    }
}
