use super::lifecycle::invalid;
use rig::completion::CompletionError;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum Identity {
    Item(String),
    Output(u64),
}

#[derive(Default)]
pub(super) struct Sections {
    items: HashMap<String, Identity>,
    outputs: HashMap<u64, Identity>,
    delivered: HashMap<(Identity, &'static str, u64), String>,
}

impl Sections {
    fn identity(&mut self, item: &Value, output: Option<u64>) -> Result<Identity, CompletionError> {
        let item = item.as_str().filter(|item| !item.is_empty());
        let identity = if let Some(item) = item {
            self.items.get(item).cloned().unwrap_or_else(|| {
                output
                    .and_then(|output| self.outputs.get(&output))
                    .filter(|identity| matches!(identity, Identity::Output(_)))
                    .cloned()
                    .unwrap_or_else(|| Identity::Item(item.to_owned()))
            })
        } else if let Some(output) = output {
            self.outputs
                .get(&output)
                .cloned()
                .unwrap_or(Identity::Output(output))
        } else {
            return Err(invalid("missing output item identity"));
        };
        if let Some(item) = item {
            self.items.insert(item.to_owned(), identity.clone());
        }
        if let Some(output) = output {
            self.outputs.insert(output, identity.clone());
        }
        Ok(identity)
    }

    pub fn section(
        &mut self,
        location: (&Value, Option<u64>),
        kind: &'static str,
        index: u64,
        text: &Value,
        delta: bool,
    ) -> Result<Option<(String, String)>, CompletionError> {
        let identity = self.identity(location.0, location.1)?;
        let text = text
            .as_str()
            .ok_or_else(|| invalid("expected readable section text"))?;
        let delivered = self
            .delivered
            .entry((identity.clone(), kind, index))
            .or_default();
        let unseen = if delta {
            text
        } else {
            text.strip_prefix(delivered.as_str()).unwrap_or_default()
        };
        if unseen.is_empty() {
            return Ok(None);
        }
        let unseen = unseen.to_owned();
        delivered.push_str(&unseen);
        Ok(Some((format!("{identity:?}:{kind}:{index}"), unseen)))
    }
}
