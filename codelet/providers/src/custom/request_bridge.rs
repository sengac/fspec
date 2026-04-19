//! Request bridge: `&[Message]` + `&[ToolDefinition]` → Rhai `Dynamic`
//! (PROV-063).
//!
//! The custom-provider Rhai contract consumes a single argument named
//! `request`, a map with two keys:
//!
//! ```rhai
//! #{
//!     messages: [ … ],  // array of message maps
//!     tools:    [ … ],  // array of tool-definition maps
//! }
//! ```
//!
//! This module exposes `messages_to_rhai` for direct message-array
//! conversion (used by unit tests) plus `request_to_rhai` for the full
//! request-map build.
//!
//! Serialisation uses serde_json as an intermediate: each
//! `codelet_common::Message` / `codelet_tools::ToolDefinition` is first
//! serialised to `serde_json::Value` via its `Serialize` impl and then
//! bridged into `rhai::Dynamic` by [`super::conversion::json_value_to_dynamic`].

use codelet_common::Message;
use codelet_tools::ToolDefinition;
use rhai::{Array, Dynamic, Map};

use super::conversion::json_value_to_dynamic;
use super::error::CustomProviderError;

/// Convert a slice of `Message`s into a Rhai `Array` (one entry per
/// message). Each entry is a `Map` mirroring the Message's JSON
/// serialisation.
///
/// The returned `Array` is wrapped in `Dynamic` via `Dynamic::from_array`
/// by callers when they need a `Dynamic`. The tests additionally call
/// `messages_to_rhai(&messages)` directly and then unwrap the array, so
/// the outer `Dynamic` layer is provided here.
pub fn messages_to_rhai(messages: &[Message]) -> Result<Dynamic, CustomProviderError> {
    let json = serde_json::to_value(messages).map_err(|e| {
        CustomProviderError::RhaiRuntimeError(format!("serialize messages: {e}"))
    })?;
    Ok(json_value_to_dynamic(&json))
}

/// Convert a slice of tool definitions into a Rhai `Dynamic` (an `Array`
/// of `Map`s).
pub fn tools_to_rhai(tools: &[ToolDefinition]) -> Result<Dynamic, CustomProviderError> {
    let json = serde_json::to_value(tools)
        .map_err(|e| CustomProviderError::RhaiRuntimeError(format!("serialize tools: {e}")))?;
    Ok(json_value_to_dynamic(&json))
}

/// Build the single `request` map passed to `build_request(request)`.
/// Shape: `#{ messages: […], tools: […] }`.
pub fn request_to_rhai(
    messages: &[Message],
    tools: &[ToolDefinition],
) -> Result<Dynamic, CustomProviderError> {
    let messages_dyn = messages_to_rhai(messages)?;
    let tools_dyn = tools_to_rhai(tools)?;

    // Guarantee the messages/tools sides are actually arrays before
    // inserting them into the outer map — this matches the contract the
    // scripts expect and makes failures surface early.
    let messages_arr = messages_dyn
        .into_typed_array::<Dynamic>()
        .map_err(|typ| {
            CustomProviderError::RhaiRuntimeError(format!(
                "messages bridge produced non-array ({typ})"
            ))
        })?;
    let tools_arr: Array = tools_dyn.into_typed_array::<Dynamic>().map_err(|typ| {
        CustomProviderError::RhaiRuntimeError(format!("tools bridge produced non-array ({typ})"))
    })?;

    let mut map = Map::new();
    map.insert("messages".into(), Dynamic::from_array(messages_arr));
    map.insert("tools".into(), Dynamic::from_array(tools_arr));
    Ok(Dynamic::from_map(map))
}
