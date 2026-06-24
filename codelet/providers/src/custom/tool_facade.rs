//! Rhai-scriptable tool facade types (PROV-066).
//!
//! This module defines [`RhaiToolDef`] — the Rust-side representation of
//! a tool entry returned by a custom provider's optional
//! `define_tools(config)` Rhai function — together with
//! [`RhaiToolFacadeAdapter`], a thin adapter that exposes the tool to
//! the rest of the stack (system prompt rendering, request builders)
//! and delegates parameter mapping to the optional Rhai
//! `map_tool_params(config, tool_name, maps_to, params)` function.
//!
//! The adapter is deliberately *not* a full `rig::Tool` implementation —
//! `rig::Tool` requires a `const NAME: &'static str`, which is
//! incompatible with dynamic tool names defined at runtime by a user
//! script. Downstream code uses `RhaiToolDef` directly when bridging
//! into rig-compatible request builders.

use std::sync::Arc;

use rhai::{Dynamic, Map, Scope};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::config::ProviderConfig;
use super::conversion::{dynamic_to_json_value, json_value_to_dynamic};
use super::error::CustomProviderError;
use super::script_loader::ScriptLoader;
use codelet_tools::facade::InternalFileParams;

/// A tool definition produced by a custom provider's `define_tools`
/// script entry (PROV-066).
///
/// Mirrors the Rhai map shape `#{ name, description, parameters,
/// maps_to }`. `parameters` is kept as `serde_json::Value` so arbitrary
/// JSON Schemas round-trip untouched through the resolver and caching
/// layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RhaiToolDef {
    /// Tool name as surfaced to the LLM provider.
    pub name: String,
    /// Human-readable tool description.
    pub description: String,
    /// JSON schema for the tool's parameters object.
    pub parameters: Value,
    /// Logical target identifier (e.g. `"file:read"`, `"bash"`).
    /// The resolver validates this against the known set before
    /// surfacing the tool to downstream consumers.
    pub maps_to: String,
}

/// Adapter that exposes a single [`RhaiToolDef`] to the rest of the
/// stack.
///
/// Holds `Arc`-shared handles to the script loader and config so it can
/// invoke `map_tool_params` on demand without holding locks across
/// `.await` points. Cloning is cheap — all fields are `Arc`.
#[derive(Clone)]
pub struct RhaiToolFacadeAdapter {
    def: Arc<RhaiToolDef>,
    config: Arc<ProviderConfig>,
    loader: Arc<ScriptLoader>,
}

impl RhaiToolFacadeAdapter {
    /// Build a new adapter. The three inputs are cloned as `Arc` handles
    /// so cloning the adapter is cheap.
    pub fn new(
        def: Arc<RhaiToolDef>,
        config: Arc<ProviderConfig>,
        loader: Arc<ScriptLoader>,
    ) -> Result<Self, CustomProviderError> {
        Ok(Self {
            def,
            config,
            loader,
        })
    }

    /// Name surfaced to the LLM provider — sourced from the
    /// underlying [`RhaiToolDef`] at construction time.
    pub fn name(&self) -> String {
        self.def.name.clone()
    }

    /// JSON schema for the tool parameters.
    pub fn parameters_schema(&self) -> &Value {
        &self.def.parameters
    }

    /// Logical target identifier (`file:read`, `bash`, …).
    pub fn maps_to(&self) -> &str {
        &self.def.maps_to
    }

    /// Underlying tool definition.
    pub fn def(&self) -> &RhaiToolDef {
        &self.def
    }

    /// Shared loader handle.
    pub fn loader(&self) -> &Arc<ScriptLoader> {
        &self.loader
    }

    /// Shared provider config.
    pub fn config(&self) -> &Arc<ProviderConfig> {
        &self.config
    }
}

/// Name of the optional Rhai function invoked to remap tool parameters.
const FN_MAP_TOOL_PARAMS: &str = "map_tool_params";

/// Invoke the custom provider's optional `map_tool_params` Rhai function
/// for the adapter's tool.
///
/// When the script does not define `map_tool_params`, or when the
/// function returns `()` (Rhai unit), the input `params` are returned
/// unchanged so callers can fall back to default field-by-field
/// deserialisation.
///
/// The function signature expected in Rhai is:
/// `fn map_tool_params(config, tool_name, maps_to, params)`
pub fn apply_map_tool_params(
    adapter: &RhaiToolFacadeAdapter,
    params: Value,
) -> Result<Value, CustomProviderError> {
    let script_path = std::path::PathBuf::from(&adapter.config.script);
    let ast = adapter.loader.load(&script_path)?;

    let has_fn = ast
        .iter_functions()
        .any(|meta| meta.name == FN_MAP_TOOL_PARAMS);
    if !has_fn {
        return Ok(params);
    }

    // Build the 4-arg call.
    let config_dyn = {
        let mut map = Map::new();
        map.insert("name".into(), Dynamic::from(adapter.config.name.clone()));
        map.insert(
            "base_url".into(),
            Dynamic::from(adapter.config.base_url.clone()),
        );
        Dynamic::from_map(map)
    };
    let tool_name_dyn = Dynamic::from(adapter.def.name.clone());
    let maps_to_dyn = Dynamic::from(adapter.def.maps_to.clone());
    let params_dyn = json_value_to_dynamic(&params);

    let engine = adapter.loader.engine();
    let mut scope = Scope::new();
    let result = engine.call_fn::<Dynamic>(
        &mut scope,
        &ast,
        FN_MAP_TOOL_PARAMS,
        (config_dyn, tool_name_dyn, maps_to_dyn, params_dyn),
    );

    match result {
        Ok(value) => {
            // Rhai unit == "default mapping" signal.
            if value.is_unit() {
                Ok(params)
            } else {
                Ok(dynamic_to_json_value(&value))
            }
        }
        Err(e) => {
            tracing::warn!(
                provider = %adapter.config.name,
                tool = %adapter.def.name,
                error = %e,
                "map_tool_params raised error; using default mapping"
            );
            Ok(params)
        }
    }
}

/// Default field-by-field mapping from a `maps_to` + params JSON object
/// into the internal file-operation params type (`InternalFileParams`).
///
/// Currently supports the three `file:*` targets. Other targets are
/// rejected with a `RhaiRuntimeError` listing the supported prefixes.
pub fn default_to_internal_file(
    maps_to: &str,
    params: &Value,
) -> Result<InternalFileParams, CustomProviderError> {
    match maps_to {
        "file:read" => {
            #[derive(Deserialize)]
            struct ReadShape {
                file_path: String,
                #[serde(default)]
                offset: Option<usize>,
                #[serde(default)]
                limit: Option<usize>,
                #[serde(default)]
                mode: Option<String>,
            }
            let shape: ReadShape = serde_json::from_value(params.clone()).map_err(|e| {
                CustomProviderError::RhaiRuntimeError(format!(
                    "default file:read mapping failed: {e}"
                ))
            })?;
            Ok(InternalFileParams::Read {
                file_path: shape.file_path,
                offset: shape.offset,
                limit: shape.limit,
                mode: shape.mode,
                indentation: None,
            })
        }
        "file:write" => {
            #[derive(Deserialize)]
            struct WriteShape {
                file_path: String,
                content: String,
            }
            let shape: WriteShape = serde_json::from_value(params.clone()).map_err(|e| {
                CustomProviderError::RhaiRuntimeError(format!(
                    "default file:write mapping failed: {e}"
                ))
            })?;
            Ok(InternalFileParams::Write {
                file_path: shape.file_path,
                content: shape.content,
            })
        }
        "file:edit" => {
            #[derive(Deserialize)]
            struct EditShape {
                file_path: String,
                old_string: String,
                new_string: String,
            }
            let shape: EditShape = serde_json::from_value(params.clone()).map_err(|e| {
                CustomProviderError::RhaiRuntimeError(format!(
                    "default file:edit mapping failed: {e}"
                ))
            })?;
            Ok(InternalFileParams::Edit {
                file_path: shape.file_path,
                old_string: shape.old_string,
                new_string: shape.new_string,
            })
        }
        other => Err(CustomProviderError::RhaiRuntimeError(format!(
            "default_to_internal_file only supports file:read/file:write/file:edit, got '{other}'"
        ))),
    }
}
