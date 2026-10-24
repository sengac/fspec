use super::ResponsesCompletionModel;
use crate::completion::{CompletionError, CompletionRequest};
use crate::http_client::HttpClientExt;

impl<T> ResponsesCompletionModel<T>
where
    T: HttpClientExt + Clone + Default + std::fmt::Debug + 'static,
{
    pub(super) fn streaming_request_bytes(
        &self,
        request: CompletionRequest,
    ) -> Result<Vec<u8>, CompletionError> {
        let mut request = self.create_completion_request(request)?;
        request.stream = Some(true);
        Ok(self
            .client
            .http_client()
            .prepare_request_body(serde_json::to_vec(&request)?)?)
    }
}
