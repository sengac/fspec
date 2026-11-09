//! RPC-389/RPC-399 unit tests for `chunk_wrap.rs` (moved out by TUI-130 to
//! keep that file under the 300-line ceiling).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

/// **RPC-389** boundary: a SETTLED body of exactly `COLLAPSED_LINES`
/// (8) lines is the largest body that stays fully inline — the
/// `8 > 8 == false` branch returns all lines with no indicator.
#[test]
fn settled_body_of_exactly_eight_lines_keeps_all_with_no_indicator() {
    let body: Vec<&str> = vec!["l1", "l2", "l3", "l4", "l5", "l6", "l7", "l8"];
    let (visible, indicator) = collapse_tool_body(&body, false);
    assert_eq!(visible, body);
    assert_eq!(visible.len(), 8);
    assert!(indicator.is_none());
}

/// **RPC-389** boundary: a STREAMING body of exactly
/// `STREAMING_WINDOW_SIZE` (10) lines is the largest tail that stays
/// fully visible — the `10 > 10 == false` branch returns all lines
/// with no indicator.
#[test]
fn streaming_body_of_exactly_ten_lines_keeps_all_with_no_indicator() {
    let body: Vec<&str> = vec!["l1", "l2", "l3", "l4", "l5", "l6", "l7", "l8", "l9", "l10"];
    let (visible, indicator) = collapse_tool_body(&body, true);
    assert_eq!(visible, body);
    assert_eq!(visible.len(), 10);
    assert!(indicator.is_none());
}

/// **RPC-399** boundary: a SETTLED body of 9 lines (one over the
/// threshold) collapses to the LAST 8 (end-pinned) with a `... +1 lines`
/// indicator (the single hidden line is ABOVE the window).
#[test]
fn settled_body_of_nine_lines_collapses_to_last_eight_with_indicator() {
    let body: Vec<&str> = vec!["l1", "l2", "l3", "l4", "l5", "l6", "l7", "l8", "l9"];
    let (visible, indicator) = collapse_tool_body(&body, false);
    assert_eq!(
        visible,
        vec!["l2", "l3", "l4", "l5", "l6", "l7", "l8", "l9"]
    );
    assert_eq!(
        indicator,
        Some("... +1 lines (Tab to select, Enter to view full)".to_string())
    );
}
