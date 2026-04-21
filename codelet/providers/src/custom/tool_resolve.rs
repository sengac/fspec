//! Resolve the per-provider tool list (PROV-066).
//!
//! [`resolve_tools`] invokes the optional Rhai `define_tools(config)`
//! function, validates the returned list against the known `maps_to`
//! set, and caches the result in
//! `ProviderConfig.resolved_tools`. On any runtime / shape error it
//! falls back to [`super::tool_presets::preset_tools`] keyed by
//! `tool_style`.

use rhai::{Dynamic, Map, Scope};

use super::config::ProviderConfig;
use super::conversion::dynamic_to_json_value;
use super::error::CustomProviderError;
use super::script_loader::ScriptLoader;
use super::tool_facade::RhaiToolDef;
use super::tool_presets::preset_tools;

/// Known `maps_to` identifiers. The resolver rejects anything outside
/// this set so typos surface as hard errors rather than silently
/// resolving at tool-dispatch time.
pub(crate) const KNOWN_MAPS_TO: &[&str] = &[
    "file:read",
    "file:write",
    "file:edit",
    "bash",
    "search:grep",
    "search:glob",
    "search:ast_grep",
    "ls",
    "web_search:search",
    "fspec",
    "bridge",
    "exec:run",
    "hitl",
];

/// Name of the optional Rhai function enumerating tool definitions.
const FN_DEFINE_TOOLS: &str = "define_tools";

/// Resolve the tool list for `config`.
///
/// Resolution order:
/// 1. Invoke `define_tools(config)` if the script defines it. Validate
///    each returned entry and surface validation errors verbatim.
/// 2. If `define_tools` is absent, return the preset matching
///    `config.tool_style`.
/// 3. If `define_tools` raises a runtime error, log a warning and fall
///    back to the preset (no error surfaces to the caller).
///
/// On success, populates `config.resolved_tools` so subsequent lookups
/// can reuse the cached list.
pub fn resolve_tools(
    config: &mut ProviderConfig,
    loader: &ScriptLoader,
) -> Result<Vec<RhaiToolDef>, CustomProviderError> {
    let script_path = std::path::PathBuf::from(&config.script);
    let ast = loader.load(&script_path)?;

    let has_fn = ast.iter_functions().any(|meta| meta.name == FN_DEFINE_TOOLS);
    if !has_fn {
        let tools = preset_tools(config.tool_style);
        config.resolved_tools = Some(tools.clone());
        return Ok(tools);
    }

    // Build the single-arg `config` Rhai map mirroring provider.rs.
    let config_dyn = {
        let mut map = Map::new();
        map.insert("name".into(), Dynamic::from(config.name.clone()));
        map.insert("base_url".into(), Dynamic::from(config.base_url.clone()));
        Dynamic::from_map(map)
    };

    let engine = loader.engine();
    let mut scope = Scope::new();
    let result =
        engine.call_fn::<Dynamic>(&mut scope, &ast, FN_DEFINE_TOOLS, (config_dyn,));

    let dyn_result = match result {
        Ok(value) => value,
        Err(e) => {
            tracing::warn!(
                provider = %config.name,
                error = %e,
                "define_tools raised runtime error; falling back to preset"
            );
            let tools = preset_tools(config.tool_style);
            config.resolved_tools = Some(tools.clone());
            return Ok(tools);
        }
    };

    let tools = parse_define_tools_result(dyn_result)?;
    validate_maps_to(&tools)?;
    config.resolved_tools = Some(tools.clone());
    Ok(tools)
}

/// Convert the raw `Dynamic` returned by `define_tools` into a vector
/// of [`RhaiToolDef`].
fn parse_define_tools_result(value: Dynamic) -> Result<Vec<RhaiToolDef>, CustomProviderError> {
    if !value.is_array() {
        return Err(CustomProviderError::RhaiRuntimeError(
            "define_tools must return an array of tool maps".to_string(),
        ));
    }
    let entries = value.into_typed_array::<Dynamic>().map_err(|typ| {
        CustomProviderError::RhaiRuntimeError(format!(
            "define_tools returned a non-array ({typ})"
        ))
    })?;

    let mut out: Vec<RhaiToolDef> = Vec::with_capacity(entries.len());
    for (idx, entry) in entries.into_iter().enumerate() {
        let map = entry.try_cast::<Map>().ok_or_else(|| {
            CustomProviderError::RhaiRuntimeError(format!(
                "define_tools[{idx}] must be a map with name, description, parameters, maps_to"
            ))
        })?;
        out.push(tool_def_from_map(idx, map)?);
    }
    Ok(out)
}

/// Extract a single tool definition from one Rhai map entry.
fn tool_def_from_map(idx: usize, map: Map) -> Result<RhaiToolDef, CustomProviderError> {
    let name = map
        .get("name")
        .cloned()
        .and_then(|v| v.into_string().ok())
        .ok_or_else(|| missing_field(idx, "name"))?;
    let description = map
        .get("description")
        .cloned()
        .and_then(|v| v.into_string().ok())
        .ok_or_else(|| missing_field(idx, "description"))?;
    let parameters = map
        .get("parameters")
        .cloned()
        .map(|d| dynamic_to_json_value(&d))
        .unwrap_or(serde_json::Value::Object(Default::default()));
    let maps_to = map
        .get("maps_to")
        .cloned()
        .and_then(|v| v.into_string().ok())
        .ok_or_else(|| missing_field(idx, "maps_to"))?;

    Ok(RhaiToolDef {
        name,
        description,
        parameters,
        maps_to,
    })
}

fn missing_field(idx: usize, field: &str) -> CustomProviderError {
    CustomProviderError::RhaiRuntimeError(format!(
        "define_tools[{idx}] missing required field '{field}'"
    ))
}

/// Reject tools whose `maps_to` is not in [`KNOWN_MAPS_TO`].
fn validate_maps_to(tools: &[RhaiToolDef]) -> Result<(), CustomProviderError> {
    for tool in tools {
        if !KNOWN_MAPS_TO.contains(&tool.maps_to.as_str()) {
            let valid = KNOWN_MAPS_TO.join(", ");
            return Err(CustomProviderError::RhaiRuntimeError(format!(
                "unknown maps_to '{}' for tool '{}'; valid identifiers: {valid}",
                tool.maps_to, tool.name
            )));
        }
    }
    Ok(())
}
