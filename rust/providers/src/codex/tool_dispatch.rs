use super::super::lifecycle::invalid;
use rig::completion::CompletionError;
use rig::streaming::RawStreamingToolCall;
use serde_json::Value;
use std::collections::HashSet;

#[derive(Default)]
pub(super) struct ToolDispatch {
    items: HashSet<String>,
    calls: HashSet<String>,
}

impl ToolDispatch {
    pub fn accept(
        &mut self,
        item: &Value,
    ) -> Result<Option<RawStreamingToolCall>, CompletionError> {
        let field = |key: &str| {
            item[key]
                .as_str()
                .filter(|value| !value.is_empty())
                .ok_or_else(|| invalid(format!("missing tool {key}")))
        };
        let id = field("id")?;
        let call_id = field("call_id")?;
        if self.items.contains(id) || self.calls.contains(call_id) {
            return Ok(None);
        }
        if item
            .get("status")
            .is_some_and(|status| status != "completed")
        {
            return Err(invalid("tool item is not completed"));
        }
        let name = field("name")?;
        let arguments: Value = serde_json::from_str(field("arguments")?)
            .map_err(|_| invalid("invalid tool argument JSON"))?;
        if !arguments.is_object() {
            return Err(invalid("tool arguments must be an object"));
        }
        self.items.insert(id.to_owned());
        self.calls.insert(call_id.to_owned());
        Ok(Some(
            RawStreamingToolCall::new(id.to_owned(), name.to_owned(), arguments)
                .with_call_id(call_id.to_owned()),
        ))
    }
}
