use super::{full_protocol, CheckpointError, CodexCheckpoint, CodexHistory};

impl CodexHistory {
    pub fn reset(&self) -> Result<(), CheckpointError> {
        let generation = self
            .generation()?
            .checked_add(1)
            .ok_or(CheckpointError::Lineage("generation exhausted"))?;
        self.promote_checked(None, None, Some(generation))?;
        self.blocked
            .store(false, std::sync::atomic::Ordering::Release);
        Ok(())
    }

    pub fn transition_generation(&self, generation: u64) -> Result<(), CheckpointError> {
        self.promote(Some(CodexCheckpoint {
            version: 1,
            generation,
            protocol: full_protocol(),
            model: None,
            items: Vec::new(),
            compacted_users: Vec::new(),
        }))?;
        self.blocked
            .store(false, std::sync::atomic::Ordering::Release);
        Ok(())
    }

    pub fn generation(&self) -> Result<u64, CheckpointError> {
        self.state
            .lock()
            .map(|state| state.generation)
            .map_err(|_| CheckpointError::Inaccessible("state"))
    }

    pub fn prepare_generation(
        &self,
        generation: u64,
        model: &str,
    ) -> Result<CodexCheckpoint, CheckpointError> {
        if generation <= self.generation()? || model.is_empty() {
            return Err(CheckpointError::Lineage(
                "generation must advance with a selected model",
            ));
        }
        Ok(CodexCheckpoint {
            version: 1,
            generation,
            protocol: full_protocol(),
            model: Some(model.to_owned()),
            items: Vec::new(),
            compacted_users: Vec::new(),
        })
    }

    pub fn install_committed_generation(
        &self,
        generation: u64,
        checkpoint: CodexCheckpoint,
    ) -> Result<(), CheckpointError> {
        if checkpoint.version != 1 {
            return Err(CheckpointError::UnsupportedVersion(checkpoint.version));
        }
        if checkpoint.protocol != full_protocol()
            || checkpoint.generation != generation
            || checkpoint.model.as_ref().is_none_or(String::is_empty)
            || !checkpoint.items.is_empty()
            || !checkpoint.compacted_users.is_empty()
        {
            return Err(CheckpointError::Lineage("committed generation checkpoint"));
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| CheckpointError::Inaccessible("state"))?;
        let previous = state.generation;
        if generation < previous {
            return Err(CheckpointError::Lineage(
                "committed generation does not advance",
            ));
        }
        if generation == previous {
            if state
                .checkpoint
                .as_ref()
                .is_none_or(|saved| saved.model != checkpoint.model)
            {
                return Err(CheckpointError::Lineage(
                    "committed generation model changed",
                ));
            }
            self.blocked
                .store(false, std::sync::atomic::Ordering::Release);
            return Ok(());
        }
        state.checkpoint = Some(checkpoint);
        state.generation = generation;
        self.blocked
            .store(false, std::sync::atomic::Ordering::Release);
        Ok(())
    }
}
