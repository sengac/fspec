//! Supervisor-AI interjection parsing (RPC-043, originally WATCH-020).
//!
//! Extracted from `codelet/napi/src/session_manager.rs` lines 149-255 by
//! RPC-043. The struct and parser remain napi-side because their sole
//! consumer is the napi agent_loop (which also stays napi-side per
//! RPC-043 scope).

/// Parsed interjection from supervisor AI response (WATCH-020)
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Interjection {
    /// Whether this is an urgent interjection (interrupt subordinate mid-stream)
    pub urgent: bool,
    /// The message content to inject
    pub content: String,
}

/// Parse an interjection from a supervisor AI response (WATCH-020)
///
/// Looks for [INTERJECT]...[/INTERJECT] or [CONTINUE]...[/CONTINUE] blocks.
/// Returns Some(Interjection) for valid [INTERJECT] blocks, None otherwise.
///
/// Format requirements (strict parsing):
/// - Block markers must be exact: [INTERJECT], [/INTERJECT], [CONTINUE], [/CONTINUE]
/// - Field names must be lowercase: 'urgent:', 'content:'
/// - urgent value must be 'true' or 'false' (exact)
/// - Optional whitespace allowed after colons
/// - Empty content is invalid
///
/// Example valid [INTERJECT] block:
/// ```text
/// [INTERJECT]
/// urgent: true
/// content: Security vulnerability detected
/// [/INTERJECT]
/// ```
#[allow(dead_code)]
pub(crate) fn parse_interjection(response: &str) -> Option<Interjection> {
    // Check for [CONTINUE] block first - this means no interjection
    if response.contains("[CONTINUE]") && response.contains("[/CONTINUE]") {
        tracing::debug!("Supervisor response contains [CONTINUE] block - no interjection");
        return None;
    }

    // Look for [INTERJECT] block
    let start_marker = "[INTERJECT]";
    let end_marker = "[/INTERJECT]";

    let start_idx = response.find(start_marker)?;
    let content_start = start_idx + start_marker.len();
    let end_idx = response[content_start..].find(end_marker)?;

    let block_content = &response[content_start..content_start + end_idx];

    // Parse urgent field - must be 'urgent:' followed by 'true' or 'false'
    let urgent = if let Some(urgent_line) = block_content
        .lines()
        .find(|line| line.trim().starts_with("urgent:"))
    {
        let value = urgent_line
            .trim()
            .strip_prefix("urgent:")
            .map(|s| s.trim())?;

        match value {
            "true" => true,
            "false" => false,
            _ => {
                tracing::error!(
                    "Invalid urgent value '{}' in [INTERJECT] block - must be 'true' or 'false'",
                    value
                );
                return None;
            }
        }
    } else {
        tracing::error!("Missing 'urgent:' field in [INTERJECT] block");
        return None;
    };

    // Parse content field - 'content:' followed by the message (can be multiline)
    let content_line_idx = block_content
        .lines()
        .position(|line| line.trim().starts_with("content:"))?;

    let lines: Vec<&str> = block_content.lines().collect();
    let first_content_line = lines.get(content_line_idx)?;

    // Get content after 'content:' prefix
    let first_part = first_content_line
        .trim()
        .strip_prefix("content:")
        .map(|s| s.trim_start())?;

    // Collect remaining lines as part of content (multiline support)
    let mut content_parts = vec![first_part.to_string()];
    for line in lines.iter().skip(content_line_idx + 1) {
        // Stop if we hit another field (like a malformed duplicate)
        if line.trim().starts_with("urgent:") {
            break;
        }
        content_parts.push(line.to_string());
    }

    let content = content_parts.join("\n").trim().to_string();

    if content.is_empty() {
        tracing::error!("Empty content in [INTERJECT] block");
        return None;
    }

    tracing::info!(
        "Parsed interjection: urgent={}, content_len={}",
        urgent,
        content.len()
    );

    Some(Interjection { urgent, content })
}
