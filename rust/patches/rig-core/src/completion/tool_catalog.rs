use super::{CompletionError, CompletionRequest};

impl CompletionRequest {
    pub(crate) fn validate_tool_catalog_overrides(&self) -> Result<(), CompletionError> {
        if !self.tools.is_empty()
            && let Some(options) = self
                .additional_params
                .as_ref()
                .and_then(serde_json::Value::as_object)
        {
            for key in ["tools", "functions", "mcp_servers"] {
                if options.contains_key(key) {
                    return Err(CompletionError::RequestError(
                        format!("additional_params.{key} cannot replace or bypass the configured tool catalog; register executable tools through the tool server").into(),
                    ));
                }
            }
        }
        Ok(())
    }
}
