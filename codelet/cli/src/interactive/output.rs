//! Stream output abstraction for CLI vs NAPI rendering
//!
//! This module provides a trait-based abstraction that allows the stream loop
//! to work with both CLI output (stdout) and NAPI output (JavaScript callbacks).
//!
//! The trait separates I/O concerns from message history management, enabling
//! code reuse between codelet-cli and codelet-napi.

use std::io::Write;

/// Token usage information for streaming updates
#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
}

/// Stream output handler trait
///
/// Implementations handle rendering stream events to their target output:
/// - CliOutput: Prints to stdout with terminal formatting
/// - NapiOutput: Sends StreamChunk via ThreadsafeFunction callback
pub trait StreamOutput: Send + Sync {
    /// Emit streaming text chunk
    fn emit_text(&self, text: &str);

    /// Emit tool call notification
    fn emit_tool_call(&self, id: &str, name: &str, args: &serde_json::Value);

    /// Emit tool result
    fn emit_tool_result(&self, id: &str, content: &str, is_error: bool);

    /// Emit stream completion
    fn emit_done(&self);

    /// Emit error
    fn emit_error(&self, error: &str);

    /// Emit interruption notification
    fn emit_interrupted(&self, queued_inputs: &[String]);

    /// Emit status message (e.g., compaction notifications)
    fn emit_status(&self, message: &str);

    /// Emit token usage update (real-time token tracking)
    fn emit_tokens(&self, tokens: &TokenInfo);
}

/// CLI output implementation - prints to stdout
pub struct CliOutput;

impl StreamOutput for CliOutput {
    fn emit_text(&self, text: &str) {
        // Replace \n with \r\n for proper terminal display in raw mode
        let display_text = text.replace('\n', "\r\n");
        print!("{display_text}");
        std::io::stdout().flush().ok();
    }

    fn emit_tool_call(&self, _id: &str, name: &str, args: &serde_json::Value) {
        // Display tool name
        print!("\r\n[Planning to use tool: {name}]");

        // Display arguments
        if let Some(obj) = args.as_object() {
            if !obj.is_empty() {
                for (key, value) in obj.iter() {
                    let formatted_value = match value {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::Bool(b) => b.to_string(),
                        serde_json::Value::Array(_) => format!("{value}"),
                        serde_json::Value::Object(_) => format!("{value}"),
                        serde_json::Value::Null => "null".to_string(),
                    };
                    print!("\r\n  {key}: {formatted_value}");
                }
            }
        }
        println!("\r\n");
        std::io::stdout().flush().ok();
    }

    fn emit_tool_result(&self, _id: &str, content: &str, _is_error: bool) {
        // Truncate if too long
        const MAX_PREVIEW_LENGTH: usize = 500;
        let preview = if content.len() > MAX_PREVIEW_LENGTH {
            format!("{}...", &content[..MAX_PREVIEW_LENGTH])
        } else {
            content.to_string()
        };

        // Indent each line and format for raw mode
        let indented_lines: Vec<String> = preview.lines().map(|line| format!("  {line}")).collect();
        let formatted_preview = indented_lines.join("\r\n");

        print!("\r\n[Tool result preview]\r\n-------\r\n{formatted_preview}\r\n-------\r\n");
        std::io::stdout().flush().ok();
    }

    fn emit_done(&self) {
        // CLI doesn't need explicit done signal
    }

    fn emit_error(&self, error: &str) {
        eprintln!("\r\nError: {error}");
    }

    fn emit_interrupted(&self, queued_inputs: &[String]) {
        // Use \r\n for raw mode compatibility
        print!("\r\n⚠️ Agent interrupted\r\n");
        if queued_inputs.is_empty() {
            print!("Queued inputs: (none)\r\n");
        } else {
            let joined = queued_inputs.join("\r\n\r\n");
            print!("Queued inputs:\r\n{joined}\r\n");
        }
        std::io::stdout().flush().ok();
    }

    fn emit_status(&self, message: &str) {
        // Use \r\n for raw mode compatibility
        let formatted = message.replace('\n', "\r\n");
        print!("{formatted}\r\n");
        std::io::stdout().flush().ok();
    }

    fn emit_tokens(&self, _tokens: &TokenInfo) {
        // CLI doesn't display real-time token updates (shown in status line instead)
    }
}
