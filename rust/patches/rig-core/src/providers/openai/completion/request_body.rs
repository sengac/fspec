use super::{CompletionModel, CompletionRequest, OpenAIRequestParams};
use crate::completion::{CompletionError, CompletionRequest as CoreCompletionRequest};
use crate::http_client::HttpClientExt;
use crate::json_utils::merge;
use serde_json::{Value, json};

impl<T: HttpClientExt> CompletionModel<T> {
    pub(super) fn selected_request_json(
        &self,
        request: CoreCompletionRequest,
    ) -> Result<Value, CompletionError> {
        let request = CompletionRequest::try_from(OpenAIRequestParams {
            model: self.model.clone(),
            request,
            strict_tools: self.strict_tools,
            tool_result_array_content: self.tool_result_array_content,
        })?;
        let mut body = serde_json::to_value(request)?;
        body = merge(body, json!({"stream": self.stream}));
        if self.stream {
            body = merge(body, json!({"stream_options": {"include_usage": true}}));
        } else if let Some(body) = body.as_object_mut() {
            body.remove("stream_options");
        }
        Ok(body)
    }

    pub(super) fn selected_request_bytes(&self, body: &Value) -> Result<Vec<u8>, CompletionError> {
        Ok(self
            .client
            .http_client()
            .prepare_request_body(serde_json::to_vec(body)?)?)
    }
}
