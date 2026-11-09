# COV-061 AST research

## fspec astgrep --pattern 'use codelet::$$$A;' --lang rust --path rust/tests
rust/tests/integration_wiring_test.rs:92:5:use codelet::rig;


## fspec astgrep --pattern 'pub fn handle_debug_command($$$A) $R { $$$B }' --lang rust --path rust/common/src/debug_capture
rust/common/src/debug_capture/mod.rs:47:1:pub fn handle_debug_command() -> DebugCommandResult {


## fspec astgrep --pattern 'pub fn init_logging($$$A) $$$R' --lang rust --path rust/common/src/logging
rust/common/src/logging/mod.rs:77:1:pub fn init_logging(verbose: bool) -> Result<()> {


## fspec astgrep --pattern 'use $P;' --lang rust --path rust/tests (filtered to codelet:: paths)
rust/tests/integration_wiring_test.rs:11:1:use codelet::agent::{RigAgent, DEFAULT_MAX_DEPTH};
rust/tests/integration_wiring_test.rs:12:1:use codelet::providers::{ClaudeProvider, LlmProvider};
rust/tests/integration_wiring_test.rs:13:1:use codelet::tools::{
rust/tests/integration_wiring_test.rs:92:5:use codelet::rig;
rust/tests/project_scaffold_test.rs:13:5:use codelet::agent::Runner;
rust/tests/project_scaffold_test.rs:14:5:use codelet::cli::Cli;
rust/tests/project_scaffold_test.rs:15:5:use codelet::context::TokenTracker;
rust/tests/project_scaffold_test.rs:16:5:use codelet::providers::ProviderType;
rust/tests/project_scaffold_test.rs:17:5:use codelet::tools::ToolRegistry;
rust/tests/project_scaffold_test.rs:50:5:use codelet::tools::{Tool, ToolOutput, ToolParameters, ToolRegistry};
rust/tests/project_scaffold_test.rs:87:5:use codelet::providers::{LlmProvider, ProviderType};
rust/tests/project_scaffold_test.rs:108:5:use codelet::agent::{Message, MessageContent, MessageRole, Runner};
rust/tests/project_scaffold_test.rs:125:5:use codelet::context::TokenTracker;
rust/tests/project_scaffold_test.rs:148:5:use codelet::cli::Cli;
rust/tests/large_write_intent_test.rs:7:1:use codelet::cli::large_write_intent::{
rust/tests/agent_multi_turn_test.rs:4:1:use codelet::agent::{RigAgent, Runner, DEFAULT_MAX_DEPTH};
rust/tests/agent_multi_turn_test.rs:5:1:use codelet::tools::ToolRegistry;
rust/tests/agent_multi_turn_test.rs:157:5:use codelet::tools::{
rust/tests/claude_provider_test.rs:10:1:use codelet::agent::Message;
rust/tests/claude_provider_test.rs:11:1:use codelet::providers::ClaudeProvider;
rust/tests/claude_provider_test.rs:12:1:use codelet::providers::LlmProvider;
rust/tests/debug_capture_test.rs:19:1:use codelet::debug_capture::{get_debug_capture_manager, handle_debug_command, DebugEvent};
rust/tests/astgrep_tool_test.rs:11:1:use codelet::agent::Runner;
rust/tests/astgrep_tool_test.rs:12:1:use codelet::tools::{astgrep::AstGrepTool, Tool, ToolRegistry};
rust/tests/core_file_tools_test.rs:8:1:use codelet::tools::{
rust/tests/core_file_tools_test.rs:446:1:use codelet::tools::ToolRegistry;
rust/tests/core_file_tools_test.rs:528:1:use codelet::agent::Runner;
rust/tests/cache_token_extraction_test.rs:9:1:use codelet::agent::compaction::TokenTracker;
rust/tests/agent_runner_test.rs:12:1:use codelet::agent::{ContentPart, Message, MessageContent, MessageRole, Runner};
rust/tests/agent_runner_test.rs:13:1:use codelet::providers::{CompletionResponse, LlmProvider, StopReason};
rust/tests/agent_runner_test.rs:14:1:use codelet::tools::{ToolDefinition, ToolRegistry};
rust/tests/grep_glob_tools_test.rs:11:1:use codelet::agent::Runner;
rust/tests/grep_glob_tools_test.rs:12:1:use codelet::tools::{glob::GlobTool, grep::GrepTool, Tool, ToolRegistry};
rust/tests/claude_oauth_test.rs:12:1:use codelet::providers::ClaudeProvider;
rust/tests/provider_manager_test.rs:8:1:use codelet::providers::ProviderManager;
rust/tests/bash_tool_test.rs:7:1:use codelet::agent::Runner;
rust/tests/bash_tool_test.rs:8:1:use codelet::tools::{bash::BashTool, limits::OutputLimits, Tool, ToolRegistry};
rust/tests/rig_provider_streaming_test.rs:10:1:use codelet::agent::{Message, MessageContent, MessageRole};
rust/tests/rig_provider_streaming_test.rs:11:1:use codelet::providers::{ClaudeProvider, LlmProvider};
rust/tests/rig_provider_streaming_test.rs:12:1:use codelet::tools::ToolDefinition;
rust/tests/rig_provider_streaming_test.rs:128:5:use codelet::providers::AuthMode;
rust/tests/context_gathering_test.rs:7:1:use codelet::session::context_gathering::{discover_agents_md, gather_environment_info};
rust/tests/system_reminder_infrastructure_test.rs:14:1:use codelet::session::{
rust/tests/openai_provider_test.rs:8:1:use codelet::agent::{Message, MessageContent, MessageRole};
rust/tests/openai_provider_test.rs:9:1:use codelet::providers::{LlmProvider, OpenAIProvider};
rust/tests/system_reminder_cache_preservation_test.rs:12:1:use codelet::session::system_reminders::{
rust/tests/rig_agent_multi_turn_test.rs:10:1:use codelet::agent::Runner;
rust/tests/rig_agent_multi_turn_test.rs:11:1:use codelet::providers::ClaudeProvider;
rust/tests/rig_agent_multi_turn_test.rs:12:1:use codelet::tools::ToolRegistry;
rust/tests/gemini_provider_test.rs:8:1:use codelet::providers::{GeminiProvider, LlmProvider};
rust/tests/prompt_cache_control_test.rs:5:1:use codelet::providers::claude::{build_cached_system_prompt, CacheControl};

## fspec astgrep --pattern 'use $P;' --lang rust --path rust/examples (filtered to codelet:: paths)
rust/examples/detailed_demo.rs:1:1:use codelet::tools::{
rust/examples/test_all_17_events.rs:6:1:use codelet::debug_capture::{
rust/examples/openai_demo.rs:6:1:use codelet::providers::{LlmProvider, OpenAIProvider};
rust/examples/agent_tool_calling.rs:8:1:use codelet::agent::RigAgent;
rust/examples/agent_tool_calling.rs:9:1:use codelet::providers::ClaudeProvider;
rust/examples/test_all_tools.rs:5:1:use codelet::agent::RigAgent;
rust/examples/test_all_tools.rs:6:1:use codelet::providers::{ClaudeProvider, LlmProvider};
rust/examples/test_all_tools.rs:7:1:use codelet::tools::{Tool, ToolRegistry};
rust/examples/test_debug_e2e.rs:3:1:use codelet::debug_capture::{

## Findings

- Every rust/tests and rust/examples file except five (context_window_limits_test, env_config_test,
  integration_test, project_scaffold_test, tool_display_test, verify_rig_reexport — which use only
  std or a binary) imports the non-existent `codelet` crate (`codelet::agent::Runner`,
  `codelet::tools::ToolRegistry`, `codelet::providers::*`, ...). `Runner`, `ToolRegistry` and
  `build_cached_system_prompt` exist in no workspace crate any more.
- The debug-capture and logging APIs they exercise still exist
  (common/src/debug_capture/mod.rs:47 `handle_debug_command`, common/src/logging/mod.rs:77
  `init_logging`).
