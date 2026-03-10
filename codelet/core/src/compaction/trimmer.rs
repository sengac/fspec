//! Structurally Lossless Trimmer — Layer 0 of Hierarchical Compaction
//!
//! Deterministic, zero-LLM-cost content transformer that strips mechanical
//! bloat from StoredMessage content before the agent sees it during DAG
//! construction.
//!
//! Feature: spec/features/structurally-lossless-trimmer.feature

use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

use codelet_common::token_estimator::count_tokens;
use super::trimmer_base64::{is_base64_image, trim_base64_image};
use super::trimmer_metadata::{extract_tool_result_info, extract_tool_uses, ToolUseInfo};

/// Regex for detecting grep-style output lines (file:line:text).
static GREP_PATTERN: Lazy<Regex> = Lazy::new(|| {
    // This is a compile-time-known pattern; panic on invalid regex is intentional.
    #[allow(clippy::expect_used)]
    Regex::new(r"^[^\s:]+:\d+:").expect("GREP_PATTERN regex is valid")
});

const BASH_LINE_THRESHOLD: usize = 15;
const BASH_HEAD_LINES: usize = 10;
const BASH_TAIL_LINES: usize = 5;
const GREP_MATCH_THRESHOLD: usize = 10;

/// Structurally lossless trimmer for tool outputs and parameters.
///
/// Processes messages in order, maintaining a registry of tool_use_id → tool info
/// mappings from assistant ToolUse blocks to correlate with user ToolResult blocks.
pub struct Trimmer {
    /// Maps tool_use_id → ToolUseInfo, built as messages are processed in order.
    tool_registry: HashMap<String, ToolUseInfo>,
}

impl Trimmer {
    pub fn new() -> Self {
        Self {
            tool_registry: HashMap::new(),
        }
    }

    /// Trim a message's content based on role and metadata.
    ///
    /// Messages should be processed in order for tool_use_id correlation
    /// (assistant ToolUse blocks register mappings used by subsequent user
    /// ToolResult blocks).
    pub fn trim_message(
        &mut self,
        role: &str,
        content: &str,
        metadata: &HashMap<String, Value>,
    ) -> String {
        match role {
            "assistant" => self.trim_assistant_message(content, metadata),
            "user" => self.trim_user_message(content, metadata),
            _ => content.to_string(),
        }
    }

    /// Register tool uses and trim large Write/Edit inputs in assistant messages.
    fn trim_assistant_message(
        &mut self,
        content: &str,
        metadata: &HashMap<String, Value>,
    ) -> String {
        let tool_uses = extract_tool_uses(metadata);
        if tool_uses.is_empty() {
            return content.to_string();
        }

        for (id, info) in &tool_uses {
            self.tool_registry.insert(id.clone(), info.clone());
        }

        let mut result = content.to_string();
        for (_id, info) in &tool_uses {
            match info.name.as_str() {
                "Write" => result = trim_write_in_content(&info.input),
                "Edit" => result = trim_edit_in_content(&info.input),
                _ => {}
            }
        }
        result
    }

    /// Detect tool results in user messages and trim accordingly.
    fn trim_user_message(&self, content: &str, metadata: &HashMap<String, Value>) -> String {
        if is_base64_image(content) {
            return trim_base64_image(content, &self.get_image_source_path(metadata));
        }

        let result_info = match extract_tool_result_info(metadata) {
            Some(info) => info,
            None => return content.to_string(),
        };

        if result_info.is_image {
            return trim_base64_image(content, &self.get_image_source_path(metadata));
        }

        let tool_name = self
            .tool_registry
            .get(&result_info.tool_use_id)
            .map(|info| info.name.as_str());

        match tool_name {
            Some("Read") => {
                let path = self.lookup_file_path(&result_info.tool_use_id);
                trim_read_result(content, &path)
            }
            Some("Bash") => trim_bash_output(content, result_info.is_error),
            Some("Grep" | "AstGrep" | "Glob" | "Ls") => trim_grep_output(content),
            _ => trim_by_content_heuristics(content),
        }
    }

    /// Look up file_path from a tool_use_id's registered input.
    fn lookup_file_path(&self, tool_use_id: &str) -> String {
        self.tool_registry
            .get(tool_use_id)
            .and_then(|info| info.input.get("file_path").and_then(|v| v.as_str()))
            .unwrap_or("unknown")
            .to_string()
    }

    /// Try to determine image source path from metadata.
    fn get_image_source_path(&self, metadata: &HashMap<String, Value>) -> String {
        if let Some(info) = extract_tool_result_info(metadata) {
            if let Some(path) = self
                .tool_registry
                .get(&info.tool_use_id)
                .and_then(|t| t.input.get("file_path").and_then(|v| v.as_str()))
            {
                return path.to_string();
            }
        }
        "image".to_string()
    }
}

impl Default for Trimmer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Trimming functions for each tool type
// ============================================================================

/// Replace Write tool input content with compact persistence reference.
fn trim_write_in_content(input: &Value) -> String {
    let path = input.get("file_path").and_then(|v| v.as_str()).unwrap_or("unknown");
    let lines = input.get("content").and_then(|v| v.as_str()).unwrap_or("").lines().count();
    format!("[Write: {path}, {lines} lines — file persisted to disk]")
}

/// Replace Edit tool input content with condensed change summary.
fn trim_edit_in_content(input: &Value) -> String {
    let path = input.get("file_path").and_then(|v| v.as_str()).unwrap_or("unknown");
    let old_len = input.get("old_string").and_then(|v| v.as_str()).map(str::len).unwrap_or(0);
    let new_len = input.get("new_string").and_then(|v| v.as_str()).map(str::len).unwrap_or(0);
    format!("[Edit: {path} — replaced {old_len} chars with {new_len} chars]")
}

/// Trim a Read tool result to a compact file reference.
fn trim_read_result(content: &str, path: &str) -> String {
    let line_count = content.lines().count();
    let token_estimate = count_tokens(content);
    format!("[file: {path}, {line_count} lines, {token_estimate} tok — use Read to retrieve]")
}

/// Trim Bash tool output: keep head + tail if long, append exit code.
///
/// Exit code is derived from content ("Command failed with exit code N") or
/// inferred as 0 for success. Short output (≤15 lines) passes through unchanged.
fn trim_bash_output(content: &str, is_error: bool) -> String {
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() <= BASH_LINE_THRESHOLD {
        return content.to_string();
    }

    let exit_code = parse_exit_code_from_content(content)
        .unwrap_or(if is_error { 1 } else { 0 });
    let omitted = lines.len() - BASH_HEAD_LINES - BASH_TAIL_LINES;

    let mut result = lines[..BASH_HEAD_LINES].join("\n");
    result.push_str(&format!("\n... ({omitted} lines omitted)\n"));
    result.push_str(&lines[lines.len() - BASH_TAIL_LINES..].join("\n"));
    result.push_str(&format!("\n[exit code: {exit_code}]"));
    result
}

/// Parse exit code from Bash tool output content.
/// The Bash tool embeds exit codes as: "Command failed with exit code N\n{output}"
fn parse_exit_code_from_content(content: &str) -> Option<i32> {
    let rest = content.strip_prefix("Command failed with exit code ")?;
    let code_str: String = rest.chars().take_while(char::is_ascii_digit).collect();
    code_str.parse().ok()
}

/// Trim Grep/Search tool output: keep first N matches + count.
fn trim_grep_output(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() <= GREP_MATCH_THRESHOLD {
        return content.to_string();
    }

    let remaining = lines.len() - GREP_MATCH_THRESHOLD;
    let mut result = lines[..GREP_MATCH_THRESHOLD].join("\n");
    result.push_str(&format!("\n... ({remaining} more matches)"));
    result
}

/// Fallback: use content patterns to determine trimming strategy.
fn trim_by_content_heuristics(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();

    if lines.len() > GREP_MATCH_THRESHOLD && looks_like_grep_output(&lines) {
        return trim_grep_output(content);
    }
    if lines.len() > BASH_LINE_THRESHOLD {
        let is_error = content.starts_with("Command failed with exit code ");
        return trim_bash_output(content, is_error);
    }

    content.to_string()
}

/// Check if lines look like grep output (file:line:text pattern).
fn looks_like_grep_output(lines: &[&str]) -> bool {
    lines.iter().take(5).filter(|l| GREP_PATTERN.is_match(l)).count() >= 3
}
