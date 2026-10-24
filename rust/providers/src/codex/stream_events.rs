use super::answers::Answers;
use super::lifecycle::invalid;
use super::reasoning::Reasoning;
use rig::completion::CompletionError;
use rig::providers::openai::responses_api::streaming::StreamingCompletionResponse;
use rig::streaming::{RawStreamingChoice, RawStreamingToolCall, ToolCallDeltaContent};
use serde_json::Value;

#[path = "tool_dispatch.rs"]
mod tool_dispatch;
use tool_dispatch::ToolDispatch;

pub(super) type Choice = RawStreamingChoice<StreamingCompletionResponse>;

pub(super) struct OutputState {
    pub tools: Vec<RawStreamingToolCall>,
    pub saw_tool: bool,
    dispatch: ToolDispatch,
    reasoning: Reasoning,
    answers: Answers,
    completed: Vec<(Option<u64>, Value)>,
    custom_tools: crate::codex::custom_tools::CustomToolRegistry,
}

fn merge_item(items: &mut Vec<(Option<u64>, Value)>, index: Option<u64>, item: &Value) {
    let existing = items.iter_mut().find(|(position, known)| {
        item.is_object()
            && match (known["id"].as_str(), item["id"].as_str()) {
                (Some(known), Some(incoming)) => known == incoming,
                _ => index.is_some() && *position == index,
            }
    });
    if let Some((position, known)) = existing {
        if index.is_some() {
            *position = index;
        }
        if let (Some(known), Some(incoming)) = (known.as_object_mut(), item.as_object()) {
            known.extend(incoming.clone());
        }
    } else {
        items.push((index, item.clone()));
    }
}

fn string(value: &Value, key: &str) -> Result<String, CompletionError> {
    value[key]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| invalid(format!("missing {key}")))
}

impl OutputState {
    pub fn new(
        completed_reasoning: bool,
        custom_tools: crate::codex::custom_tools::CustomToolRegistry,
    ) -> Self {
        Self {
            tools: Vec::new(),
            saw_tool: false,
            dispatch: ToolDispatch::default(),
            reasoning: Reasoning::new(completed_reasoning),
            answers: Answers::default(),
            completed: Vec::new(),
            custom_tools,
        }
    }

    pub fn merge_terminal(&self, output: &mut Vec<Value>) {
        let mut merged = Vec::new();
        for (index, item) in output.iter().enumerate() {
            merge_item(&mut merged, Some(index as u64), item);
        }
        for (index, item) in &self.completed {
            merge_item(&mut merged, *index, item);
        }
        merged.sort_by_key(|(index, _)| index.unwrap_or(u64::MAX));
        *output = merged.into_iter().map(|(_, item)| item).collect();
    }

    pub fn completed_item(
        &mut self,
        item: &Value,
        output: Option<u64>,
    ) -> Result<Vec<Choice>, CompletionError> {
        match item["type"].as_str() {
            Some("reasoning") => self.reasoning.item(item, output),
            Some("message") => self.answers.item(item, output),
            _ => Ok(Vec::new()),
        }
    }

    pub fn tool(&mut self, item: &Value) -> Result<(), CompletionError> {
        self.saw_tool = true;
        let item = self
            .custom_tools
            .normalize_call(item)
            .map_err(|error| invalid(error.to_string()))?;
        if let Some(tool) = self.dispatch.accept(&item)? {
            self.tools.push(tool);
        }
        Ok(())
    }

    pub fn event(&mut self, kind: &str, event: &Value) -> Result<Vec<Choice>, CompletionError> {
        if kind == "response.output_item.done" {
            merge_item(
                &mut self.completed,
                event["output_index"].as_u64(),
                &event["item"],
            );
        }
        let choice = match kind {
            "response.output_text.delta" | "response.refusal.delta" => {
                return self.answers.event(kind, event)
            }
            "response.reasoning_text.delta"
            | "response.reasoning_text.done"
            | "response.reasoning_summary_text.delta"
            | "response.reasoning_summary_text.done"
            | "response.reasoning_summary_part.done" => return self.reasoning.event(kind, event),
            "response.function_call_arguments.delta" | "response.custom_tool_call_input.delta" => {
                self.saw_tool = true;
                Some(Choice::ToolCallDelta {
                    id: string(event, "item_id")?,
                    content: ToolCallDeltaContent::Delta(string(event, "delta")?),
                })
            }
            "response.output_item.added"
                if matches!(
                    event["item"]["type"].as_str(),
                    Some("function_call" | "custom_tool_call")
                ) =>
            {
                self.saw_tool = true;
                Some(Choice::ToolCallDelta {
                    id: string(&event["item"], "id")?,
                    content: ToolCallDeltaContent::Name(string(&event["item"], "name")?),
                })
            }
            "response.output_item.done"
                if matches!(
                    event["item"]["type"].as_str(),
                    Some("function_call" | "custom_tool_call")
                ) =>
            {
                self.saw_tool = true;
                self.tool(&event["item"])?;
                None
            }
            "response.output_item.done" => {
                return self.completed_item(&event["item"], event["output_index"].as_u64())
            }
            _ => None,
        };
        Ok(choice.into_iter().collect())
    }
}
