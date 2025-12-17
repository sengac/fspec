use anyhow::Result;

pub(super) fn add_assistant_text_message(messages: &mut Vec<rig::message::Message>, text: String) {
    use rig::message::{AssistantContent, Message, Text};
    use rig::OneOrMany;

    messages.push(Message::Assistant {
        id: None,
        content: OneOrMany::one(AssistantContent::Text(Text { text })),
    });
}

pub(super) fn add_assistant_tool_calls_message(
    messages: &mut Vec<rig::message::Message>,
    tool_calls: Vec<rig::message::AssistantContent>,
) -> Result<()> {
    use rig::message::Message;
    use rig::OneOrMany;
    use tracing::error;

    match OneOrMany::many(tool_calls) {
        Ok(content) => {
            messages.push(Message::Assistant { id: None, content });
            Ok(())
        }
        Err(e) => {
            error!("Failed to convert tool calls to message: {:?}", e);
            Err(anyhow::anyhow!("Failed to convert tool calls: {e:?}"))
        }
    }
}
