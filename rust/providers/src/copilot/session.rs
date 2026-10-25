use crate::session::{ExtensionError, ProviderSession, ProviderSessionExtension, StateWriter};
use rig::{
    completion::{CompletionError, CompletionRequest},
    message::{AssistantContent, Message},
    OneOrMany,
};
use serde_json::Value;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct CopilotSession {
    state: Arc<Mutex<State>>,
    owner: Option<uuid::Uuid>,
}

#[derive(Default)]
struct State {
    epoch: u64,
    sequence: u64,
    committed: Option<Lineage>,
}

struct Lineage {
    binding: String,
    context: Value,
    history: Vec<Value>,
    opaque: Value,
}

pub(crate) struct Attempt {
    session: CopilotSession,
    epoch: u64,
    sequence: u64,
    context: Value,
    history: Vec<Value>,
    previous: Option<Lineage>,
    pub capture: super::capture::Capture,
    settled: bool,
}

fn unavailable() -> CompletionError {
    CompletionError::ResponseError("Copilot live session state unavailable".into())
}

pub(crate) fn bind_live_session(session: &mut ProviderSession, owner: uuid::Uuid) {
    if session
        .extension::<CopilotSession>("github-copilot")
        .is_some_and(|extension| extension.owner != Some(owner))
    {
        session.register(
            "github-copilot",
            Arc::new(CopilotSession {
                owner: Some(owner),
                ..CopilotSession::default()
            }),
        );
    }
}

impl CopilotSession {
    pub(crate) fn start(&self, request: &CompletionRequest) -> Result<Attempt, CompletionError> {
        let context =
            serde_json::to_value((&request.preamble, &request.documents, &request.tools))?;
        let history = request
            .chat_history
            .iter()
            .map(serde_json::to_value)
            .collect::<Result<Vec<_>, _>>()?;
        let mut state = self.state.lock().map_err(|_| unavailable())?;
        state.sequence = state.sequence.checked_add(1).ok_or_else(unavailable)?;
        let previous = state.committed.take();
        let (epoch, sequence) = (state.epoch, state.sequence);
        let shared = self.state.clone();
        let authorized = Arc::new(move || {
            shared
                .lock()
                .is_ok_and(|state| state.epoch == epoch && state.sequence == sequence)
        });
        Ok(Attempt {
            session: self.clone(),
            epoch: state.epoch,
            sequence: state.sequence,
            context,
            history,
            previous,
            capture: super::capture::Capture::new(authorized),
            settled: false,
        })
    }
}

impl Attempt {
    pub(crate) fn bind(&mut self, binding: String, model: &str, deployment: &str) {
        let replay = self
            .previous
            .take()
            .filter(|lineage| {
                lineage.binding == binding
                    && lineage.context == self.context
                    && self.history.starts_with(&lineage.history)
            })
            .map(|lineage| lineage.opaque);
        self.capture.configure(replay, binding, model, deployment);
    }

    pub(crate) fn settle(&mut self, content: Vec<AssistantContent>) -> Result<(), CompletionError> {
        let content = OneOrMany::many(content).map_err(|_| unavailable())?;
        let message = serde_json::to_value(Message::Assistant { id: None, content })?;
        let mut state = self.session.state.lock().map_err(|_| unavailable())?;
        if state.epoch == self.epoch && state.sequence == self.sequence {
            let mut history = self.history.clone();
            history.push(message);
            state.committed = self.capture.candidate()?.map(|(binding, opaque)| Lineage {
                binding,
                context: self.context.clone(),
                history,
                opaque,
            });
        }
        self.settled = true;
        Ok(())
    }
}

impl Drop for Attempt {
    fn drop(&mut self) {
        if !self.settled {
            if let Ok(mut state) = self.session.state.lock() {
                if state.epoch == self.epoch && state.sequence == self.sequence {
                    state.committed = None;
                }
            }
        }
    }
}

impl ProviderSessionExtension for CopilotSession {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn restore(&self, _envelopes: &[String]) -> Result<(), ExtensionError> {
        self.reset()
    }
    fn set_writer(&self, _writer: StateWriter) -> Result<(), ExtensionError> {
        Ok(())
    }
    fn begin_turn(&self) {
        if let Ok(mut state) = self.state.lock() {
            if let Some(epoch) = state.epoch.checked_add(1) {
                state.epoch = epoch;
            } else {
                state.committed = None;
            }
        }
    }
    fn reset(&self) -> Result<(), ExtensionError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ExtensionError::Persistence("Copilot session lock poisoned".into()))?;
        state.committed = None;
        state.epoch = state.epoch.checked_add(1).ok_or_else(|| {
            ExtensionError::Persistence("Copilot session generation exhausted".into())
        })?;
        Ok(())
    }
    fn prepare_generation(
        &self,
        _generation: u64,
        _model: &str,
    ) -> Result<std::collections::HashMap<String, Value>, ExtensionError> {
        self.reset()?;
        Ok(std::collections::HashMap::new())
    }
    fn install_committed_generation(
        &self,
        _generation: u64,
        _metadata: &std::collections::HashMap<String, Value>,
    ) -> Result<(), ExtensionError> {
        self.reset()
    }
}
