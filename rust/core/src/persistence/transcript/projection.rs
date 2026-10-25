use super::{JournalError, JournalEvent, JournalRecord};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolState {
    Requested,
    Started,
    Completed,
    Cancelled,
    NotStarted,
    OutcomeUnknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecoveredTool {
    pub request_id: Uuid,
    pub attempt_id: Uuid,
    pub id: String,
    pub call_id: Option<String>,
    pub name: String,
    pub state: ToolState,
    pub result: Option<String>,
    pub is_error: bool,
}

pub struct JournalProjection {
    pub records: Vec<JournalRecord>,
    pub tools: Vec<RecoveredTool>,
}

impl JournalProjection {
    pub(crate) fn from_records(
        records: Vec<JournalRecord>,
        originals: &HashSet<Uuid>,
    ) -> Result<Self, JournalError> {
        let aborted: HashSet<_> = records
            .iter()
            .filter_map(|record| {
                matches!(record.event, JournalEvent::AttemptAborted)
                    .then_some((record.identity.request_id, record.identity.attempt_id))
            })
            .collect();
        let mut projection = Self {
            records: Vec::new(),
            tools: Vec::new(),
        };
        let mut calls = HashMap::new();
        let mut lifecycle = super::lifecycle::Lifecycle::default();
        for record in records {
            let transition = lifecycle.check(&record)?;
            lifecycle.commit(transition);
            let identity = &record.identity;
            match &record.event {
                JournalEvent::ToolRequested {
                    id, call_id, name, ..
                } => {
                    let key = (identity.request_id, identity.attempt_id, id.clone());
                    calls.insert(key, projection.tools.len());
                    projection.tools.push(RecoveredTool {
                        request_id: identity.request_id,
                        attempt_id: identity.attempt_id,
                        id: id.clone(),
                        call_id: call_id.clone(),
                        name: name.clone(),
                        state: ToolState::NotStarted,
                        result: None,
                        is_error: false,
                    });
                }
                JournalEvent::ToolStarted { id }
                | JournalEvent::ToolOutcomeUnknown { id }
                | JournalEvent::ToolCompleted { id, .. }
                | JournalEvent::ToolCancelled { id } => {
                    let index = calls
                        .get(&(identity.request_id, identity.attempt_id, id.clone()))
                        .ok_or(JournalError::Lifecycle)?;
                    let tool = &mut projection.tools[*index];
                    match &record.event {
                        JournalEvent::ToolStarted { .. } => {
                            tool.state = ToolState::OutcomeUnknown;
                        }
                        JournalEvent::ToolOutcomeUnknown { .. } => {}
                        JournalEvent::ToolCompleted {
                            content, is_error, ..
                        } => {
                            tool.state = ToolState::Completed;
                            tool.result = Some(content.clone());
                            tool.is_error = *is_error;
                        }
                        JournalEvent::ToolCancelled { .. } => {
                            tool.state = ToolState::Cancelled;
                        }
                        _ => return Err(JournalError::Lifecycle),
                    }
                }
                _ => {}
            }
            let hidden_attempt = aborted.contains(&(identity.request_id, identity.attempt_id))
                && matches!(
                    record.event,
                    JournalEvent::Text { .. } | JournalEvent::Thinking { .. }
                );
            let original = originals.contains(&identity.message_id)
                && !matches!(
                    record.event,
                    JournalEvent::Terminal { .. }
                        | JournalEvent::ToolOutcomeUnknown { .. }
                        | JournalEvent::ToolCancelled { .. }
                );
            if !hidden_attempt
                && !original
                && !matches!(
                    record.event,
                    JournalEvent::AttemptStarted | JournalEvent::AttemptAborted
                )
            {
                projection.records.push(record);
            }
        }
        Ok(projection)
    }
}
