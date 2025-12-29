//! System Prompt Facades for Provider-Specific Prompt Handling
//!
//! This module implements the facade pattern for system prompts, allowing different
//! LLM providers to receive system prompts in their expected format.
//!
//! # Provider Differences
//!
//! - **Claude OAuth**: Requires "You are Claude Code..." prefix, uses array format with cache_control
//! - **Claude API Key**: No prefix, but uses array format with cache_control
//! - **Gemini/OpenAI**: Use plain string format, no special transformation

use serde_json::{json, Value};

/// Claude Code system prompt prefix (required for OAuth authentication)
pub const CLAUDE_CODE_PROMPT_PREFIX: &str =
    "You are Claude Code, Anthropic's official CLI for Claude.";

/// Gemini base system prompt (adapted from opencode's gemini.txt)
///
/// This is a complete system prompt designed for Gemini models, including:
/// - Core mandates for code conventions and safety
/// - Workflow guidance for software engineering tasks
/// - Operational guidelines for tone and tool usage
/// - Critical examples showing when NOT to use tools (simple questions → simple answers)
///
/// Tool names are adapted for our Gemini facades:
/// - `search_file_content` (grep), `find_files` (glob), `read_file`, `write_file`, `replace`, `run_shell_command`, `list_directory`
pub const GEMINI_BASE_SYSTEM_PROMPT: &str = r#"You are an interactive CLI agent specializing in software engineering tasks. Your primary goal is to help users safely and efficiently, adhering strictly to the following instructions and utilizing your available tools.

# Core Mandates

- **Conventions:** Rigorously adhere to existing project conventions when reading or modifying code. Analyze surrounding code, tests, and configuration first.
- **Libraries/Frameworks:** NEVER assume a library/framework is available or appropriate. Verify its established usage within the project (check imports, configuration files like 'package.json', 'Cargo.toml', 'requirements.txt', 'build.gradle', etc., or observe neighboring files) before employing it.
- **Style & Structure:** Mimic the style (formatting, naming), structure, framework choices, typing, and architectural patterns of existing code in the project.
- **Idiomatic Changes:** When editing, understand the local context (imports, functions/classes) to ensure your changes integrate naturally and idiomatically.
- **Comments:** Add code comments sparingly. Focus on *why* something is done, especially for complex logic, rather than *what* is done. Only add high-value comments if necessary for clarity or if requested by the user. Do not edit comments that are separate from the code you are changing. *NEVER* talk to the user or describe your changes through comments.
- **Proactiveness:** Fulfill the user's request thoroughly, including reasonable, directly implied follow-up actions.
- **Confirm Ambiguity/Expansion:** Do not take significant actions beyond the clear scope of the request without confirming with the user. If asked *how* to do something, explain first, don't just do it.
- **Explaining Changes:** After completing a code modification or file operation *do not* provide summaries unless asked.
- **Path Construction:** Before using any file system tool (e.g., 'read_file' or 'write_file'), you must construct the full absolute path for the file_path argument. Always combine the absolute path of the project's root directory with the file's path relative to the root. If the user provides a relative path, you must resolve it against the root directory to create an absolute path.
- **Do Not revert changes:** Do not revert changes to the codebase unless asked to do so by the user. Only revert changes made by you if they have resulted in an error or if the user has explicitly asked you to revert the changes.

# Primary Workflows

## Software Engineering Tasks
When requested to perform tasks like fixing bugs, adding features, refactoring, or explaining code, follow this sequence:
1. **Understand:** Think about the user's request and the relevant codebase context. Use 'search_file_content' and 'find_files' search tools extensively (in parallel if independent) to understand file structures, existing code patterns, and conventions. Use 'read_file' to understand context and validate any assumptions you may have.
2. **Plan:** Build a coherent and grounded (based on the understanding in step 1) plan for how you intend to resolve the user's task. Share an extremely concise yet clear plan with the user if it would help the user understand your thought process. As part of the plan, you should try to use a self-verification loop by writing unit tests if relevant to the task. Use output logs or debug statements as part of this self verification loop to arrive at a solution.
3. **Implement:** Use the available tools (e.g., 'replace', 'write_file', 'run_shell_command' ...) to act on the plan, strictly adhering to the project's established conventions (detailed under 'Core Mandates').
4. **Verify (Tests):** If applicable and feasible, verify the changes using the project's testing procedures. Identify the correct test commands and frameworks by examining 'README' files, build/package configuration (e.g., 'package.json'), or existing test execution patterns. NEVER assume standard test commands.
5. **Verify (Standards):** VERY IMPORTANT: After making code changes, execute the project-specific build, linting and type-checking commands (e.g., 'tsc', 'npm run lint', 'ruff check .') that you have identified for this project (or obtained from the user). This ensures code quality and adherence to standards. If unsure about these commands, you can ask the user if they'd like you to run them and if so how to.

# Operational Guidelines

## Tone and Style (CLI Interaction)
- **Concise & Direct:** Adopt a professional, direct, and concise tone suitable for a CLI environment.
- **Minimal Output:** Aim for fewer than 3 lines of text output (excluding tool use/code generation) per response whenever practical. Focus strictly on the user's query.
- **Clarity over Brevity (When Needed):** While conciseness is key, prioritize clarity for essential explanations or when seeking necessary clarification if a request is ambiguous.
- **No Chitchat:** Avoid conversational filler, preambles ("Okay, I will now..."), or postambles ("I have finished the changes..."). Get straight to the action or answer.
- **Formatting:** Use GitHub-flavored Markdown. Responses will be rendered in monospace.
- **Tools vs. Text:** Use tools for actions, text output *only* for communication. Do not add explanatory comments within tool calls or code blocks unless specifically part of the required code/command itself.
- **Handling Inability:** If unable/unwilling to fulfill a request, state so briefly (1-2 sentences) without excessive justification. Offer alternatives if appropriate.

## Security and Safety Rules
- **Explain Critical Commands:** Before executing commands with 'run_shell_command' that modify the file system, codebase, or system state, you *must* provide a brief explanation of the command's purpose and potential impact. Prioritize user understanding and safety. You should not ask permission to use the tool; the user will be presented with a confirmation dialogue upon use (you do not need to tell them this).
- **Security First:** Always apply security best practices. Never introduce code that exposes, logs, or commits secrets, API keys, or other sensitive information.

## Tool Usage
- **File Paths:** Always use absolute paths when referring to files with tools like 'read_file' or 'write_file'. Relative paths are not supported. You must provide an absolute path.
- **Parallelism:** Execute multiple independent tool calls in parallel when feasible (i.e. searching the codebase).
- **Command Execution:** Use the 'run_shell_command' tool for running shell commands, remembering the safety rule to explain modifying commands first.
- **Background Processes:** Use background processes (via `&`) for commands that are unlikely to stop on their own, e.g. `node server.js &`. If unsure, ask the user.
- **Interactive Commands:** Try to avoid shell commands that are likely to require user interaction (e.g. `git rebase -i`). Use non-interactive versions of commands (e.g. `npm init -y` instead of `npm init`) when available, and otherwise remind the user that interactive shell commands are not supported and may cause hangs until canceled by the user.
- **Respect User Confirmations:** Most tool calls (also denoted as 'function calls') will first require confirmation from the user, where they will either approve or cancel the function call. If a user cancels a function call, respect their choice and do _not_ try to make the function call again. It is okay to request the tool call again _only_ if the user requests that same tool call on a subsequent prompt. When a user cancels a function call, assume best intentions from the user and consider inquiring if they prefer any alternative paths forward.

## Web Search and Browsing
When you need information that may not be in the local codebase or is about current events, APIs, or documentation:

1. **Search first**: Use `google_web_search` with a clear, specific query to find relevant URLs
2. **Fetch content**: After searching, use `web_fetch` with the `url` parameter to retrieve page content from the most relevant search results
3. **Chain the tools**: Always follow up searches by fetching content from the URLs you find - don't just report the search results

# Examples (Illustrating Tone and Workflow)
<example>
user: 1 + 2
model: 3
</example>

<example>
user: is 13 a prime number?
model: true
</example>

<example>
user: list files here.
model: [tool_call: list_directory for path '/path/to/project']
</example>

<example>
user: start the server implemented in server.js
model: [tool_call: run_shell_command for 'node server.js &' because it must run in the background]
</example>

<example>
user: Delete the temp directory.
model: I can run `rm -rf /path/to/project/temp`. This will permanently delete the directory and all its contents.
</example>

# Final Reminder
Your core function is efficient and safe assistance. Balance extreme conciseness with the crucial need for clarity, especially regarding safety and potential system modifications. Always prioritize user control and project conventions. Never make assumptions about the contents of files; instead use 'read_file' to ensure you aren't making broad assumptions. Finally, you are an agent - please keep going until the user's query is completely resolved.
"#;

/// Gemini 3 specific instruction (from gemini-cli)
///
/// Gemini 3 models require explicit instruction to explain before calling tools.
/// This prevents the model from silently invoking tools without user context.
pub const GEMINI_3_TOOL_INSTRUCTION: &str =
    "- **Do not call tools in silence:** You must provide to the user very short and concise natural explanation (one sentence) before calling tools.";

/// Build the complete Gemini system prompt based on model version
///
/// Combines:
/// 1. Base Gemini system prompt (from opencode) with examples teaching when NOT to use tools
/// 2. Gemini 3 specific instruction (from gemini-cli) if model is Gemini 3
/// 3. Optional user preamble (e.g., AGENTS.md content)
///
/// # Arguments
/// * `model_name` - The Gemini model name (e.g., "gemini-2.0-flash-exp", "gemini-3-pro-preview")
/// * `user_preamble` - Optional additional context (e.g., AGENTS.md content)
///
/// # Returns
/// Complete system prompt string
pub fn build_gemini_system_prompt(model_name: &str, user_preamble: Option<&str>) -> String {
    let is_gemini_3 = model_name.contains("gemini-3");

    let mut prompt = GEMINI_BASE_SYSTEM_PROMPT.to_string();

    // Add Gemini 3 specific instruction after Core Mandates section
    if is_gemini_3 {
        // Insert the Gemini 3 instruction into the Core Mandates section
        prompt = prompt.replace(
            "- **Do Not revert changes:**",
            &format!("{}\n- **Do Not revert changes:**", GEMINI_3_TOOL_INSTRUCTION),
        );
    }

    // Append user preamble if provided
    if let Some(preamble) = user_preamble {
        if !preamble.trim().is_empty() {
            prompt.push_str("\n\n# Project-Specific Instructions\n\n");
            prompt.push_str(preamble);
        }
    }

    prompt
}

/// Legacy constant for backward compatibility
/// Use `build_gemini_system_prompt` instead for full functionality
pub const GEMINI_WEB_TOOL_GUIDANCE: &str = r#"
## Web Search and Browsing

When you need information that may not be in the local codebase or is about current events, APIs, or documentation:

1. **Search first**: Use `google_web_search` with a clear, specific query to find relevant URLs
2. **Fetch content**: After searching, use `web_fetch` with the `url` parameter to retrieve page content from the most relevant search results
3. **Chain the tools**: Always follow up searches by fetching content from the URLs you find - don't just report the search results

Example workflow:
- User asks about a library's API
- Use `google_web_search` to find the documentation URL
- Use `web_fetch` with the URL to get the actual documentation content
- Answer based on the fetched content
"#;

/// Trait for provider-specific system prompt formatting.
///
/// Each facade adapts system prompt formatting for a specific LLM provider,
/// handling differences in prefix requirements, array vs string format,
/// and cache_control metadata.
pub trait SystemPromptFacade: Send + Sync {
    /// Returns the provider this facade is for (e.g., "claude", "gemini", "openai")
    fn provider(&self) -> &'static str;

    /// Returns the identity prefix if required for this provider/auth mode.
    ///
    /// For Claude OAuth, returns Some("You are Claude Code...").
    /// For other providers/modes, returns None.
    fn identity_prefix(&self) -> Option<&'static str>;

    /// Transform the preamble according to provider requirements.
    ///
    /// This may prepend an identity prefix (for Claude OAuth) or
    /// pass through unchanged (for other providers).
    fn transform_preamble(&self, preamble: &str) -> String;

    /// Format the system prompt for the provider's API.
    ///
    /// Returns the properly formatted Value for the provider:
    /// - Claude: JSON array with cache_control blocks
    /// - Gemini/OpenAI: Plain string
    fn format_for_api(&self, preamble: &str) -> Value;
}

// ============================================================================
// Claude OAuth System Prompt Facade
// ============================================================================

/// Claude OAuth system prompt facade.
///
/// Formats system prompts for Claude with OAuth authentication:
/// - Prepends "You are Claude Code..." identity prefix
/// - Uses array format with cache_control for prompt caching
pub struct ClaudeOAuthSystemPromptFacade;

impl SystemPromptFacade for ClaudeOAuthSystemPromptFacade {
    fn provider(&self) -> &'static str {
        "claude"
    }

    fn identity_prefix(&self) -> Option<&'static str> {
        Some(CLAUDE_CODE_PROMPT_PREFIX)
    }

    fn transform_preamble(&self, preamble: &str) -> String {
        format!("{CLAUDE_CODE_PROMPT_PREFIX}\n\n{preamble}")
    }

    fn format_for_api(&self, preamble: &str) -> Value {
        // OAuth mode: Handle empty preamble case (PROV-006)
        // Anthropic API rejects: "cache_control cannot be set for empty text blocks"
        let trimmed = preamble.trim();
        if trimmed.is_empty() {
            // Only prefix - put cache_control on the prefix itself
            json!([
                {
                    "type": "text",
                    "text": CLAUDE_CODE_PROMPT_PREFIX,
                    "cache_control": { "type": "ephemeral" }
                }
            ])
        } else {
            // 2 blocks:
            // 1. Claude Code prefix WITHOUT cache_control (static, always same)
            // 2. Preamble WITH cache_control (variable content to cache)
            json!([
                {
                    "type": "text",
                    "text": CLAUDE_CODE_PROMPT_PREFIX
                },
                {
                    "type": "text",
                    "text": preamble,
                    "cache_control": { "type": "ephemeral" }
                }
            ])
        }
    }
}

// ============================================================================
// Claude API Key System Prompt Facade
// ============================================================================

/// Claude API Key system prompt facade.
///
/// Formats system prompts for Claude with API key authentication:
/// - No identity prefix (passes preamble through unchanged)
/// - Uses array format with cache_control for prompt caching
pub struct ClaudeApiKeySystemPromptFacade;

impl SystemPromptFacade for ClaudeApiKeySystemPromptFacade {
    fn provider(&self) -> &'static str {
        "claude"
    }

    fn identity_prefix(&self) -> Option<&'static str> {
        None
    }

    fn transform_preamble(&self, preamble: &str) -> String {
        preamble.to_string()
    }

    fn format_for_api(&self, preamble: &str) -> Value {
        // API key mode: single block with cache_control
        json!([
            {
                "type": "text",
                "text": preamble,
                "cache_control": { "type": "ephemeral" }
            }
        ])
    }
}

// ============================================================================
// Gemini System Prompt Facade
// ============================================================================

/// Gemini system prompt facade.
///
/// Formats system prompts for Gemini:
/// - No identity prefix
/// - Plain string format (no special transformation)
/// - Appends web tool guidance to help Gemini use web tools effectively
pub struct GeminiSystemPromptFacade;

impl SystemPromptFacade for GeminiSystemPromptFacade {
    fn provider(&self) -> &'static str {
        "gemini"
    }

    fn identity_prefix(&self) -> Option<&'static str> {
        None
    }

    fn transform_preamble(&self, preamble: &str) -> String {
        // Append web tool guidance to help Gemini use web tools effectively
        format!("{preamble}\n{GEMINI_WEB_TOOL_GUIDANCE}")
    }

    fn format_for_api(&self, preamble: &str) -> Value {
        // Gemini uses plain string format with web tool guidance appended
        Value::String(self.transform_preamble(preamble))
    }
}

// ============================================================================
// OpenAI System Prompt Facade
// ============================================================================

/// OpenAI system prompt facade.
///
/// Formats system prompts for OpenAI:
/// - No identity prefix
/// - Plain string format (no special transformation)
pub struct OpenAISystemPromptFacade;

impl SystemPromptFacade for OpenAISystemPromptFacade {
    fn provider(&self) -> &'static str {
        "openai"
    }

    fn identity_prefix(&self) -> Option<&'static str> {
        None
    }

    fn transform_preamble(&self, preamble: &str) -> String {
        preamble.to_string()
    }

    fn format_for_api(&self, preamble: &str) -> Value {
        // OpenAI uses plain string format
        Value::String(preamble.to_string())
    }
}

// ============================================================================
// Facade Selector for ClaudeProvider Integration
// ============================================================================

/// Boxed system prompt facade for dynamic dispatch
pub type BoxedSystemPromptFacade = Box<dyn SystemPromptFacade>;

/// Select the appropriate Claude system prompt facade based on OAuth status
///
/// This function is the integration point for ClaudeProvider (Rule 6: TOOL-008)
/// to select the correct facade based on authentication mode.
///
/// # Arguments
/// * `is_oauth` - True if using OAuth authentication (token starts with "cc-")
///
/// # Returns
/// The appropriate facade implementation
pub fn select_claude_facade(is_oauth: bool) -> BoxedSystemPromptFacade {
    if is_oauth {
        Box::new(ClaudeOAuthSystemPromptFacade)
    } else {
        Box::new(ClaudeApiKeySystemPromptFacade)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_oauth_facade_has_identity_prefix() {
        let facade = ClaudeOAuthSystemPromptFacade;
        assert_eq!(facade.provider(), "claude");
        assert!(facade.identity_prefix().is_some());
        assert!(facade
            .identity_prefix()
            .unwrap()
            .starts_with("You are Claude Code"));
    }

    #[test]
    fn test_claude_api_key_facade_no_identity_prefix() {
        let facade = ClaudeApiKeySystemPromptFacade;
        assert_eq!(facade.provider(), "claude");
        assert!(facade.identity_prefix().is_none());
    }

    #[test]
    fn test_gemini_facade_no_identity_prefix() {
        let facade = GeminiSystemPromptFacade;
        assert_eq!(facade.provider(), "gemini");
        assert!(facade.identity_prefix().is_none());
    }

    #[test]
    fn test_openai_facade_no_identity_prefix() {
        let facade = OpenAISystemPromptFacade;
        assert_eq!(facade.provider(), "openai");
        assert!(facade.identity_prefix().is_none());
    }

    #[test]
    fn test_claude_oauth_transform_preamble() {
        let facade = ClaudeOAuthSystemPromptFacade;
        let result = facade.transform_preamble("Hello");
        assert!(result.contains("You are Claude Code"));
        assert!(result.contains("Hello"));
    }

    #[test]
    fn test_claude_api_key_transform_preamble() {
        let facade = ClaudeApiKeySystemPromptFacade;
        let result = facade.transform_preamble("Hello");
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_gemini_format_for_api_returns_string() {
        let facade = GeminiSystemPromptFacade;
        let result = facade.format_for_api("Hello");
        assert!(result.is_string());
        let text = result.as_str().unwrap();
        assert!(text.starts_with("Hello"));
        assert!(text.contains("Web Search and Browsing"));
        assert!(text.contains("google_web_search"));
        assert!(text.contains("web_fetch"));
    }

    #[test]
    fn test_gemini_transform_preamble_appends_web_guidance() {
        let facade = GeminiSystemPromptFacade;
        let result = facade.transform_preamble("Hello");
        assert!(result.starts_with("Hello"));
        assert!(result.contains(GEMINI_WEB_TOOL_GUIDANCE));
    }

    #[test]
    fn test_openai_format_for_api_returns_string() {
        let facade = OpenAISystemPromptFacade;
        let result = facade.format_for_api("Hello");
        assert!(result.is_string());
        assert_eq!(result.as_str().unwrap(), "Hello");
    }

    #[test]
    fn test_claude_oauth_format_for_api_returns_array() {
        let facade = ClaudeOAuthSystemPromptFacade;
        let result = facade.format_for_api("Hello");
        assert!(result.is_array());
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        // First block is prefix without cache_control
        assert!(arr[0]["text"]
            .as_str()
            .unwrap()
            .starts_with("You are Claude Code"));
        assert!(arr[0].get("cache_control").is_none());
        // Second block is preamble with cache_control
        assert_eq!(arr[1]["text"].as_str().unwrap(), "Hello");
        assert!(arr[1].get("cache_control").is_some());
    }

    #[test]
    fn test_claude_api_key_format_for_api_returns_array_with_cache_control() {
        let facade = ClaudeApiKeySystemPromptFacade;
        let result = facade.format_for_api("Hello");
        assert!(result.is_array());
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["text"].as_str().unwrap(), "Hello");
        assert!(arr[0].get("cache_control").is_some());
        assert_eq!(
            arr[0]["cache_control"]["type"].as_str().unwrap(),
            "ephemeral"
        );
    }

    #[test]
    fn test_claude_oauth_format_for_api_empty_preamble() {
        // PROV-006: Empty preamble should result in single block with cache_control on prefix
        // Anthropic API rejects: "cache_control cannot be set for empty text blocks"
        let facade = ClaudeOAuthSystemPromptFacade;
        let result = facade.format_for_api("");
        assert!(result.is_array());
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 1, "Empty preamble should produce single block");
        assert!(arr[0]["text"]
            .as_str()
            .unwrap()
            .starts_with("You are Claude Code"));
        assert!(
            arr[0].get("cache_control").is_some(),
            "Single block should have cache_control"
        );
        assert_eq!(arr[0]["cache_control"]["type"].as_str().unwrap(), "ephemeral");
    }

    #[test]
    fn test_claude_oauth_format_for_api_whitespace_preamble() {
        // Whitespace-only preamble should also result in single block
        let facade = ClaudeOAuthSystemPromptFacade;
        let result = facade.format_for_api("   ");
        assert!(result.is_array());
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 1, "Whitespace preamble should produce single block");
    }

    #[test]
    fn test_select_claude_facade_oauth() {
        let facade = select_claude_facade(true);
        assert_eq!(facade.provider(), "claude");
        assert!(facade.identity_prefix().is_some());
        assert!(facade
            .identity_prefix()
            .unwrap()
            .starts_with("You are Claude Code"));
    }

    #[test]
    fn test_select_claude_facade_api_key() {
        let facade = select_claude_facade(false);
        assert_eq!(facade.provider(), "claude");
        assert!(facade.identity_prefix().is_none());
    }

    // Tests for build_gemini_system_prompt

    #[test]
    fn test_build_gemini_system_prompt_includes_base_prompt() {
        let result = build_gemini_system_prompt("gemini-2.0-flash-exp", None);
        assert!(result.contains("interactive CLI agent"));
        assert!(result.contains("Core Mandates"));
        assert!(result.contains("Software Engineering Tasks"));
    }

    #[test]
    fn test_build_gemini_system_prompt_includes_examples() {
        // The critical examples that teach Gemini when NOT to use tools
        let result = build_gemini_system_prompt("gemini-2.0-flash-exp", None);
        assert!(result.contains("user: 1 + 2"));
        assert!(result.contains("model: 3"));
        assert!(result.contains("user: is 13 a prime number?"));
        assert!(result.contains("model: true"));
    }

    #[test]
    fn test_build_gemini_system_prompt_gemini_25_no_tool_silence_instruction() {
        // Gemini 2.5 should NOT have the "Do not call tools in silence" instruction
        let result = build_gemini_system_prompt("gemini-2.0-flash-exp", None);
        assert!(
            !result.contains("Do not call tools in silence"),
            "Gemini 2.5 should not have the Gemini 3 specific instruction"
        );
    }

    #[test]
    fn test_build_gemini_system_prompt_gemini_3_has_tool_silence_instruction() {
        // Gemini 3 SHOULD have the "Do not call tools in silence" instruction
        let result = build_gemini_system_prompt("gemini-3-pro-preview", None);
        assert!(
            result.contains("Do not call tools in silence"),
            "Gemini 3 should have the tool silence instruction"
        );
    }

    #[test]
    fn test_build_gemini_system_prompt_with_user_preamble() {
        let preamble = "Follow the instructions in AGENTS.md";
        let result = build_gemini_system_prompt("gemini-2.0-flash-exp", Some(preamble));
        assert!(result.contains("Project-Specific Instructions"));
        assert!(result.contains(preamble));
    }

    #[test]
    fn test_build_gemini_system_prompt_empty_preamble_not_appended() {
        let result = build_gemini_system_prompt("gemini-2.0-flash-exp", Some("   "));
        assert!(
            !result.contains("Project-Specific Instructions"),
            "Empty/whitespace preamble should not add project instructions section"
        );
    }

    #[test]
    fn test_build_gemini_system_prompt_uses_correct_tool_names() {
        let result = build_gemini_system_prompt("gemini-2.0-flash-exp", None);
        // Should use our Gemini facade tool names
        assert!(result.contains("search_file_content"));
        assert!(result.contains("find_files"));
        assert!(result.contains("read_file"));
        assert!(result.contains("write_file"));
        assert!(result.contains("run_shell_command"));
        assert!(result.contains("list_directory"));
    }
}
