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
mod bridge_facade;
mod bridge_registration;
mod codex;
mod file_ops;
mod fspec_facade;
mod fspec_registration;
mod gemini_history;
mod ls;
pub(crate) mod param_extract;
mod registry;
mod search;
mod system_prompt;
mod thinking_config;
mod traits;
mod web_search;
pub mod wrapper;
mod zai;

pub use bash::GeminiRunShellCommandFacade;
pub use bridge_facade::{
    BoxedBridgeToolFacade, BridgeToolFacade, ClaudeBridgeFacade, GeminiBridgeFacade,
    InternalBridgeParams, OpenAIBridgeFacade, ZAIBridgeFacade,
};
pub use bridge_registration::{
    bridge_tool_for_provider, claude_bridge_tool, codex_bridge_tool, gemini_bridge_tool,
    openai_bridge_tool, zai_bridge_tool,
};
pub use file_ops::{GeminiReadFileFacade, GeminiReplaceFacade, GeminiWriteFileFacade};
pub use fspec_facade::{
    ClaudeFspecFacade, GeminiFspecFacade, InternalFspecParams, OpenAIFspecFacade, ZAIFspecFacade,
};
pub use fspec_registration::{
    claude_fspec_tool, codex_fspec_tool, fspec_tool_for_provider, gemini_fspec_tool,
    openai_fspec_tool, zai_fspec_tool,
};
pub use ls::GeminiListDirectoryFacade;
pub use zai::{
    ZAIEditFileFacade, ZAIFindFilesFacade, ZAIGrepFilesFacade, ZAIListDirFacade,
    ZAIReadFileFacade, ZAIRunCommandFacade, ZAIWriteFileFacade,
};
pub use codex::{
    CodexExecCommandFacade, CodexGrepFilesFacade, CodexListDirFacade, CodexReadFileFacade,
    CodexShellCommandFacade, CodexShellFacade, CodexViewImageFacade,
};
pub use registry::ProviderToolRegistry;
pub use search::{GeminiGlobFacade, GeminiSearchFileContentFacade};
pub use system_prompt::{
    build_gemini_system_prompt, prepend_fspec_guidance, select_claude_facade, BoxedSystemPromptFacade,
    ClaudeApiKeySystemPromptFacade, ClaudeOAuthSystemPromptFacade, GeminiSystemPromptFacade,
    OpenAISystemPromptFacade, SystemPromptFacade, CLAUDE_CODE_PROMPT_PREFIX,
    GEMINI_3_TOOL_INSTRUCTION, GEMINI_BASE_SYSTEM_PROMPT,
};
pub use thinking_config::{
    ClaudeThinkingFacade, Gemini25ThinkingFacade, Gemini3ThinkingFacade, ThinkingConfigFacade,
    ThinkingLevel,
    // PROV-005: Claude model constants and adaptive thinking helpers
    CLAUDE_OPUS_4_6, CLAUDE_SONNET_4_6, CLAUDE_SONNET_4_5, CLAUDE_OPUS_4_5,
    ADAPTIVE_THINKING_MODELS, CONTEXT_1M_MODELS,
    is_adaptive_thinking_model, supports_1m_context,
};
pub use gemini_history::{
    ContinuationStrategy, DefaultHistoryFacade, DefaultTurnCompletionFacade, GeminiHistoryFacade,
    GeminiTurnCompletionFacade, HistoryPreparationFacade, TurnCompletionFacade,
    SYNTHETIC_THOUGHT_SIGNATURE,
};
pub use traits::{
    BashToolFacade, BoxedBashToolFacade, BoxedFileToolFacade, BoxedLsToolFacade,
    BoxedSearchToolFacade, BoxedToolFacade, BoxedFspecToolFacade, FileToolFacade,
    FspecToolFacade, InternalBashParams, InternalFileParams, InternalIndentationParams,
    InternalLsParams, InternalSearchParams, InternalWebSearchParams, LsToolFacade,
    SearchToolFacade, ToolDefinition, ToolFacade,
    // TOOL-016: Unified exec facade types
    BoxedExecToolFacade, ExecToolFacade, InternalExecParams,
};
pub use web_search::{
    ClaudeWebSearchFacade, GeminiGoogleWebSearchFacade, GeminiWebFetchFacade,
    GeminiWebScreenshotFacade,
};
pub use wrapper::{
    BashToolFacadeWrapper, BridgeToolFacadeWrapper, FacadeToolWrapper, FileToolFacadeWrapper,
    FspecToolFacadeWrapper, LsToolFacadeWrapper, SearchToolFacadeWrapper,
    // TOOL-016: Exec tool facade wrapper
    ExecToolFacadeWrapper, ExecOperationResult,
    // BLOCK-006: Block notification callbacks
    emit_block_notification, set_block_notification_callback, set_get_work_unit_stage_callback,
    BlockNotificationCallback, GetWorkUnitStageCallback,
    // GIT-020: Effective CWD callback for isolated session support
    set_get_effective_cwd_callback, get_effective_cwd, get_isolation_context, 
    GetEffectiveCwdCallback, IsolationContext,
    // TOOL-014: Path validation for worktree isolation
    validate_and_resolve_path, validate_and_resolve_path_with_cwd, validate_and_resolve_path_with_isolation,
};
