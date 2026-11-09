//! BUG-284: an admitted submission installed in authoritative runtime history
//! before provider construction. The stream handoff detaches exactly that
//! installed message for the first request (which appends the prompt itself)
//! and reinstalls the same message, so runtime never loses or duplicates it.

use super::Session;
use rig::message::Message;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct AdmittedInput {
    pub id: Uuid,
    pub index: usize,
    pub message: Message,
}

impl Session {
    /// Install the exact admitted original at the end of runtime history.
    pub fn install_admitted_input(&mut self, id: Uuid, message: Message) {
        self.messages.push(message.clone());
        self.admitted_input = Some(AdmittedInput {
            id,
            index: self.messages.len().saturating_sub(1),
            message,
        });
    }

    /// Whether the admitted input is still awaiting its first provider request.
    pub fn admitted_input_pending(&self) -> Option<Uuid> {
        self.admitted_input.as_ref().map(|admitted| admitted.id)
    }

    /// Release the admission marker without changing runtime history.
    pub fn clear_admitted_input(&mut self) -> Option<Uuid> {
        self.admitted_input.take().map(|admitted| admitted.id)
    }

    /// Detach the installed admitted message for the first request. When
    /// pre-prompt compaction already consumed it, it stays in history.
    pub fn detach_admitted_input(&mut self, consumed_by_compaction: bool) -> Option<Message> {
        let admitted = self.admitted_input.take()?;
        if consumed_by_compaction {
            return None;
        }
        let installed = self.messages.len() == admitted.index.saturating_add(1)
            && self.messages.last() == Some(&admitted.message);
        if !installed {
            tracing::warn!(submission = %admitted.id, "admitted input is no longer the runtime tail; request keeps current history");
            return None;
        }
        self.messages.pop()
    }
}

/// Durable writer for internal continuation prompts that replace an admitted
/// input consumed by pre-prompt compaction (registered by the task loop).
pub type ContinuationPersister = std::sync::Arc<dyn Fn(&str) -> Result<(), String> + Send + Sync>;

static PERSISTERS: std::sync::LazyLock<
    std::sync::RwLock<std::collections::HashMap<Uuid, ContinuationPersister>>,
> = std::sync::LazyLock::new(Default::default);

pub fn set_continuation_persister(session_id: Uuid, persister: Option<ContinuationPersister>) {
    let mut registry = PERSISTERS
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    match persister {
        Some(persister) => registry.insert(session_id, persister),
        None => registry.remove(&session_id),
    };
}

/// Make the continuation that follows a consumed admitted input durable
/// before it is sent, so cold replay keeps the same conversation.
pub(crate) fn persist_continuation(session_id: Uuid, text: &str) -> Result<(), String> {
    let persister = PERSISTERS
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(&session_id)
        .cloned()
        .ok_or("continuation persistence is unavailable for the admitted input")?;
    persister(text)
}
