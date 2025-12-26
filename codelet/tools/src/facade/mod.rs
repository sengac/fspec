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

mod registry;
mod traits;
mod web_search;
mod wrapper;

pub use registry::ProviderToolRegistry;
pub use traits::{BoxedToolFacade, InternalWebSearchParams, ToolDefinition, ToolFacade};
pub use web_search::{
    ClaudeWebSearchFacade, GeminiGoogleWebSearchFacade, GeminiWebFetchFacade,
};
pub use wrapper::FacadeToolWrapper;
