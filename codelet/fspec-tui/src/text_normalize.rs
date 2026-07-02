//! Shared line-ending normalization for pasted text.
//!
//! Feature: spec/features/agent-input-bracketed-paste-routing.feature
//!
//! RPC-403 review promotion: `normalize_line_endings` originally lived
//! `pub(super)` inside `views::agent::multiline_input_paste`, which
//! forced `components::hitl_dialog` to re-implement the two `replace`
//! calls inline and left `components::role_dialog` skipping lone-`\r`
//! normalization entirely. Every paste sink (agent input, HITL free-
//! text row, role-dialog textarea) now shares this single definition
//! so the normalization contract cannot drift.

/// Normalize Windows `\r\n` (CRLF) and lone `\r` (CR) line endings to
/// `\n` (LF).
///
/// Ordering matters: `\r\n` is collapsed first so a CRLF pair can
/// never be double-converted into `\n\n`.
pub(crate) fn normalize_line_endings(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

#[cfg(test)]
mod tests {
    use super::normalize_line_endings;

    #[test]
    fn crlf_lone_cr_and_lf_all_collapse_to_lf() {
        // CRLF pairs collapse to a single LF (never \n\n).
        assert_eq!(normalize_line_endings("a\r\nb"), "a\nb");
        // Lone CR becomes LF.
        assert_eq!(normalize_line_endings("a\rb"), "a\nb");
        // Existing LF is untouched.
        assert_eq!(normalize_line_endings("a\nb"), "a\nb");
        // Mixed input: every variant lands on exactly one LF each.
        assert_eq!(normalize_line_endings("a\r\nb\rc\nd"), "a\nb\nc\nd");
        // No line endings — string passes through byte-for-byte.
        assert_eq!(normalize_line_endings("plain"), "plain");
        // CR CR LF: first CR is lone (-> LF), then CRLF pair (-> LF).
        assert_eq!(normalize_line_endings("a\r\r\nb"), "a\n\nb");
    }
}
