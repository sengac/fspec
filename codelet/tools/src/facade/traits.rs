//! Core traits and types for the tool facade pattern.

use crate::ToolError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

/// Tool definition that can be sent to LLM providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name as the provider expects it
    pub name: String,
    /// Tool description
    pub description: String,
    /// JSON Schema for the tool parameters
    pub parameters: Value,
}

/// Internal parameters for web search operations.
/// All provider-specific parameters are mapped to these internal types.
#[derive(Debug, Clone, PartialEq)]
pub enum InternalWebSearchParams {
    /// Perform a web search with the given query
    Search { query: String },
    /// Open and fetch content from a URL
    OpenPage { url: String },
    /// Find a pattern within a page's content
    FindInPage { url: String, pattern: String },
}

/// Provider-specific tool facade trait.
///
/// Each facade adapts a tool's interface for a specific LLM provider,
/// handling differences in tool naming, parameter schemas, and parameter formats.
pub trait ToolFacade: Send + Sync {
    /// Returns the provider this facade is for (e.g., "claude", "gemini", "openai")
    fn provider(&self) -> &'static str;

    /// Returns the tool name as the provider expects it
    fn tool_name(&self) -> &'static str;

    /// Returns the tool definition with provider-specific schema
    fn definition(&self) -> ToolDefinition;

    /// Maps provider-specific parameters to internal parameters
    fn map_params(&self, input: Value) -> Result<InternalWebSearchParams, ToolError>;
}

/// Type alias for a boxed ToolFacade
pub type BoxedToolFacade = Arc<dyn ToolFacade>;
