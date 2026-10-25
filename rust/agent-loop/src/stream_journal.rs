use codelet_cli::interactive::StreamEvent;
use codelet_core::persistence::transcript::{
    JournalEvent, JournalIdentity, JournalRecord, RequestAnchor, TerminalOutcome,
};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub(crate) struct StreamJournal {
    identity: JournalIdentity,
    sequence: u64,
    blocked: bool,
    terminal: Option<TerminalOutcome>,
    failure_diagnostic_published: bool,
    requested: HashSet<String>,
    started: HashSet<String>,
    result_ids: HashMap<String, Uuid>,
    pending: Option<crate::persistence_retry::RecoveryRecord>,
    synthetic_results: HashSet<String>,
}

impl StreamJournal {
    pub(crate) fn new(session_id: Uuid) -> Self {
        Self {
            identity: JournalIdentity {
                session_id,
                request_id: Uuid::new_v4(),
                attempt_id: Uuid::new_v4(),
                message_id: Uuid::new_v4(),
            },
            sequence: 0,
            blocked: false,
            terminal: None,
            failure_diagnostic_published: false,
            requested: HashSet::new(),
            started: HashSet::new(),
            result_ids: HashMap::new(),
            pending: None,
            synthetic_results: HashSet::new(),
        }
    }

    pub(crate) fn admit_cancellation(&mut self, id: &str) -> Result<(), String> {
        if self.blocked || !self.requested.contains(id) {
            return Err("tool interruption lacks an acknowledged invocation".into());
        }
        let message_id = *self
            .result_ids
            .entry(id.into())
            .or_insert_with(Uuid::new_v4);
        let event = if self.started.contains(id) {
            JournalEvent::ToolOutcomeUnknown { id: id.into() }
        } else {
            JournalEvent::ToolCancelled { id: id.into() }
        };
        self.append(event, Some(message_id))?;
        self.synthetic_results.insert(id.into());
        Ok(())
    }

    fn append(&mut self, event: JournalEvent, message_id: Option<Uuid>) -> Result<(), String> {
        if self.blocked {
            return Err("transcript recovery is required before publication or dispatch".into());
        }
        let mut identity = self.identity.clone();
        if let Some(message_id) = message_id {
            identity.message_id = message_id;
        }
        let mut record = JournalRecord::new(identity, self.sequence, event);
        if self.sequence == 0 {
            let manifest = codelet_core::persistence::load_session(self.identity.session_id)?;
            record.request_anchor = Some(RequestAnchor {
                original_count: manifest.messages.len(),
                preceding_message_id: manifest
                    .messages
                    .last()
                    .map(|reference| reference.message_id),
            });
        }
        self.sequence = self
            .sequence
            .checked_add(1)
            .ok_or("transcript sequence exhausted")?;
        let encoded = serde_json::to_value(&record).map_err(|_| "transcript encoding failed")?;
        let pending = crate::persistence_retry::RecoveryRecord {
            version: 1,
            id: record.id,
            role: "transcript".into(),
            content: String::new(),
            metadata: HashMap::from([("_transcript".into(), encoded)]),
        };
        let result = crate::persistence_retry::submit(self.identity.session_id, pending.clone());
        if result.is_err() {
            self.blocked = true;
            self.pending = Some(pending);
        }
        result
    }

    pub(crate) fn admit(
        &mut self,
        event: &StreamEvent,
        reason: Option<codelet_rpc_types::InterruptionReason>,
    ) -> Result<bool, String> {
        if self.blocked {
            if !matches!(
                event,
                StreamEvent::Done(_) | StreamEvent::Error(_) | StreamEvent::Interrupted(_)
            ) {
                return Err(
                    "transcript recovery is required before publication or dispatch".into(),
                );
            }
            crate::persist::retry_pending_persistence(&self.identity.session_id)?;
            if let Some(pending) = &self.pending {
                crate::persistence_retry::submit(self.identity.session_id, pending.clone())?;
                self.pending = None;
            }
            self.blocked = false;
        }
        let event = match event {
            StreamEvent::ResponseAttemptStarted => {
                self.identity.attempt_id = Uuid::new_v4();
                JournalEvent::AttemptStarted
            }
            StreamEvent::ResponseAttemptAborted => JournalEvent::AttemptAborted,
            StreamEvent::Text(text) => JournalEvent::Text { text: text.clone() },
            StreamEvent::Thinking(thinking) => JournalEvent::Thinking {
                thinking: thinking.clone(),
            },
            StreamEvent::ToolCall(call) => {
                self.requested.insert(call.id.clone());
                JournalEvent::ToolRequested {
                    id: call.id.clone(),
                    call_id: call.call_id.clone(),
                    name: call.name.clone(),
                    input: call.args.clone(),
                }
            }
            StreamEvent::ToolResult(result) => {
                if !self.requested.contains(&result.id) {
                    return Err("tool result lacks an acknowledged invocation".into());
                }
                if self.synthetic_results.remove(&result.id) {
                    return Ok(true);
                }
                let message_id = *self
                    .result_ids
                    .entry(result.id.clone())
                    .or_insert_with(Uuid::new_v4);
                self.append(
                    JournalEvent::ToolCompleted {
                        id: result.id.clone(),
                        content: result.content.clone(),
                        is_error: result.is_error,
                    },
                    Some(message_id),
                )?;
                return Ok(true);
            }
            StreamEvent::Error(_) => return self.terminal(TerminalOutcome::ProviderError),
            StreamEvent::Interrupted(_) => {
                let outcome = match reason {
                    Some(codelet_rpc_types::InterruptionReason::User) => {
                        TerminalOutcome::UserCancelled
                    }
                    Some(codelet_rpc_types::InterruptionReason::LoopDetector) => {
                        TerminalOutcome::LoopInterrupted
                    }
                    None => TerminalOutcome::Interrupted,
                };
                return self.terminal(outcome);
            }
            StreamEvent::CompactionFailed { .. } => {
                return self.terminal(TerminalOutcome::CompactionFailed)
            }
            StreamEvent::Done(_) => {
                if self.terminal.is_none() {
                    self.terminal(TerminalOutcome::Completed)?;
                }
                return Ok(true);
            }
            _ => return Ok(true),
        };
        self.append(event, None)?;
        Ok(true)
    }

    fn terminal(&mut self, outcome: TerminalOutcome) -> Result<bool, String> {
        if self.terminal.is_some() {
            let publish = self.terminal == Some(TerminalOutcome::CompactionFailed)
                && outcome == TerminalOutcome::ProviderError
                && !self.failure_diagnostic_published;
            self.failure_diagnostic_published |= publish;
            return Ok(publish);
        }
        self.append(JournalEvent::Terminal { outcome }, None)?;
        self.terminal = Some(outcome);
        Ok(true)
    }

    pub(crate) fn prepare_dispatch(&mut self, id: &str) -> Result<(), String> {
        if self.blocked || !self.requested.contains(id) || self.started.contains(id) {
            return Err("tool dispatch lacks an acknowledged unique invocation".into());
        }
        self.append(JournalEvent::ToolStarted { id: id.into() }, None)?;
        self.started.insert(id.into());
        Ok(())
    }

    pub(crate) fn message_id(&self) -> Uuid {
        self.identity.message_id
    }

    pub(crate) fn message_promoted(&mut self) {
        self.identity.message_id = Uuid::new_v4();
    }

    pub(crate) fn result_id(&self, id: &str) -> Result<Uuid, String> {
        self.result_ids
            .get(id)
            .copied()
            .ok_or_else(|| "tool result identity unavailable".into())
    }

    pub(crate) fn capture_result_id(&mut self, id: &str) -> Result<Uuid, String> {
        if !self.requested.contains(id) {
            return Err("tool result lacks an acknowledged invocation".into());
        }
        Ok(*self
            .result_ids
            .entry(id.into())
            .or_insert_with(Uuid::new_v4))
    }
}
