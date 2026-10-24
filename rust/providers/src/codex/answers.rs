use super::lifecycle::invalid;
use super::sections::Sections;
use super::stream_events::Choice;
use rig::completion::CompletionError;
use serde_json::Value;

#[derive(Default)]
pub(super) struct Answers {
    sections: Sections,
}

impl Answers {
    pub fn event(&mut self, kind: &str, event: &Value) -> Result<Vec<Choice>, CompletionError> {
        let kind = if kind.starts_with("response.refusal.") {
            "refusal"
        } else {
            "text"
        };
        let index = event["content_index"].as_u64().unwrap_or(0);
        Ok(self
            .sections
            .section(
                (&event["item_id"], event["output_index"].as_u64()),
                kind,
                index,
                &event["delta"],
                true,
            )?
            .into_iter()
            .map(|(_, text)| Choice::Message(text))
            .collect())
    }

    pub fn item(
        &mut self,
        item: &Value,
        output: Option<u64>,
    ) -> Result<Vec<Choice>, CompletionError> {
        let content = item["content"]
            .as_array()
            .ok_or_else(|| invalid("expected answer sections"))?;
        let mut choices = Vec::new();
        for (index, part) in content.iter().enumerate() {
            let (kind, text) = match part["type"].as_str() {
                Some("output_text") => ("text", &part["text"]),
                Some("refusal") => ("refusal", &part["refusal"]),
                _ => continue,
            };
            if let Some((_, text)) =
                self.sections
                    .section((&item["id"], output), kind, index as u64, text, false)?
            {
                choices.push(Choice::Message(text));
            }
        }
        Ok(choices)
    }
}
