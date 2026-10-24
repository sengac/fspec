use std::sync::Arc;

use crate::{
    completion::ToolDefinition,
    wasm_compat::{WasmBoxedFuture, WasmCompatSend, WasmCompatSync},
};

use super::{ToolDyn, ToolError, ToolSet, ToolSetError, ToolType};

pub trait ToolMiddleware: WasmCompatSend + WasmCompatSync {
    fn scope_id(&self) -> &str;

    fn policy_id(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn decorate_definition(&self, definition: ToolDefinition) -> Result<ToolDefinition, ToolError>;

    fn call<'a>(
        &'a self,
        tool: &'a dyn ToolDyn,
        args: String,
    ) -> WasmBoxedFuture<'a, Result<String, ToolError>> {
        tool.call(args)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("tool middleware execution policy cannot be rebound from {current} to {requested}")]
struct PolicyConflict {
    current: &'static str,
    requested: &'static str,
}

#[derive(Debug, thiserror::Error)]
#[error("tool middleware scope cannot be rebound from {current:?} to {requested:?}")]
struct ScopeConflict {
    current: String,
    requested: String,
}

impl ToolSet {
    pub async fn configure_middleware(
        &mut self,
        middleware: Arc<dyn ToolMiddleware>,
    ) -> Result<(), ToolSetError> {
        if let Some(current) = &self.middleware {
            if current.scope_id() != middleware.scope_id() {
                return Err(ToolError::ToolCallError(Box::new(ScopeConflict {
                    current: current.scope_id().into(),
                    requested: middleware.scope_id().into(),
                }))
                .into());
            }
            if current.policy_id() != middleware.policy_id() {
                return Err(ToolError::ToolCallError(Box::new(PolicyConflict {
                    current: current.policy_id(),
                    requested: middleware.policy_id(),
                }))
                .into());
            }
            return Ok(());
        }
        for tool in self.tools.values() {
            middleware.decorate_definition(tool.definition(String::new()).await)?;
        }
        self.middleware = Some(middleware);
        Ok(())
    }

    pub async fn get_tool_definition(
        &self,
        name: &str,
        prompt: String,
    ) -> Result<ToolDefinition, ToolSetError> {
        let tool = self
            .get(name)
            .ok_or_else(|| ToolSetError::ToolNotFoundError(name.into()))?;
        self.decorate_definition(tool.definition(prompt).await)
    }

    pub(crate) fn decorate_definition(
        &self,
        definition: ToolDefinition,
    ) -> Result<ToolDefinition, ToolSetError> {
        match &self.middleware {
            Some(middleware) => Ok(middleware.decorate_definition(definition)?),
            None => Ok(definition),
        }
    }

    pub(crate) async fn validate_addition(&self, tool: &dyn ToolDyn) -> Result<(), ToolSetError> {
        self.decorate_definition(tool.definition(String::new()).await)?;
        Ok(())
    }

    pub(crate) async fn validate_append(&self, tools: &ToolSet) -> Result<(), ToolSetError> {
        if let Some(source) = &tools.middleware {
            let destination = self.middleware.as_ref().map(|policy| policy.scope_id());
            if destination != Some(source.scope_id()) {
                return Err(ToolError::ToolCallError(Box::new(ScopeConflict {
                    current: destination.unwrap_or("unbound").into(),
                    requested: source.scope_id().into(),
                }))
                .into());
            }
        }
        for tool in tools.tools.values() {
            self.decorate_definition(tool.definition(String::new()).await)?;
        }
        Ok(())
    }

    pub(crate) async fn call_with_middleware(
        &self,
        tool: &ToolType,
        args: String,
    ) -> Result<String, ToolError> {
        let tool: &dyn ToolDyn = match tool {
            ToolType::Simple(tool) => tool.as_ref(),
            ToolType::Embedding(tool) => tool.as_ref(),
        };
        match &self.middleware {
            Some(middleware) => middleware.call(tool, args).await,
            None => tool.call(args).await,
        }
    }
}
