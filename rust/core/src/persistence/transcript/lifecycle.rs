use super::{JournalError, JournalEvent, JournalRecord, ToolState};
use std::collections::HashMap;
use uuid::Uuid;

type ToolKey = (Uuid, Uuid, String);
pub(super) type Transition = Option<(ToolKey, ToolState)>;

#[derive(Default)]
pub(super) struct Lifecycle {
    tools: HashMap<ToolKey, ToolState>,
}

impl Lifecycle {
    pub(super) fn check(&self, record: &JournalRecord) -> Result<Transition, JournalError> {
        crate::persistence::messages::durability::perform("transcript.lifecycle.admit", || Ok(()))?;
        if record.request_anchor.is_some() && record.sequence != 0 {
            return Err(JournalError::Lifecycle);
        }
        let id = match &record.event {
            JournalEvent::ToolRequested { id, .. }
            | JournalEvent::ToolStarted { id }
            | JournalEvent::ToolOutcomeUnknown { id }
            | JournalEvent::ToolCompleted { id, .. }
            | JournalEvent::ToolCancelled { id } => id,
            _ => return Ok(None),
        };
        let key = (
            record.identity.request_id,
            record.identity.attempt_id,
            id.clone(),
        );
        let previous = self.tools.get(&key).copied();
        let state = match (&record.event, previous) {
            (JournalEvent::ToolRequested { .. }, None) => ToolState::NotStarted,
            (JournalEvent::ToolStarted { .. }, Some(ToolState::NotStarted))
            | (JournalEvent::ToolOutcomeUnknown { .. }, Some(ToolState::OutcomeUnknown)) => {
                ToolState::OutcomeUnknown
            }
            (
                JournalEvent::ToolCompleted { .. },
                Some(ToolState::NotStarted | ToolState::OutcomeUnknown),
            ) => ToolState::Completed,
            (JournalEvent::ToolCancelled { .. }, Some(ToolState::NotStarted)) => {
                ToolState::Cancelled
            }
            _ => return Err(JournalError::Lifecycle),
        };
        Ok(Some((key, state)))
    }

    pub(super) fn commit(&mut self, transition: Transition) {
        if let Some((key, state)) = transition {
            self.tools.insert(key, state);
        }
    }
}
