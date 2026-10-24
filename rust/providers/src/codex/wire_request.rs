use super::{capability_lock_error, CodexResponsesModel};
use crate::codex::{
    memory_policy,
    request_options::{resolve, RequestOptionsError},
};
use rig::completion::{CompletionError, CompletionRequest};
use rig::providers::openai::responses_api;
use serde_json::Value;

impl CodexResponsesModel {
    pub(super) fn wire_request(
        &self,
        mut request: CompletionRequest,
        stream: bool,
    ) -> Result<Value, CompletionError> {
        if self.strict_attribution {
            for tool in &request.tools {
                crate::tool_contract::validate_parameters(&tool.parameters)
                    .map_err(|error| CompletionError::RequestError(Box::new(error)))?;
            }
        }
        if let Some(configuration) = &request.additional_params {
            memory_policy::validate_request(configuration)?;
        }
        let capabilities = self
            .capabilities
            .read()
            .map_err(|_| CompletionError::RequestError(Box::new(capability_lock_error())))?;
        let options = resolve(request.additional_params.as_ref(), capabilities.as_ref())
            .map_err(|error| CompletionError::RequestError(Box::new(error)))?;
        drop(capabilities);
        request.additional_params = None;
        let mut wire = responses_api::CompletionRequest::try_from((self.model.clone(), request))?;
        wire.stream = stream.then_some(true);
        let mut body = serde_json::to_value(wire)?;
        let expected_tools = if self.strict_attribution {
            let mut tools = body["tools"].clone();
            if tools.is_array() {
                self.custom_tools
                    .register(&mut tools)
                    .map_err(|error| CompletionError::RequestError(Box::new(error)))?;
            }
            Some(tools)
        } else {
            None
        };
        let object = body.as_object_mut().ok_or_else(|| {
            CompletionError::RequestError(Box::new(RequestOptionsError::Invalid {
                option: "request".into(),
                reason: "expected a JSON object".into(),
            }))
        })?;
        if let Value::Object(options) = options {
            object.extend(options);
        }
        self.history
            .prepare_with_tools(&mut body, &self.custom_tools)?;
        if !self.strict_attribution {
            self.native_capabilities.apply(&mut body);
        }
        if body["tools"].is_array() {
            self.custom_tools
                .register(&mut body["tools"])
                .map_err(|error| CompletionError::RequestError(Box::new(error)))?;
        }
        if expected_tools
            .as_ref()
            .is_some_and(|expected| expected != &body["tools"])
        {
            return Err(CompletionError::ProviderError(
                "strict attribution rejects a modified executable tool catalog".into(),
            ));
        }
        Ok(body)
    }
}
