use super::{session::CopilotSession, CopilotEndpoint, CopilotEndpointFacade, CopilotHttpClient};
use rig::client::CompletionClient;
use rig::completion::{CompletionError, CompletionModel, CompletionRequest, CompletionResponse};
use rig::providers::openai;
use rig::streaming::StreamingCompletionResponse;
use serde::{Deserialize, Serialize};

#[path = "completion_stream.rs"]
mod streaming;
#[path = "completion_terminal.rs"]
mod terminal;
pub use streaming::CopilotStreamResponse;

#[derive(Clone)]
pub struct CopilotClient {
    chat: openai::CompletionsClient<CopilotHttpClient>,
    session: CopilotSession,
    deployment: String,
}

impl CopilotClient {
    pub(crate) fn new(
        chat: openai::CompletionsClient<CopilotHttpClient>,
        deployment: String,
    ) -> Self {
        Self {
            chat,
            session: CopilotSession::default(),
            deployment,
        }
    }

    pub(crate) fn bind_session(&mut self, session: CopilotSession) {
        self.session = session;
    }
}

impl CompletionClient for CopilotClient {
    type CompletionModel = CopilotCompletionModel;
}

#[derive(Serialize, Deserialize)]
pub enum CopilotCompletionResponse {
    Chat(openai::completion::CompletionResponse),
    Responses(Box<openai::responses_api::CompletionResponse>),
}

#[derive(Clone)]
pub struct CopilotCompletionModel {
    client: CopilotClient,
    model: String,
}

enum RoutedModel {
    Chat(openai::completion::CompletionModel<CopilotHttpClient>),
    Responses(openai::responses_api::ResponsesCompletionModel<CopilotHttpClient>),
}

fn sanitize(mut request: CompletionRequest) -> CompletionRequest {
    if let Some(params) = request
        .additional_params
        .as_mut()
        .and_then(serde_json::Value::as_object_mut)
    {
        params.remove("reasoning_opaque");
    }
    request
}

impl CopilotCompletionModel {
    fn routed(
        &self,
        capture: Option<super::capture::Capture>,
    ) -> Result<RoutedModel, CompletionError> {
        let original = &self.client.chat;
        let http = original.http_client().clone().with_capture(capture);
        let client = openai::CompletionsClient::<CopilotHttpClient>::builder()
            .api_key(original.http_client().access_token())
            .base_url(original.base_url())
            .http_headers(original.headers().clone())
            .http_client(http)
            .build()?;
        Ok(match CopilotEndpointFacade::select(&self.model) {
            CopilotEndpoint::ChatCompletions => {
                RoutedModel::Chat(client.completion_model(&self.model))
            }
            CopilotEndpoint::Responses => {
                RoutedModel::Responses(client.responses_api().completion_model(&self.model))
            }
        })
    }

    async fn start(
        &self,
        request: &CompletionRequest,
    ) -> Result<super::session::Attempt, CompletionError> {
        let mut attempt = self.client.session.start(request)?;
        let binding = self
            .client
            .chat
            .http_client()
            .live_binding(&self.model, &self.client.deployment)
            .await?;
        attempt.bind(binding, &self.model, &self.client.deployment);
        Ok(attempt)
    }
}

impl CompletionModel for CopilotCompletionModel {
    type Client = CopilotClient;
    type Response = CopilotCompletionResponse;
    type StreamingResponse = CopilotStreamResponse;

    fn make(client: &Self::Client, model: impl Into<String>) -> Self {
        Self {
            client: client.clone(),
            model: model.into(),
        }
    }

    fn prepare_request(&self, request: CompletionRequest) -> Result<Vec<u8>, CompletionError> {
        let request = sanitize(request);
        match self.routed(None)? {
            RoutedModel::Chat(model) => model.prepare_request(request),
            RoutedModel::Responses(model) => model.prepare_request(request),
        }
    }

    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse<Self::Response>, CompletionError> {
        let request = sanitize(request);
        let mut attempt = self.start(&request).await?;
        let response = match self.routed(Some(attempt.capture.clone()))? {
            RoutedModel::Chat(model) => {
                let response = model.completion(request).await?;
                CompletionResponse {
                    choice: response.choice,
                    usage: response.usage,
                    raw_response: CopilotCompletionResponse::Chat(response.raw_response),
                }
            }
            RoutedModel::Responses(model) => {
                let response = terminal::validate(model.completion(request).await?)?;
                CompletionResponse {
                    choice: response.choice,
                    usage: response.usage,
                    raw_response: CopilotCompletionResponse::Responses(Box::new(
                        response.raw_response,
                    )),
                }
            }
        };
        attempt.settle(response.choice.iter().cloned().collect())?;
        Ok(response)
    }

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError> {
        let request = sanitize(request);
        let attempt = self.start(&request).await?;
        match self.routed(Some(attempt.capture.clone()))? {
            RoutedModel::Chat(model) => Ok(streaming::adapt(model.stream(request).await?, attempt)),
            RoutedModel::Responses(model) => {
                Ok(streaming::adapt(model.stream(request).await?, attempt))
            }
        }
    }
}
