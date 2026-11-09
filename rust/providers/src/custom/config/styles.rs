//! Tool-calling and API request/response styles (split from config.rs by REFAC-030).

use serde::{Deserialize, Serialize};

/// Tool-calling convention used by a custom provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolStyle {
    /// OpenAI function-calling format (snake_case tool names).
    Openai,
    /// Anthropic `tool_use` format (kept for backward compatibility with
    /// earlier configs; equivalent to [`ToolStyle::Claude`]).
    Anthropic,
    /// Claude-native tool names (PascalCase, e.g. `Read`, `Write`).
    Claude,
    /// Gemini-native tool names.
    Gemini,
    /// Codex-native tool names (camelCase).
    Codex,
}

/// API request/response shape used by a custom provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiStyle {
    /// OpenAI `/chat/completions` shape.
    OpenaiChat,
    /// Anthropic `/v1/messages` shape.
    AnthropicMessages,
}
