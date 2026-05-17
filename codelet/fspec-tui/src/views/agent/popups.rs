//! RPC-020 — Popup orchestration helpers used by AgentView.
//!
//! Feature: spec/features/rpc020-slash-and-file-popups.feature
//!
//! Factored out of `views/agent.rs` so the orchestrator file stays
//! under the 300-LoC ceiling (rule [10] / RPC-002 invariant). All
//! helpers here are pure with respect to AgentView mutability — they
//! either inspect the joined buffer or compute the post-event mutator
//! call AgentView should make.

/// Result of inspecting the joined input buffer for popup triggers.
///
/// `OpenSlash(filter)` — buffer starts with `/`, no leading space; the
/// caller should open a slash popup with this filter.
/// `OpenFile(anchor, filter)` — the buffer contains a `@` followed by
/// zero-or-more non-space chars; `anchor` is the byte offset of `@`
/// and `filter` is the trailing text.
/// `Close` — neither trigger is satisfied; popups should be dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PopupTrigger {
    OpenSlash(String),
    OpenFile { anchor: usize, filter: String },
    Close,
}

/// Inspect the joined buffer text and decide which popup (if any)
/// should be open after the most recent edit.
///
/// Rules:
/// - Empty buffer → Close.
/// - Buffer starts with `/`. If the substring after the leading `/`
///   contains a space we treat it as an arguments-mode command and
///   Close the popup. Otherwise OpenSlash(filter) where filter is the
///   text after the leading `/`.
/// - Otherwise look for the LAST `@` in the buffer. If the substring
///   after that `@` is empty OR contains no space, OpenFile{anchor,
///   filter}. If the substring contains a space, Close.
/// - Falls through to Close.
pub fn classify_buffer(buf: &str) -> PopupTrigger {
    if buf.is_empty() {
        return PopupTrigger::Close;
    }
    if let Some(stripped) = buf.strip_prefix('/') {
        if stripped.contains(' ') {
            return PopupTrigger::Close;
        }
        return PopupTrigger::OpenSlash(stripped.to_string());
    }
    if let Some(idx) = buf.rfind('@') {
        let after = &buf[idx + 1..];
        if after.contains(' ') {
            return PopupTrigger::Close;
        }
        return PopupTrigger::OpenFile {
            anchor: idx,
            filter: after.to_string(),
        };
    }
    PopupTrigger::Close
}

/// Splice a selected file path into the joined buffer at `anchor`,
/// replacing the original `@<filter>` token with `@<path>` plus an
/// optional trailing space.
pub fn splice_file_selection(
    buf: &str,
    anchor: usize,
    filter_len: usize,
    path: &str,
    trailing_space: bool,
) -> String {
    let before = &buf[..anchor];
    let after_token_end = anchor + 1 + filter_len;
    let after = if after_token_end <= buf.len() {
        &buf[after_token_end..]
    } else {
        ""
    };
    let suffix = if trailing_space { " " } else { "" };
    format!("{before}@{path}{suffix}{after}")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn empty_buffer_closes_popups() {
        assert_eq!(classify_buffer(""), PopupTrigger::Close);
    }

    #[test]
    fn leading_slash_opens_slash_popup_with_filter() {
        assert_eq!(classify_buffer("/"), PopupTrigger::OpenSlash("".to_string()));
        assert_eq!(
            classify_buffer("/he"),
            PopupTrigger::OpenSlash("he".to_string())
        );
    }

    #[test]
    fn slash_with_space_closes_popup() {
        assert_eq!(classify_buffer("/help foo"), PopupTrigger::Close);
    }

    #[test]
    fn at_symbol_anywhere_opens_file_popup() {
        let buf = "hello @rea";
        let anchor = buf.rfind('@').unwrap();
        assert_eq!(
            classify_buffer(buf),
            PopupTrigger::OpenFile {
                anchor,
                filter: "rea".to_string()
            }
        );
    }

    #[test]
    fn space_after_at_closes_file_popup() {
        assert_eq!(classify_buffer("hello @ "), PopupTrigger::Close);
        assert_eq!(classify_buffer("hello @rea world"), PopupTrigger::Close);
    }

    #[test]
    fn last_at_wins_when_multiple_present() {
        let buf = "@first hello @sec";
        let anchor = buf.rfind('@').unwrap();
        match classify_buffer(buf) {
            PopupTrigger::OpenFile { anchor: a, filter } => {
                assert_eq!(a, anchor);
                assert_eq!(filter, "sec");
            }
            other => panic!("expected OpenFile, got {other:?}"),
        }
    }

    #[test]
    fn plain_text_closes_popups() {
        assert_eq!(classify_buffer("hello world"), PopupTrigger::Close);
    }

    #[test]
    fn splice_replaces_token_with_path_and_trailing_space() {
        let out = splice_file_selection("hello @rea", 6, 3, "README.md", true);
        assert_eq!(out, "hello @README.md ");
    }

    #[test]
    fn splice_without_trailing_space() {
        let out = splice_file_selection("hello @rea", 6, 3, "README.md", false);
        assert_eq!(out, "hello @README.md");
    }

    #[test]
    fn splice_preserves_post_token_suffix() {
        let out = splice_file_selection("hello @rea world", 6, 3, "README.md", true);
        // The original token was "@rea" followed by " world". After
        // splice the buffer reads "hello @README.md  world" with the
        // injected trailing space.
        assert_eq!(out, "hello @README.md  world");
    }
}
