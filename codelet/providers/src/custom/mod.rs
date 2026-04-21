//! Custom scripted providers (PROV-062 + PROV-063).
//!
//! This module implements discovery, validation, compilation, and the
//! runtime `LlmProvider` bridge for user-supplied custom LLM provider
//! plugins. A plugin is a JSON [`ProviderConfig`] file plus a sibling
//! `.rhai` script that defines the 7 required provider-lifecycle
//! functions.
//!
//! Public surface:
//! - [`ProviderConfig`], [`AuthConfig`], [`ModelDef`], [`Defaults`],
//!   [`SystemPromptConfig`], [`ToolStyle`], [`ApiStyle`] — config schema
//! - [`ScriptLoader`] — compiles and caches Rhai ASTs, validates required
//!   functions against the sandboxed engine built on top of PROV-060
//! - [`discover_provider_configs`] — scans `~/.fspec/providers/*.json` and
//!   `.fspec/providers/*.json` with project-local override
//! - [`RhaiCustomProvider`] — async `LlmProvider` implementation that
//!   delegates to the compiled Rhai script
//! - [`request_bridge`] / [`response_bridge`] — Rhai ↔ Rust conversion
//! - [`error::CustomProviderError`] — all failure modes

pub mod config;
mod conversion;
pub mod custom_provider;
pub mod discovery;
pub mod error;
pub mod error_mapping;
mod http;
mod internal_dispatch;
pub(crate) mod log_helpers;
pub mod management;
pub mod provider;
mod provider_stream;
pub mod request_bridge;
pub mod response_bridge;
mod rhai_call;
pub mod rig_message_convert;
pub mod rig_model;
pub mod rig_tool;
pub mod script_loader;
pub mod stream;
mod stream_convert;
mod stream_http;
pub mod system_prompt;
pub mod tool_dispatch;
mod tool_dispatch_extras;
pub mod tool_facade;
pub mod tool_presets;
pub mod tool_resolve;
mod tool_schemas;

pub use config::{
    ApiStyle, AuthConfig, Defaults, ModelDef, ProviderConfig, SystemPromptConfig, ToolStyle,
};
pub use custom_provider::{CustomProvider, CustomRigAgent};
pub use discovery::discover_provider_configs;
pub use management::{
    apply_custom_provider_env_vars, derive_facade_for_custom, init_provider_template,
    list_providers_info, resolve_custom_model_id, show_provider_info, test_provider_connection,
    validate_provider_config, ProviderInfo, ProviderTestResult,
};
pub use provider::RhaiCustomProvider;
pub use rig_model::{RhaiCustomCompletion, RhaiCustomProviderModel};
pub use rig_tool::{RhaiToolArgs, RhaiToolWrapper};
pub use script_loader::ScriptLoader;
pub use system_prompt::RhaiSystemPromptFacade;
pub use tool_dispatch::{default_to_internal, DispatchedToolParams};
pub use tool_facade::{
    apply_map_tool_params, default_to_internal_file, RhaiToolDef, RhaiToolFacadeAdapter,
};
pub use tool_presets::preset_tools;
pub use tool_resolve::resolve_tools;
