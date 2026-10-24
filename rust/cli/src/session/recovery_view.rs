use super::Session;
use crate::compaction_dag::{
    detect_existing_dag, COMPACTION_INSTRUCTION_FRESH, COMPACTION_INSTRUCTION_INCREMENTAL,
    DAG_NODE_SYNTAX, DAG_RANGE_RULES,
};
use crate::interactive_helpers::validate_no_orphan_tool_calls;
use rig::message::Message;

#[derive(Debug)]
pub(crate) struct CompactionWorkingView {
    messages: Vec<Message>,
    previous_inference_block: Option<String>,
}

impl Session {
    pub fn prepare_compaction_view(&mut self, prompt: Option<&str>) -> Result<(), String> {
        validate_no_orphan_tool_calls(&self.messages).map_err(|orphans| {
            format!(
                "compaction refuses orphan tool calls: {}",
                orphans.join(", ")
            )
        })?;
        let instruction = match detect_existing_dag(&self.messages) {
            Some((content, end)) => COMPACTION_INSTRUCTION_INCREMENTAL
                .replace("{existing_dag_content}", &content)
                .replace("{last_compacted_turn}", &end.saturating_add(1).to_string()),
            None => COMPACTION_INSTRUCTION_FRESH.to_owned(),
        };
        let instruction = format!("{instruction}\n\n{DAG_NODE_SYNTAX}\n\n{DAG_RANGE_RULES}");
        let instruction = match prompt {
            Some(prompt) => format!("{instruction}\n\nAfter building the DAG and calling inject_summary, resume working on:\n{prompt}"),
            None => instruction,
        };
        let (mut messages, _) = super::system_reminders::partition_for_compaction(&self.messages);
        messages.push(Message::user(instruction));
        let previous_inference_block = self.compaction_working_messages.as_ref().map_or_else(
            || self.compaction_inference_block.clone(),
            |view| view.previous_inference_block.clone(),
        );
        self.compaction_working_messages = Some(CompactionWorkingView {
            messages,
            previous_inference_block,
        });
        self.block_compaction_inference("DAG construction awaits durable application".into());
        Ok(())
    }

    pub fn inference_messages(&mut self) -> &mut Vec<Message> {
        self.compaction_working_messages
            .as_mut()
            .map(|view| &mut view.messages)
            .unwrap_or(&mut self.messages)
    }

    pub fn has_compaction_working_view(&self) -> bool {
        self.compaction_working_messages.is_some()
    }

    pub fn discard_compaction_working_view(&mut self) {
        if let Some(view) = self.compaction_working_messages.take() {
            if self.pending_compaction_commit.is_none() {
                self.compaction_inference_block = view.previous_inference_block;
            }
        }
    }
}
