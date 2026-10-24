use super::completion::gemini_api_types::{Content, PartKind};
use super::completion::{CompletionModel, create_request_body};
use crate::completion::{CompletionError, CompletionRequest};
use crate::http_client::HttpClientExt;
use crate::message::{AssistantContent, Message, UserContent};
use std::collections::HashMap;

pub(super) fn history_contents(history: Vec<Message>) -> Result<Vec<Content>, CompletionError> {
    let mut functions = HashMap::new();
    history
        .into_iter()
        .map(|message| {
            if let Message::Assistant { content, .. } = &message {
                for item in content.iter() {
                    if let AssistantContent::ToolCall(call) = item {
                        functions.insert(call.id.clone(), call.function.name.clone());
                        if let Some(call_id) = &call.call_id {
                            functions.insert(call_id.clone(), call.function.name.clone());
                        }
                    }
                }
            }
            let names = match &message {
                Message::User { content } => content
                    .iter()
                    .map(|item| match item {
                        UserContent::ToolResult(result) => functions
                            .get(&result.id)
                            .or_else(|| result.call_id.as_ref().and_then(|id| functions.get(id)))
                            .cloned(),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
                _ => Vec::new(),
            };
            let mut content = Content::try_from(message)
                .map_err(|error| CompletionError::RequestError(Box::new(error)))?;
            for (part, name) in content.parts.iter_mut().zip(names) {
                if let (PartKind::FunctionResponse(response), Some(name)) = (&mut part.part, name) {
                    response.name = name;
                }
            }
            Ok(content)
        })
        .collect()
}

impl<T: HttpClientExt> CompletionModel<T> {
    pub(super) fn selected_request_bytes(
        &self,
        request: CompletionRequest,
    ) -> Result<Vec<u8>, CompletionError> {
        Ok(self
            .client
            .http_client()
            .prepare_request_body(serde_json::to_vec(&create_request_body(request)?)?)?)
    }
}
