use rig::completion::{CompletionError, CompletionModel, CompletionRequest, CompletionResponse};
use rig::streaming::StreamingCompletionResponse;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

type OpenAi = rig::providers::openai::completion::CompletionModel;

#[derive(Clone)]
pub struct InterruptingModel {
    pub inner: OpenAi,
    pub interrupted: Arc<AtomicBool>,
    pub during_admission: bool,
}

impl CompletionModel for InterruptingModel {
    type Response = <OpenAi as CompletionModel>::Response;
    type StreamingResponse = <OpenAi as CompletionModel>::StreamingResponse;
    type Client = OpenAi;

    fn make(client: &Self::Client, _model: impl Into<String>) -> Self {
        Self {
            inner: client.clone(),
            interrupted: Arc::new(AtomicBool::new(false)),
            during_admission: false,
        }
    }

    fn prepare_request(&self, request: CompletionRequest) -> Result<Vec<u8>, CompletionError> {
        self.inner.prepare_request(request)
    }

    fn prepare_request_for_generation(
        &self,
        request: CompletionRequest,
        generation: u64,
    ) -> Result<Vec<u8>, CompletionError> {
        let prepared = self
            .inner
            .prepare_request_for_generation(request, generation)?;
        if self.during_admission && generation > 0 {
            self.interrupted.store(true, Ordering::Release);
        }
        Ok(prepared)
    }

    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse<Self::Response>, CompletionError> {
        self.inner.completion(request).await
    }

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError> {
        self.inner.stream(request).await
    }
}
