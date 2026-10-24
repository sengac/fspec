use std::time::{Duration, Instant};

use codelet_rpc_types::SessionId;
use ratatui::style::Color;

use crate::components::Action;

use super::state::App;
use super::transcript_sync::{TranscriptAction, TranscriptSync};

const PAGE_BYTES: u32 = 61_440;
const POLL_DELAY: Duration = Duration::from_secs(1);
const PAGE_TIMEOUT: Duration = Duration::from_secs(10);

impl App {
    pub(crate) fn transcript_sync_for(
        &mut self,
        session: &SessionId,
    ) -> Option<&mut TranscriptSync> {
        let nonce = self
            .agent_view_store
            .session_context_for(session)?
            .transcript_nonce;
        if self
            .transcript_sync
            .get(session)
            .is_some_and(|sync| sync.nonce != nonce)
        {
            self.transcript_sync.remove(session);
        }
        Some(
            self.transcript_sync
                .entry(session.clone())
                .or_insert_with(|| TranscriptSync::new(nonce)),
        )
    }

    pub(crate) fn request_transcript(&mut self, session: &SessionId) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let Some(sync) = self.transcript_sync_for(session) else {
            return;
        };
        if sync.suspended {
            sync.dirty = true;
            return;
        }
        if sync.in_flight.is_some() {
            sync.dirty = true;
            return;
        }
        let cursor = sync.assembler.cursor().cloned();
        let snapshot_end = sync.snapshot_end;
        let nonce = sync.nonce;
        sync.request_id = sync.request_id.wrapping_add(1);
        let request_id = sync.request_id;
        let backend = self.backend.clone();
        let sender = self.action_tx.clone();
        let session = session.clone();
        let task_session = session.clone();
        let task = tokio::spawn(async move {
            let result = match tokio::time::timeout(
                PAGE_TIMEOUT,
                backend.get_transcript_page(task_session.clone(), cursor, snapshot_end, PAGE_BYTES),
            )
            .await
            {
                Ok(result) => result.map_err(|error| error.to_string()),
                Err(error) => Err(format!("page request timed out: {error}")),
            };
            let _ = sender.send(Action::Transcript(TranscriptAction::Loaded {
                session: task_session,
                nonce,
                request_id,
                result,
            }));
        });
        if let Some(sync) = self.transcript_sync.get_mut(&session) {
            sync.in_flight = Some(task.abort_handle());
        }
        self.pending_tasks.push(task);
    }

    pub(crate) fn poll_transcripts(&mut self) {
        self.poll_transcripts_due(false);
    }

    fn poll_transcripts_due(&mut self, force: bool) {
        self.pending_tasks.retain(|task| !task.is_finished());
        let open = self
            .agent_view_store
            .open_sessions()
            .iter()
            .map(|context| context.id.clone())
            .collect::<Vec<_>>();
        self.transcript_sync
            .retain(|session, _| open.contains(session));
        for session in open {
            let ready = self.transcript_sync_for(&session).is_some_and(|sync| {
                sync.in_flight.is_none()
                    && !sync.suspended
                    && (force || sync.poll_after <= Instant::now())
            });
            if ready {
                self.request_transcript(&session);
            }
        }
    }

    pub(crate) fn dispatch_transcript(&mut self, action: &TranscriptAction) {
        match action {
            TranscriptAction::Inspected {
                session,
                nonce,
                inspection_id,
                result,
            } => {
                self.apply_archive_inspection(session, *nonce, *inspection_id, result);
            }
            TranscriptAction::Poll => self.poll_transcripts_due(true),
            TranscriptAction::Wake(session) => self.request_transcript(session),
            TranscriptAction::Live(session, chunk) => {
                self.handle_live_transcript_wake(session, chunk)
            }
            TranscriptAction::WakeError(session, error) => {
                self.maybe_push_error_dialog_for_chunk(&codelet_rpc_types::StreamChunk::error(
                    error.clone(),
                ));
                self.request_transcript(session);
            }
            TranscriptAction::Loaded {
                session,
                nonce,
                request_id,
                result,
            } => {
                if self
                    .transcript_sync
                    .get(session)
                    .is_some_and(|sync| sync.request_id == *request_id)
                {
                    self.apply_transcript_page(session, *nonce, result);
                }
            }
            TranscriptAction::Resumed {
                session,
                nonce,
                result,
            } => {
                if !self.transcript_response_is_current(session, *nonce) {
                    return;
                }
                match result {
                    Ok(()) => {
                        if let Some(sync) = self.transcript_sync.get_mut(session) {
                            sync.suspended = false;
                        }
                        self.request_transcript(session);
                    }
                    Err(error) => self.transcript_failure(session, error),
                }
            }
        }
    }

    pub(crate) fn transcript_response_is_current(&self, session: &SessionId, nonce: u64) -> bool {
        self.agent_view_store
            .session_context_for(session)
            .is_some_and(|context| context.transcript_nonce == nonce)
            && self
                .transcript_sync
                .get(session)
                .is_some_and(|sync| sync.nonce == nonce)
    }

    fn apply_transcript_page(
        &mut self,
        session: &SessionId,
        nonce: u64,
        result: &Result<codelet_rpc_types::TranscriptPage, String>,
    ) {
        if !self.transcript_response_is_current(session, nonce) {
            return;
        }
        let Some(sync) = self.transcript_sync.get_mut(session) else {
            return;
        };
        sync.in_flight = None;
        sync.poll_after = Instant::now() + POLL_DELAY;
        let page = match result {
            Ok(page) => page,
            Err(error) => {
                self.transcript_failure(session, error);
                return;
            }
        };
        if sync
            .snapshot_end
            .is_some_and(|end| end != page.snapshot_end)
        {
            self.transcript_failure(session, "snapshot boundary changed during hydration");
            return;
        }
        let chunk = match sync.assembler.accept(page) {
            Ok(chunk) => chunk,
            Err(error) => {
                self.transcript_failure(session, &error.to_string());
                return;
            }
        };
        let original = sync.assembler.original().cloned();
        sync.snapshot_end = if page.complete {
            None
        } else {
            Some(page.snapshot_end)
        };
        let more = !page.complete || page.fragment.is_some() || std::mem::take(&mut sync.dirty);
        let notice = if page.complete {
            sync.notice.take()
        } else {
            None
        };
        if let Some(context) = self.agent_view_store.session_context_mut_for(session) {
            if let Some(chunk) = chunk
                .as_ref()
                .filter(|chunk| super::dispatch_transcript_live::is_transcript_content(chunk))
            {
                context.record_transcript_chunk(chunk, original);
            }
            if let Some(notice) = notice {
                context.remove_notice_by_seq(notice);
            }
        }
        if matches!(
            chunk,
            Some(codelet_rpc_types::StreamChunk::ResponseAttemptAborted)
        ) && self.agent_view_store.current_session() == Some(session)
        {
            self.navigator
                .agent
                .dismiss_retracted_turn_modal(&self.agent_view_store);
        }
        if more {
            self.request_transcript(session);
        }
    }

    pub(crate) fn transcript_failure(&mut self, session: &SessionId, error: &str) {
        let Some(sync) = self.transcript_sync.get_mut(session) else {
            return;
        };
        sync.poll_after = Instant::now() + POLL_DELAY;
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
