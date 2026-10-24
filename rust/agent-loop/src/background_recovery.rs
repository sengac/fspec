use super::BackgroundOutput;
use codelet_cli::interactive::StreamEvent;
use codelet_core::persistence::AssistantContent;

impl BackgroundOutput {
    pub(super) fn retain_unpublished_original(&self, event: &StreamEvent) {
        match event {
            StreamEvent::Text(text) => {
                self.add_assistant_content(AssistantContent::Text { text: text.clone() });
            }
            StreamEvent::Thinking(thinking) => {
                self.add_assistant_content(AssistantContent::Thinking {
                    thinking: thinking.clone(),
                    signature: None,
                });
            }
            StreamEvent::ToolResult(result) => {
                self.persist_assistant_message();
                let persistence = self
                    .journal
                    .lock()
                    .map_err(|_| "transcript owner unavailable".to_owned())
                    .and_then(|mut journal| journal.capture_result_id(&result.id))
                    .and_then(|id| {
                        crate::persist::persist_tool_result_identified(
                            &self.session.id,
                            id,
                            &result.id,
                            result.call_id.as_deref(),
                            &result.content,
                            result.is_error,
                        )
                    });
                if let Err(error) = persistence {
                    self.report_persistence_failure(&error);
                }
            }
            StreamEvent::Done(stop_reason) => {
                self.persist_assistant_message_with_stop_reason(stop_reason.clone());
            }
            StreamEvent::Error(_) => self.persist_assistant_message(),
            StreamEvent::Interrupted(_) => {
                self.persist_assistant_outcome(None, self.session.interruption_reason());
            }
            _ => {}
        }
    }
}
