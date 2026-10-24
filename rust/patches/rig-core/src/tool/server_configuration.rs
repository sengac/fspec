use super::{
    ToolError, ToolMiddleware, ToolServerError, ToolServerHandle, ToolServerRequest,
    ToolServerRequestMessageKind, ToolServerResponse, ToolSetError,
};
use std::sync::Arc;

impl ToolServerHandle {
    pub async fn configure_middleware(
        &self,
        middleware: Arc<dyn ToolMiddleware>,
    ) -> Result<(), ToolServerError> {
        let (sender, receiver) = futures::channel::oneshot::channel();
        self.0
            .send(ToolServerRequest {
                callback_channel: sender,
                data: ToolServerRequestMessageKind::ConfigureMiddleware(middleware),
            })
            .await?;
        match receiver.await? {
            ToolServerResponse::MiddlewareConfigured => Ok(()),
            response => Err(ToolServerError::from_response(response)),
        }
    }
}

impl ToolServerError {
    pub(super) fn from_response(response: ToolServerResponse) -> Self {
        match response {
            ToolServerResponse::ToolError { error } => Self::ToolsetError(
                ToolSetError::ToolCallError(ToolError::ToolCallError(error.into())),
            ),
            response => Self::InvalidMessage(response),
        }
    }
}
