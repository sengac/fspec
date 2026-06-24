//! `RhaiToolWrapper` — adapt a [`RhaiToolFacadeAdapter`] (PROV-066) to
//! rig's [`Tool`] trait so a Rhai-defined tool can be plugged into a
//! `rig::agent::Agent` (PROV-092).
//!
//! The wrapper carries an `Arc`-shared adapter, the session id, and the
//! provider config so it can:
//!
//! 1. Surface the Rhai-defined `name`, `description`, and JSON-schema
//!    parameters via [`Tool::definition`] / [`Tool::name`].
//! 2. On [`Tool::call`], pass raw provider-supplied params through the
//!    optional `map_tool_params` hook (via
//!    [`super::tool_facade::apply_map_tool_params`]).
//! 3. Translate the resulting `serde_json::Value` plus the
//!    adapter's `maps_to` into a typed
//!    [`super::tool_dispatch::DispatchedToolParams`] using
//!    [`super::tool_dispatch::default_to_internal`].
//! 4. Execute the dispatched params via
//!    [`super::internal_dispatch::execute_dispatched`].
//!
//! Tool errors thrown by the script (`map_tool_params` failures,
//! unknown `maps_to`, malformed params) surface as
//! `ToolError::ToolCallError` so rig's tool driver records them in the
//! conversation history exactly like a native tool error.

use std::sync::Arc;

use rig::completion::ToolDefinition as RigToolDefinition;
use rig::tool::{Tool, ToolError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::internal_dispatch::execute_dispatched;
use super::tool_dispatch::default_to_internal;
use super::tool_facade::{apply_map_tool_params, RhaiToolFacadeAdapter};

/// Args wrapper — the underlying rig agent passes the model-supplied
/// JSON object verbatim. We keep it as a raw `serde_json::Value` so
/// callers can match the dynamic `parameters` schema declared by the
/// script.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RhaiToolArgs(pub Value);

/// rig::tool::Tool implementation for a Rhai-defined custom tool.
///
/// Cloning is cheap — every field is either `Arc` or `Copy`.
#[derive(Clone)]
pub struct RhaiToolWrapper {
    adapter: RhaiToolFacadeAdapter,
    session_id: Uuid,
    /// Cached `&'static str` view of the dynamic tool name. The Rhai
    /// script defines tool names at runtime but [`Tool::NAME`] is a
    /// `const`, so we keep a sentinel here and override [`Tool::name`].
    /// The sentinel is informational only — it never reaches the LLM.
    sentinel: &'static str,
}

impl RhaiToolWrapper {
    /// Construct a new wrapper. Cloning is cheap.
    pub fn new(adapter: RhaiToolFacadeAdapter, session_id: Uuid) -> Self {
        Self {
            adapter,
            session_id,
            sentinel: "rhai_custom_tool",
        }
    }

    /// Borrow the wrapped adapter — useful for tests.
    pub fn adapter(&self) -> &RhaiToolFacadeAdapter {
        &self.adapter
    }

    /// Session id this wrapper was constructed with.
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }
}

impl Tool for RhaiToolWrapper {
    const NAME: &'static str = "rhai_custom_tool";

    type Error = ToolError;
    type Args = RhaiToolArgs;
    type Output = String;

    /// Override to surface the script-defined tool name.
    fn name(&self) -> String {
        self.adapter.name()
    }

    async fn definition(&self, _prompt: String) -> RigToolDefinition {
        RigToolDefinition {
            name: self.adapter.name(),
            description: self.adapter.def().description.clone(),
            parameters: self.adapter.parameters_schema().clone(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // 1. apply_map_tool_params — script-defined params remap.
        let mapped = apply_map_tool_params(&self.adapter, args.0).map_err(rhai_err_to_tool_err)?;

        // 2. default_to_internal — typed dispatch on `maps_to`.
        let dispatched =
            default_to_internal(self.adapter.maps_to(), &mapped).map_err(rhai_err_to_tool_err)?;

        // 3. execute_dispatched — run the inner internal tool.
        let output = Arc::new(self.clone());
        let session_id = output.session_id;
        let result = execute_dispatched(session_id, dispatched)
            .await
            .map_err(rhai_err_to_tool_err)?;

        // sentinel is intentionally unused at runtime — keep the field
        // alive without `#[allow(dead_code)]` clippy noise.
        let _ = output.sentinel;
        Ok(result)
    }
}

fn rhai_err_to_tool_err(e: super::error::CustomProviderError) -> ToolError {
    ToolError::ToolCallError(Box::new(std::io::Error::other(e.to_string())))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::custom::tool_facade::RhaiToolDef;
    use crate::custom::ProviderConfig;
    use crate::oauth::building_blocks::register_all_modules;
    use crate::oauth::engine::build_sandboxed_engine;
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::Arc;

    fn make_wrapper(maps_to: &str, name: &str) -> RhaiToolWrapper {
        let def = RhaiToolDef {
            name: name.to_string(),
            description: "test tool".to_string(),
            parameters: json!({"type":"object"}),
            maps_to: maps_to.to_string(),
        };
        let cfg_json = json!({
            "name": "test-provider",
            "display_name": "Test",
            "base_url": "http://example.com",
            "script": "",
            "models": {"default": {"id": "x"}}
        });
        let cfg: ProviderConfig = serde_json::from_value(cfg_json).expect("ProviderConfig parse");
        let _ = HashMap::<String, String>::new();
        let engine = build_sandboxed_engine(register_all_modules());
        let loader = Arc::new(crate::custom::ScriptLoader::new(engine));
        let adapter =
            RhaiToolFacadeAdapter::new(Arc::new(def), Arc::new(cfg), loader).expect("adapter");
        RhaiToolWrapper::new(adapter, Uuid::new_v4())
    }

    #[test]
    fn name_returns_rhai_defined_tool_name() {
        let wrapper = make_wrapper("file:read", "my_dynamic_name");
        let n = <RhaiToolWrapper as Tool>::name(&wrapper);
        assert_eq!(n, "my_dynamic_name");
    }
}
