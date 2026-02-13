//! Bridge operation facades for different LLM providers.
//!
//! These facades adapt the BridgeTool interface for provider-specific
//! tool naming and parameter schemas.
//!
//! Feature: spec/features/bridge-tool-unit.feature

use super::traits::ToolDefinition;
use crate::ToolError;
use serde_json::{json, Value};

/// Internal parameters for bridge operations.
/// All provider-specific parameters are mapped to these internal types.
#[derive(Debug, Clone, PartialEq)]
pub enum InternalBridgeParams {
    /// Connect to a WebSocket endpoint
    Connect { url: String },
    /// Disconnect from a WebSocket endpoint
    Disconnect { url: String },
    /// List all active bridge connections
    List,
}

/// Provider-specific tool facade trait for bridge operations.
///
/// Each facade adapts the bridge tool's interface for a specific LLM provider,
/// handling differences in tool naming, parameter schemas, and parameter formats.
pub trait BridgeToolFacade: Send + Sync {
    /// Returns the provider this facade is for (e.g., "claude", "gemini", "openai")
    fn provider(&self) -> &'static str;

    /// Returns the tool name as the provider expects it
    fn tool_name(&self) -> &'static str;

    /// Returns the tool definition with provider-specific schema
    fn definition(&self) -> ToolDefinition;

    /// Maps provider-specific parameters to internal parameters
    fn map_params(&self, input: Value) -> Result<InternalBridgeParams, ToolError>;
}

/// Type alias for a boxed BridgeToolFacade
pub type BoxedBridgeToolFacade = std::sync::Arc<dyn BridgeToolFacade>;

// ============================================================================
// Shared Schema Generators
// ============================================================================

/// Description for bridge tools (shared across providers)
const BRIDGE_DESCRIPTION_NESTED: &str = concat!(
    "Connect to external WebSocket endpoints to relay session output and receive remote input. ",
    "Use action 'connect' to establish connection to a WebSocket URL, ",
    "'disconnect' to close a connection, ",
    "'list' to show all active bridges with their status."
);

const BRIDGE_DESCRIPTION_FLAT: &str = concat!(
    "Manage WebSocket bridge connections for relaying session output to external endpoints. ",
    "Use action_type='connect' with url to establish connection, ",
    "'disconnect' with url to close, ",
    "'list' to show all active bridges."
);

/// Generate nested action schema (used by Claude, OpenAI)
/// Schema: `{ "action": { "type": "connect"|"disconnect"|"list", "url": "..." } }`
fn nested_action_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "oneOf": [
                    {
                        "type": "object",
                        "properties": {
                            "type": { "const": "connect" },
                            "url": {
                                "type": "string",
                                "description": "WebSocket URL to connect to (e.g., ws://localhost:8080)"
                            }
                        },
                        "required": ["type", "url"]
                    },
                    {
                        "type": "object",
                        "properties": {
                            "type": { "const": "disconnect" },
                            "url": {
                                "type": "string",
                                "description": "WebSocket URL to disconnect from"
                            }
                        },
                        "required": ["type", "url"]
                    },
                    {
                        "type": "object",
                        "properties": {
                            "type": { "const": "list" }
                        },
                        "required": ["type"]
                    }
                ]
            }
        },
        "required": ["action"]
    })
}

/// Generate flat action schema (used by Gemini, Z.AI)
/// Schema: `{ "action_type": "connect"|..., "url": "..." }`
fn flat_action_schema(action_field: &str, url_field: &str, url_description: &str) -> Value {
    json!({
        "type": "object",
        "properties": {
            action_field: {
                "type": "string",
                "enum": ["connect", "disconnect", "list"],
                "description": "The bridge action to perform"
            },
            url_field: {
                "type": "string",
                "description": url_description
            }
        },
        "required": [action_field],
        "additionalProperties": false
    })
}

// ============================================================================
// Shared Parameter Parsers
// ============================================================================

/// Parse action type and extract URL for actions that require it
fn parse_action_with_url(
    action_type: &str,
    url: Option<&str>,
    tool_name: &'static str,
    url_field_name: &str,
) -> Result<InternalBridgeParams, ToolError> {
    match action_type {
        "connect" => {
            let url = url
                .ok_or_else(|| ToolError::Validation {
                    tool: tool_name,
                    message: format!("Missing '{url_field_name}' field for connect action"),
                })?
                .to_string();
            Ok(InternalBridgeParams::Connect { url })
        }
        "disconnect" => {
            let url = url
                .ok_or_else(|| ToolError::Validation {
                    tool: tool_name,
                    message: format!("Missing '{url_field_name}' field for disconnect action"),
                })?
                .to_string();
            Ok(InternalBridgeParams::Disconnect { url })
        }
        "list" => Ok(InternalBridgeParams::List),
        _ => Err(ToolError::Validation {
            tool: tool_name,
            message: format!("Unknown action type: {action_type}"),
        }),
    }
}

/// Parse nested action schema used by Claude and OpenAI.
///
/// Handles both cases:
/// 1. `action` is already a JSON object: `{"action": {"type": "connect", "url": "..."}}`
/// 2. `action` is a JSON string that needs parsing: `{"action": "{\"type\": \"connect\", ...}"}`
fn parse_nested_action_schema(input: &Value, tool_name: &'static str) -> Result<InternalBridgeParams, ToolError> {
    let action_value = input
        .get("action")
        .ok_or_else(|| ToolError::Validation {
            tool: tool_name,
            message: "Missing 'action' field".to_string(),
        })?;

    // Handle case where action is a JSON string that needs parsing
    let action: std::borrow::Cow<'_, Value> = if let Some(action_str) = action_value.as_str() {
        // Try to parse the string as JSON
        let parsed: Value = serde_json::from_str(action_str).map_err(|e| ToolError::Validation {
            tool: tool_name,
            message: format!("Invalid JSON in 'action' field: {e}"),
        })?;
        std::borrow::Cow::Owned(parsed)
    } else {
        // Already a JSON object
        std::borrow::Cow::Borrowed(action_value)
    };

    let action_type = action
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolError::Validation {
            tool: tool_name,
            message: "Missing 'action.type' field".to_string(),
        })?;

    let url = action.get("url").and_then(|v| v.as_str());
    parse_action_with_url(action_type, url, tool_name, "url")
}

/// Parse flat action schema used by Gemini and Z.AI.
fn parse_flat_action_schema(
    input: &Value,
    tool_name: &'static str,
    action_field: &str,
    url_field: &str,
) -> Result<InternalBridgeParams, ToolError> {
    let action_type = input
        .get(action_field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolError::Validation {
            tool: tool_name,
            message: format!("Missing '{action_field}' field"),
        })?;

    let url = input.get(url_field).and_then(|v| v.as_str());
    parse_action_with_url(action_type, url, tool_name, url_field)
}

// ============================================================================
// Provider Facades
// ============================================================================

/// Claude-specific facade for bridge operations.
///
/// Maps Claude's `Bridge` tool with nested action schema to internal parameters.
pub struct ClaudeBridgeFacade;

impl BridgeToolFacade for ClaudeBridgeFacade {
    fn provider(&self) -> &'static str {
        "claude"
    }

    fn tool_name(&self) -> &'static str {
        "Bridge"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "Bridge".to_string(),
            description: BRIDGE_DESCRIPTION_NESTED.to_string(),
            parameters: nested_action_schema(),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalBridgeParams, ToolError> {
        parse_nested_action_schema(&input, "bridge")
    }
}

/// OpenAI-specific facade for bridge operations.
///
/// Maps OpenAI's `bridge` tool with nested action schema to internal parameters.
pub struct OpenAIBridgeFacade;

impl BridgeToolFacade for OpenAIBridgeFacade {
    fn provider(&self) -> &'static str {
        "openai"
    }

    fn tool_name(&self) -> &'static str {
        "bridge"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "bridge".to_string(),
            description: BRIDGE_DESCRIPTION_NESTED.to_string(),
            parameters: nested_action_schema(),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalBridgeParams, ToolError> {
        parse_nested_action_schema(&input, "bridge")
    }
}

/// Gemini-specific facade for bridge operations.
///
/// Maps Gemini's `bridge_connection` tool with flat snake_case schema to internal parameters.
pub struct GeminiBridgeFacade;

impl BridgeToolFacade for GeminiBridgeFacade {
    fn provider(&self) -> &'static str {
        "gemini"
    }

    fn tool_name(&self) -> &'static str {
        "bridge_connection"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "bridge_connection".to_string(),
            description: BRIDGE_DESCRIPTION_FLAT.to_string(),
            parameters: flat_action_schema(
                "action_type",
                "url",
                "WebSocket URL (required for connect/disconnect actions, e.g., ws://localhost:8080)",
            ),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalBridgeParams, ToolError> {
        parse_flat_action_schema(&input, "bridge_connection", "action_type", "url")
    }
}

/// Z.AI-specific facade for bridge operations.
///
/// Maps Z.AI's `manage_bridge` tool with flat schema to internal parameters.
pub struct ZAIBridgeFacade;

impl BridgeToolFacade for ZAIBridgeFacade {
    fn provider(&self) -> &'static str {
        "zai"
    }

    fn tool_name(&self) -> &'static str {
        "manage_bridge"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "manage_bridge".to_string(),
            description: concat!(
                "Manage WebSocket bridge connections to external endpoints. ",
                "connect: establish connection, disconnect: close connection, list: show bridges."
            )
            .to_string(),
            parameters: flat_action_schema(
                "action",
                "endpoint_url",
                "WebSocket URL (required for connect/disconnect)",
            ),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalBridgeParams, ToolError> {
        parse_flat_action_schema(&input, "manage_bridge", "action", "endpoint_url")
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_facade_provider() {
        let facade = ClaudeBridgeFacade;
        assert_eq!(facade.provider(), "claude");
        assert_eq!(facade.tool_name(), "Bridge");
    }

    #[test]
    fn test_claude_facade_connect() {
        let facade = ClaudeBridgeFacade;
        let input = json!({
            "action": {
                "type": "connect",
                "url": "ws://localhost:8080"
            }
        });
        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalBridgeParams::Connect {
                url: "ws://localhost:8080".to_string()
            }
        );
    }

    #[test]
    fn test_claude_facade_disconnect() {
        let facade = ClaudeBridgeFacade;
        let input = json!({
            "action": {
                "type": "disconnect",
                "url": "ws://localhost:8080"
            }
        });
        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalBridgeParams::Disconnect {
                url: "ws://localhost:8080".to_string()
            }
        );
    }

    #[test]
    fn test_claude_facade_list() {
        let facade = ClaudeBridgeFacade;
        let input = json!({
            "action": {
                "type": "list"
            }
        });
        let result = facade.map_params(input).unwrap();
        assert_eq!(result, InternalBridgeParams::List);
    }

    #[test]
    fn test_openai_facade_provider() {
        let facade = OpenAIBridgeFacade;
        assert_eq!(facade.provider(), "openai");
        assert_eq!(facade.tool_name(), "bridge");
    }

    #[test]
    fn test_openai_facade_uses_same_schema_as_claude() {
        let claude_def = ClaudeBridgeFacade.definition();
        let openai_def = OpenAIBridgeFacade.definition();
        // Same schema structure, different names
        assert_eq!(claude_def.parameters, openai_def.parameters);
        assert_eq!(claude_def.description, openai_def.description);
        assert_ne!(claude_def.name, openai_def.name);
    }

    #[test]
    fn test_gemini_facade_provider() {
        let facade = GeminiBridgeFacade;
        assert_eq!(facade.provider(), "gemini");
        assert_eq!(facade.tool_name(), "bridge_connection");
    }

    #[test]
    fn test_gemini_facade_connect() {
        let facade = GeminiBridgeFacade;
        let input = json!({
            "action_type": "connect",
            "url": "ws://localhost:8080"
        });
        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalBridgeParams::Connect {
                url: "ws://localhost:8080".to_string()
            }
        );
    }

    #[test]
    fn test_zai_facade_provider() {
        let facade = ZAIBridgeFacade;
        assert_eq!(facade.provider(), "zai");
        assert_eq!(facade.tool_name(), "manage_bridge");
    }

    #[test]
    fn test_zai_facade_connect() {
        let facade = ZAIBridgeFacade;
        let input = json!({
            "action": "connect",
            "endpoint_url": "ws://localhost:8080"
        });
        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalBridgeParams::Connect {
                url: "ws://localhost:8080".to_string()
            }
        );
    }

    #[test]
    fn test_missing_action_field_error() {
        let facade = ClaudeBridgeFacade;
        let input = json!({});
        let result = facade.map_params(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing 'action' field"));
    }

    #[test]
    fn test_missing_url_for_connect_error() {
        let facade = ClaudeBridgeFacade;
        let input = json!({
            "action": {
                "type": "connect"
            }
        });
        let result = facade.map_params(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing 'url' field"));
    }

    #[test]
    fn test_unknown_action_type_error() {
        let facade = ClaudeBridgeFacade;
        let input = json!({
            "action": {
                "type": "unknown"
            }
        });
        let result = facade.map_params(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown action type"));
    }

    #[test]
    fn test_claude_facade_connect_with_json_string() {
        // Test case where action is passed as a JSON string (common from XML tool calls)
        let facade = ClaudeBridgeFacade;
        let input = json!({
            "action": r#"{"type": "connect", "url": "ws://localhost:8080"}"#
        });
        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalBridgeParams::Connect {
                url: "ws://localhost:8080".to_string()
            }
        );
    }

    #[test]
    fn test_claude_facade_list_with_json_string() {
        // Test list action passed as JSON string
        let facade = ClaudeBridgeFacade;
        let input = json!({
            "action": r#"{"type": "list"}"#
        });
        let result = facade.map_params(input).unwrap();
        assert_eq!(result, InternalBridgeParams::List);
    }

    #[test]
    fn test_claude_facade_disconnect_with_json_string() {
        // Test disconnect action passed as JSON string
        let facade = ClaudeBridgeFacade;
        let input = json!({
            "action": r#"{"type": "disconnect", "url": "ws://localhost:8080"}"#
        });
        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalBridgeParams::Disconnect {
                url: "ws://localhost:8080".to_string()
            }
        );
    }

    #[test]
    fn test_invalid_json_string_error() {
        let facade = ClaudeBridgeFacade;
        let input = json!({
            "action": "not valid json"
        });
        let result = facade.map_params(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid JSON"));
    }
}
