//! Provider-Specific Tool Facades
//!
//! This module implements the facade pattern for tools, allowing different LLM providers
//! to receive tool schemas in their native format while sharing a common implementation.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     Provider Layer                          │
//! ├─────────────┬─────────────┬─────────────┬──────────────────┤
//! │   Claude    │   Gemini    │   OpenAI    │     Codex        │
//! │  Facade     │   Facade    │   Facade    │    Facade        │
//! ├─────────────┴─────────────┴─────────────┴──────────────────┤
//! │                  Tool Adapter Layer                         │
//! │         (maps provider params → internal params)            │
//! ├────────────────────────────────────────────────────────────┤
//! │                 Base Tool Implementation                    │
//! │              (Chrome browser, actual work)                  │
//! └────────────────────────────────────────────────────────────┘
//! ```

mod bash;
mod file_ops;
mod ls;
mod registry;
mod search;
mod system_prompt;
mod thinking_config;
mod traits;
mod web_search;
mod wrapper;

pub use bash::GeminiRunShellCommandFacade;
pub use file_ops::{GeminiReadFileFacade, GeminiReplaceFacade, GeminiWriteFileFacade};
pub use ls::GeminiListDirectoryFacade;
pub use registry::ProviderToolRegistry;
pub use search::{GeminiGlobFacade, GeminiSearchFileContentFacade};
pub use traits::{
    BashToolFacade, BoxedBashToolFacade, BoxedFileToolFacade, BoxedLsToolFacade,
    BoxedSearchToolFacade, BoxedToolFacade, FileToolFacade, InternalBashParams, InternalFileParams,
    InternalLsParams, InternalSearchParams, InternalWebSearchParams, LsToolFacade,
    SearchToolFacade, ToolDefinition, ToolFacade,
};
pub use system_prompt::{
    select_claude_facade, BoxedSystemPromptFacade, ClaudeApiKeySystemPromptFacade,
    ClaudeOAuthSystemPromptFacade, GeminiSystemPromptFacade, OpenAISystemPromptFacade,
    SystemPromptFacade, CLAUDE_CODE_PROMPT_PREFIX,
};
pub use thinking_config::{
    ClaudeThinkingFacade, Gemini25ThinkingFacade, Gemini3ThinkingFacade, ThinkingConfigFacade,
    ThinkingLevel,
};
pub use web_search::{
    ClaudeWebSearchFacade, GeminiGoogleWebSearchFacade, GeminiWebFetchFacade,
    GeminiWebScreenshotFacade,
};
pub use wrapper::{
    BashToolFacadeWrapper, FacadeToolWrapper, FileToolFacadeWrapper, LsToolFacadeWrapper,
    SearchToolFacadeWrapper,
};
