use super::*;

impl BackgroundSession {
    pub fn requires_model_restore(&self) -> bool {
        self.manifest_restore.load(Ordering::Acquire)
    }

    /// BUG-683: a session created from its manifest takes no input until
    /// `resume_session` has restored its saved history and provider state.
    /// A turn run before that sees an empty provider history: its Codex
    /// checkpoint is refused as a rollback below the saved generation and
    /// the session stays blocked.
    pub fn ensure_restored(&self) -> Result<(), String> {
        if self.requires_model_restore() {
            return Err(format!(
                "Session {} is still restoring its saved history, so this input was not \
                 accepted. Send it once the resume completes; if the resume failed, resume \
                 the session again.",
                self.id
            ));
        }
        Ok(())
    }

    pub(crate) fn prepare_manifest_restore(&self) {
        self.manifest_restore.store(true, Ordering::Release);
    }

    pub(crate) fn complete_manifest_restore(&self) {
        self.manifest_restore.store(false, Ordering::Release);
    }

    pub fn resident_transcript_chunks(&self) -> usize {
        self.output_buffer
            .read()
            .expect("output buffer lock poisoned")
            .resident_chunks()
    }

    pub fn seed_archival_transcript<I>(
        &self,
        chunks: I,
    ) -> Result<bool, codelet_rpc_types::TranscriptError>
    where
        I: IntoIterator<
            Item = Result<
                (StreamChunk, Option<codelet_rpc_types::TranscriptOriginal>),
                codelet_rpc_types::TranscriptError,
            >,
        >,
    {
        use codelet_rpc_types::TranscriptError;
        let _seed = self
            .transcript_seed
            .lock()
            .map_err(|error| TranscriptError::JournalUnavailable(error.to_string()))?;
        let generation = {
            let journal = self
                .output_buffer
                .read()
                .map_err(|error| TranscriptError::JournalUnavailable(error.to_string()))?;
            if journal.is_seeded() {
                return Ok(false);
            }
            journal.generation().to_owned()
        };
        let archive = match crate::transcript_archive::TranscriptArchive::build(chunks) {
            Ok(archive) => archive,
            Err(error) => {
                self.output_buffer
                    .write()
                    .map_err(|error| TranscriptError::JournalUnavailable(error.to_string()))?
                    .record_archive_failure(&generation, &error);
                return Err(error);
            }
        };
        self.output_buffer
            .write()
            .map_err(|error| TranscriptError::JournalUnavailable(error.to_string()))?
            .install_archive(archive, &generation)
    }

    pub fn get_transcript_page(
        &self,
        cursor: Option<codelet_rpc_types::TranscriptCursor>,
        snapshot_end: Option<u64>,
        max_bytes: u32,
    ) -> Result<codelet_rpc_types::TranscriptPage, codelet_rpc_types::TranscriptError> {
        self.output_buffer
            .read()
            .map_err(|error| {
                codelet_rpc_types::TranscriptError::JournalUnavailable(error.to_string())
            })?
            .page(cursor, snapshot_end, max_bytes)
    }

    pub fn get_buffered_output(&self, limit: usize) -> Vec<StreamChunk> {
        let buffer = self
            .output_buffer
            .read()
            .expect("output buffer lock poisoned");
        buffer
            .buffered(limit)
            .unwrap_or_else(|error| vec![StreamChunk::error(error.to_string())])
    }

    pub fn subscribe_to_stream(&self) -> broadcast::Receiver<StreamChunk> {
        self.supervisor_broadcast.subscribe()
    }

    pub fn subscribe_to_relay(
        &self,
    ) -> Result<
        crate::subordinate_output::RelaySubscription,
        crate::subordinate_output::RelayOutputError,
    > {
        let _projection = self
            .committed_context
            .lock()
            .map_err(|_| crate::subordinate_output::RelayOutputError::Poisoned)?;
        self.relay_output
            .subscribe(self.supervisor_broadcast.subscribe())
    }
}
