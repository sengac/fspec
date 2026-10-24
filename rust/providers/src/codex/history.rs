#[path = "checkpoint_error.rs"]
mod error;
#[path = "history_generation.rs"]
mod generation;
#[path = "history_items.rs"]
mod items;
#[path = "history_lineage.rs"]
mod lineage;
#[path = "history_projection.rs"]
mod projection;
#[path = "history_replay.rs"]
mod replay;
#[cfg(test)]
#[path = "history_state_tests.rs"]
mod state_tests;
#[path = "history_tool_calls.rs"]
mod tool_calls;
pub use error::CheckpointError;
pub use items::CodexItem;
use rig::completion::CompletionError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CodexCheckpoint {
    pub version: u32,
    #[serde(default)]
    pub generation: u64,
    #[serde(default = "full_protocol")]
    pub protocol: String,
    pub model: Option<String>,
    pub items: Vec<CodexItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub compacted_users: Vec<CodexItem>,
}

fn full_protocol() -> String {
    "full-current-dag".into()
}

impl std::fmt::Debug for CodexCheckpoint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("CodexCheckpoint { .. }")
    }
}

type CheckpointWriter =
    Arc<dyn Fn(Option<CodexCheckpoint>, u64) -> Result<(), String> + Send + Sync>;

#[derive(Default)]
struct HistoryState {
    checkpoint: Option<CodexCheckpoint>,
    generation: u64,
    tools: super::custom_tools::CustomToolRegistry,
}

#[derive(Clone, Default)]
pub struct CodexHistory {
    state: Arc<Mutex<HistoryState>>,
    writer: Arc<Mutex<Option<CheckpointWriter>>>,
    blocked: Arc<AtomicBool>,
}

impl std::fmt::Debug for CodexHistory {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("CodexHistory { .. }")
    }
}

impl CodexHistory {
    pub(super) fn block(&self) {
        self.blocked.store(true, Ordering::Release);
    }

    pub fn snapshot(&self) -> Result<Option<CodexCheckpoint>, CheckpointError> {
        self.state
            .lock()
            .map(|state| state.checkpoint.clone())
            .map_err(|_| CheckpointError::Inaccessible("state"))
    }

    pub fn restore(&self, checkpoint: Option<CodexCheckpoint>) -> Result<(), CheckpointError> {
        let generation = checkpoint.as_ref().map_or(0, |saved| saved.generation);
        self.restore_state(checkpoint, generation)
    }

    pub(super) fn restore_state(
        &self,
        checkpoint: Option<CodexCheckpoint>,
        generation: u64,
    ) -> Result<(), CheckpointError> {
        if let Some(saved) = &checkpoint {
            if saved.version != 1 {
                self.blocked.store(true, Ordering::Release);
                return Err(CheckpointError::UnsupportedVersion(saved.version));
            }
            if saved.protocol != full_protocol() {
                self.blocked.store(true, Ordering::Release);
                return Err(CheckpointError::Lineage("protocol"));
            }
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| CheckpointError::Inaccessible("state"))?;
        if generation < state.generation {
            return Err(CheckpointError::Lineage("installed generation rollback"));
        }
        state.checkpoint = checkpoint;
        state.generation = generation;
        self.blocked.store(false, Ordering::Release);
        Ok(())
    }

    pub(super) fn ensure_generation(&self, generation: u64) -> Result<(), CompletionError> {
        self.validate_checkpoint()?;
        if self.generation()? != generation {
            return Err(CheckpointError::Lineage("prepared generation changed").into());
        }
        Ok(())
    }

    pub fn set_writer(
        &self,
        writer: Arc<dyn Fn(Option<CodexCheckpoint>) -> Result<(), String> + Send + Sync>,
    ) -> Result<(), CheckpointError> {
        self.set_writer_with_generation(Arc::new(move |checkpoint, _| writer(checkpoint)))
    }

    pub(super) fn set_writer_with_generation(
        &self,
        writer: CheckpointWriter,
    ) -> Result<(), CheckpointError> {
        *self
            .writer
            .lock()
            .map_err(|_| CheckpointError::Inaccessible("writer"))? = Some(writer);
        Ok(())
    }

    fn promote(&self, checkpoint: Option<CodexCheckpoint>) -> Result<(), CheckpointError> {
        self.promote_checked(checkpoint, None, None)
    }

    fn promote_checked(
        &self,
        checkpoint: Option<CodexCheckpoint>,
        generation: Option<u64>,
        reset_generation: Option<u64>,
    ) -> Result<(), CheckpointError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| CheckpointError::Inaccessible("state"))?;
        if let Some(generation) = generation {
            if self.blocked.load(Ordering::Acquire) {
                return Err(CheckpointError::Blocked);
            }
            if state.generation != generation {
                return Err(CheckpointError::Lineage(
                    "generation changed before checkpoint publication",
                ));
            }
        }
        let next_generation = checkpoint.as_ref().map_or_else(
            || reset_generation.unwrap_or(state.generation),
            |saved| saved.generation,
        );
        if next_generation < state.generation
            || (generation.is_none() && next_generation == state.generation)
        {
            return Err(CheckpointError::Lineage("generation must advance"));
        }
        let writer = self
            .writer
            .lock()
            .map_err(|_| CheckpointError::Inaccessible("writer"))?;
        if let Some(writer) = writer.as_ref() {
            if let Err(error) = writer(checkpoint.clone(), next_generation) {
                self.blocked.store(true, Ordering::Release);
                return Err(CheckpointError::Persistence(error));
            }
        }
        state.checkpoint = checkpoint;
        state.generation = next_generation;
        Ok(())
    }

    pub fn prepare(&self, request: &mut Value) -> Result<(), CompletionError> {
        self.prepare_with_tools(request, &super::custom_tools::CustomToolRegistry::default())
    }

    pub(super) fn prepare_with_tools(
        &self,
        request: &mut Value,
        tools: &super::custom_tools::CustomToolRegistry,
    ) -> Result<(), CompletionError> {
        super::memory_policy::validate_request(request)?;
        self.validate_checkpoint()?;
        let incoming = parse_input(request)?;
        let replay = match self.snapshot()? {
            Some(saved) => lineage::assemble(request, saved, incoming, tools)?,
            None => incoming,
        };
        replay::validate(request, replay)?;
        self.state
            .lock()
            .map_err(|_| CheckpointError::Inaccessible("state"))?
            .tools = tools.clone();
        Ok(())
    }

    pub fn commit(&self, request: &Value, output: &[Value]) -> Result<(), CompletionError> {
        self.commit_for_generation(request, output, self.generation()?)
    }

    pub(super) fn commit_for_generation(
        &self,
        request: &Value,
        output: &[Value],
        generation: u64,
    ) -> Result<(), CompletionError> {
        super::memory_policy::validate_request(request)?;
        self.validate_checkpoint()?;
        for item in output {
            super::memory_policy::validate_item(item)?;
        }
        let mut items = parse_input(request)?;
        items.extend(
            output
                .iter()
                .cloned()
                .map(CodexItem::parse)
                .collect::<Result<Vec<_>, _>>()?,
        );
        self.promote_checked(
            Some(CodexCheckpoint {
                version: 1,
                generation,
                protocol: full_protocol(),
                model: request["model"].as_str().map(str::to_owned),
                items,
                compacted_users: Vec::new(),
            }),
            Some(generation),
            None,
        )?;
        Ok(())
    }

    fn validate_checkpoint(&self) -> Result<(), CompletionError> {
        if self.blocked.load(Ordering::Acquire) {
            return Err(CheckpointError::Blocked.into());
        }
        drop(
            self.writer
                .lock()
                .map_err(|_| CheckpointError::Inaccessible("writer"))?,
        );
        if self.snapshot()?.is_some_and(|saved| {
            !saved.compacted_users.is_empty()
                || saved
                    .items
                    .iter()
                    .any(|item| matches!(item, CodexItem::Compaction { .. }))
        }) {
            return Err(super::memory_policy::unsupported("continuation checkpoint"));
        }
        Ok(())
    }
}

fn parse_input(request: &Value) -> Result<Vec<CodexItem>, CompletionError> {
    request["input"]
        .as_array()
        .ok_or_else(|| {
            CompletionError::RequestError("Codex continuation requires an input array".into())
        })?
        .iter()
        .cloned()
        .map(CodexItem::parse)
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}
