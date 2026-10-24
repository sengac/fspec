use super::lifecycle::invalid;
use super::sections::Sections;
use super::stream_events::Choice;
use rig::completion::CompletionError;
use serde_json::Value;

pub(super) struct Reasoning {
    completed: bool,
    sections: Sections,
}

impl Reasoning {
    pub fn new(completed: bool) -> Self {
        Self {
            completed,
            sections: Sections::default(),
        }
    }

    fn section(
        &mut self,
        location: (&Value, Option<u64>),
        kind: &'static str,
        index: u64,
        text: &Value,
        delta: bool,
    ) -> Result<Option<Choice>, CompletionError> {
        if !delta && !self.completed {
            return Ok(None);
        }
        Ok(self
            .sections
            .section(location, kind, index, text, delta)?
            .map(|(id, reasoning)| Choice::ReasoningDelta {
                id: Some(id),
                reasoning,
            }))
    }

    pub fn event(&mut self, kind: &str, event: &Value) -> Result<Vec<Choice>, CompletionError> {
        let (text_kind, index_key) = if kind.starts_with("response.reasoning_summary_") {
            ("summary", "summary_index")
        } else {
            ("content", "content_index")
        };
        let index = event[index_key]
            .as_u64()
            .ok_or_else(|| invalid("expected reasoning section index"))?;
        let delta = kind.ends_with(".delta");
        let text = if kind == "response.reasoning_summary_part.done" {
            &event["part"]["text"]
        } else {
            &event[if delta { "delta" } else { "text" }]
        };
        Ok(self
            .section(
                (&event["item_id"], event["output_index"].as_u64()),
                text_kind,
                index,
                text,
                delta,
            )?
            .into_iter()
            .collect())
    }

    pub fn item(
        &mut self,
        item: &Value,
        output: Option<u64>,
    ) -> Result<Vec<Choice>, CompletionError> {
        let mut choices = Vec::new();
        for (key, kind, expected) in [
            ("summary", "summary", "summary_text"),
            ("content", "content", "reasoning_text"),
        ] {
            let Some(parts) = item.get(key) else {
                continue;
            };
            let parts = parts
                .as_array()
                .ok_or_else(|| invalid("expected reasoning sections"))?;
            for (index, part) in parts.iter().enumerate() {
                if part["type"] != expected {
                    continue;
                }
                if let Some(choice) = self.section(
                    (&item["id"], output),
                    kind,
                    index as u64,
                    &part["text"],
                    false,
                )? {
                    choices.push(choice);
                }
            }
        }
        Ok(choices)
    }
}
