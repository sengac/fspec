//! PROV-040: Truncated tool call recovery.
//!
//! Handles detection of truncated tool calls due to output token limits,
//! recovery message generation, and retry budget management.

/// PROV-040: Maximum number of consecutive truncation recovery retries per turn.
/// After this many retries, the error is reported to the user as non-recoverable.
/// Public for testing — tests must import the real constant, not hardcode the value.
pub const MAX_TRUNCATION_RETRIES: u32 = 2;

/// PROV-040: Build a structured recovery message for a truncated tool call.
///
/// Extracts the tool name from the error message and generates guidance telling
/// the model to use an alternative strategy for large content.
///
/// This function is public for testing.
pub fn build_truncation_recovery_message(error_str: &str) -> String {
    // Extract tool name from the error message: "Tool '...' received incomplete JSON arguments"
    let tool_name = error_str
        .split("Tool '")
        .nth(1)
        .and_then(|s| s.split('\'').next())
        .unwrap_or("unknown");

    // Extract partial arguments from the error: "Partial arguments: ..."
    let partial_args = error_str
        .split("Partial arguments: ")
        .nth(1)
        .unwrap_or("(not available)");

    format!(
        "Your {tool_name} tool call was truncated because it exceeded the output token limit. \
         The model hit max_tokens while generating the tool call arguments.\n\n\
         Truncated tool: {tool_name}\n\
         Partial arguments received: {partial_args}\n\n\
         IMPORTANT: You MUST use a different strategy to accomplish this task:\n\
         1. Use the Bash tool with cat/heredoc to write large files: \
            Bash(command='cat << '\"'\"'HEREDOC_EOF'\"'\"' > /path/to/file\\n...content...\\nHEREDOC_EOF')\n\
         2. Split the content into multiple smaller Write calls \
            (write the first portion, then use Edit to append the rest)\n\
         3. For very large generated content, use Bash with echo/printf commands\n\n\
         Do NOT retry the same large {tool_name} call — it will be truncated again."
    )
}

/// PROV-040: Build the error message displayed when the truncation retry budget is exhausted.
///
/// This function is public for testing.
pub fn build_truncation_budget_exhausted_message(max_retries: u32) -> String {
    format!(
        "Tool call truncated {max_retries} times — retry budget exhausted. \
         The content is too large for a single tool call. \
         Use Bash with heredoc/echo to write large files, \
         or split into multiple smaller operations."
    )
}
