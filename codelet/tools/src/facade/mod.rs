//! Provider-Specific Tool Facades
//!
//! This module implements the facade pattern for tools, allowing different LLM providers
//! to receive tool schemas in their native format while sharing a common implementation.
//!
//! # Architecture
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────────────┐
//! │                        Provider Layer                                  │
//! ├──────────┬──────────┬──────────┬──────────┬──────────┬───────────────┤
//! │  Claude  │  Gemini  │  OpenAI  │   Codex  │   Z.AI   │    Others     │
//! │  Facade  │  Facade  │  Facade  │  Facade  │  Facade  │   Facade      │
//! ├──────────┴──────────┴──────────┴──────────┴──────────┴───────────────┤
//! │                     Tool Adapter Layer                                 │
//! │            (maps provider params → internal params)                    │
//! ├──────────────────────────────────────────────────────────────────────┤
//! │                    Base Tool Implementation                            │
//! │                 (Chrome browser, actual work)                          │
//! └──────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Z.AI/GLM Facades
//!
//! The Z.AI facades (`ZAI*Facade`) are optimized for GLM models:
//! - snake_case tool names (e.g., `list_dir`, `read_file`)
//! - Flat JSON schemas with explicit `default` values
//! - `additionalProperties: false` to prevent unexpected fields
//! - Clear, concise descriptions

mod bash;
mod file_ops;
mod gemini_history;
mod ls;
mod registry;
mod search;
mod system_prompt;
mod thinking_config;
mod traits;
mod web_search;
mod wrapper;
mod zai;

pub use bash::GeminiRunShellCommandFacade;
pub use file_ops::{GeminiReadFileFacade, GeminiReplaceFacade, GeminiWriteFileFacade};
pub use ls::GeminiListDirectoryFacade;
pub use zai::{
    ZAIEditFileFacade, ZAIFindFilesFacade, ZAIGrepFilesFacade, ZAIListDirFacade,
    ZAIReadFileFacade, ZAIRunCommandFacade, ZAIWriteFileFacade,
};
pub use registry::ProviderToolRegistry;
pub use search::{GeminiGlobFacade, GeminiSearchFileContentFacade};
pub use system_prompt::{
    build_gemini_system_prompt, select_claude_facade, BoxedSystemPromptFacade,
    ClaudeApiKeySystemPromptFacade, ClaudeOAuthSystemPromptFacade, GeminiSystemPromptFacade,
    OpenAISystemPromptFacade, SystemPromptFacade, CLAUDE_CODE_PROMPT_PREFIX,
    GEMINI_3_TOOL_INSTRUCTION, GEMINI_BASE_SYSTEM_PROMPT,
};
pub use thinking_config::{
    ClaudeThinkingFacade, Gemini25ThinkingFacade, Gemini3ThinkingFacade, ThinkingConfigFacade,
    ThinkingLevel,
};
pub use gemini_history::{
    ContinuationStrategy, DefaultHistoryFacade, DefaultTurnCompletionFacade, GeminiHistoryFacade,
    GeminiTurnCompletionFacade, HistoryPreparationFacade, TurnCompletionFacade,
    SYNTHETIC_THOUGHT_SIGNATURE,
};
pub use traits::{
    BashToolFacade, BoxedBashToolFacade, BoxedFileToolFacade, BoxedLsToolFacade,
    BoxedSearchToolFacade, BoxedToolFacade, FileToolFacade, InternalBashParams, InternalFileParams,
    InternalLsParams, InternalSearchParams, InternalWebSearchParams, LsToolFacade,
    SearchToolFacade, ToolDefinition, ToolFacade,
};
pub use web_search::{
    ClaudeWebSearchFacade, GeminiGoogleWebSearchFacade, GeminiWebFetchFacade,
    GeminiWebScreenshotFacade,
};
pub use wrapper::{
    BashToolFacadeWrapper, FacadeToolWrapper, FileToolFacadeWrapper, LsToolFacadeWrapper,
    SearchToolFacadeWrapper,
};
