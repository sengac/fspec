//! SessionSearch handler implementation — bridges codelet-tools SessionSearchTool
//! to the persistence layer in codelet-napi.
//!
//! Feature: spec/features/session-search.feature
//!
//! Creates a `SessionSearchHandler` closure that accesses MessageStore, SessionStore,
//! and BlobStore directly to execute recent/search/show actions.
//!
//! Search uses the ripgrep libraries (grep-regex, grep-matcher) — same engine as the
//! Grep tool — per Rule [6].

use chrono::{DateTime, Duration, Utc};
use codelet_tools::session_search::{
    ContextTurn, SearchMatch, SearchMatchGroup, SessionMessage, SessionSearchHandler,
    SessionSummary, DEFAULT_RECENT_COUNT, DEFAULT_SEARCH_LIMIT, MESSAGE_TRUNCATION_LIMIT,
    USER_MESSAGE_PREVIEW_LEN,
};
use codelet_tools::session_search::reassembly::{format_sections_plain, reassemble_content};
use codelet_tools::session_search::types::{SessionSearchAction, SessionSearchResult};
use grep_matcher::Matcher;
use grep_regex::RegexMatcherBuilder;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

use crate::persistence::{
    self, get_session_messages_full, is_blob_reference, load_session, StoredMessage,
};

/// Create a session search handler for a specific session.
///
/// The handler captures the project path so it can filter sessions by project.
/// It accesses the global persistence stores (SESSION_STORE, MESSAGE_STORE, etc.)
/// via the public API in `crate::persistence`.
pub fn create_handler(project_path: PathBuf) -> SessionSearchHandler {
    Arc::new(move |action: SessionSearchAction, session_id: Uuid| {
        match action {
            SessionSearchAction::Recent { count } => {
                handle_recent(&project_path, count)
            }
            SessionSearchAction::Search {
                query,
                context_turns,
                limit,
                all_projects,
                last_hours,
                last_days,
                after,
                before,
            } => handle_search(
                &project_path,
                &query,
                context_turns,
                limit,
                all_projects.unwrap_or(false),
                last_hours,
                last_days,
                after.as_deref(),
                before.as_deref(),
            ),
            SessionSearchAction::Show {
                session_id: show_id,
                user_only,
                max_turns,
            } => handle_show(session_id, show_id.as_deref(), user_only, max_turns),
        }
    })
}

// ============================================================================
// recent action
// ============================================================================

fn handle_recent(project_path: &Path, count: Option<usize>) -> SessionSearchResult {
    let n = count.unwrap_or(DEFAULT_RECENT_COUNT);

    let sessions = match persistence::list_sessions_for_project(project_path) {
        Ok(s) => s,
        Err(e) => {
            return SessionSearchResult::Error {
                message: format!("Failed to list sessions: {e}"),
            };
        }
    };

    // Sort by updated_at descending, take first n
    let mut sorted = sessions;
    sorted.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    sorted.truncate(n);

    let summaries: Vec<SessionSummary> = sorted
        .iter()
        .map(|s| {
            // Load messages to get first/last user message previews
            let (first_user, last_user) = get_user_message_previews(s);

            SessionSummary {
                session_id: s.id.to_string(),
                name: s.name.clone(),
                work_unit_id: extract_work_unit_id(s),
                created_at: s.created_at,
                updated_at: s.updated_at,
                message_count: s.messages.len(),
                project: s.project.to_string_lossy().to_string(),
                provider: non_empty_provider(&s.provider),
                first_user_message: first_user,
                last_user_message: last_user,
            }
        })
        .collect();

    SessionSearchResult::Recent { sessions: summaries }
}

// ============================================================================
// search action
// ============================================================================

#[allow(clippy::too_many_arguments)]
fn handle_search(
    project_path: &Path,
    query: &str,
    context_turns: Option<usize>,
    limit: Option<usize>,
    all_projects: bool,
    last_hours: Option<u64>,
    last_days: Option<u64>,
    after: Option<&str>,
    before: Option<&str>,
) -> SessionSearchResult {
    let max_matches = limit.unwrap_or(DEFAULT_SEARCH_LIMIT);

    // Build ripgrep matcher — same engine as the Grep tool (Rule [6])
    let matcher = match RegexMatcherBuilder::new().build(query) {
        Ok(m) => m,
        Err(e) => {
            return SessionSearchResult::Error {
                message: format!("Invalid regex pattern: {e}"),
            };
        }
    };

    // Get sessions (project-filtered or all)
    let sessions = if all_projects {
        match persistence::list_all_sessions() {
            Ok(s) => s,
            Err(e) => {
                return SessionSearchResult::Error {
                    message: format!("Failed to list sessions: {e}"),
                };
            }
        }
    } else {
        match persistence::list_sessions_for_project(project_path) {
            Ok(s) => s,
            Err(e) => {
                return SessionSearchResult::Error {
                    message: format!("Failed to list sessions: {e}"),
                };
            }
        }
    };

    // Apply time filters
    let cutoff_after = compute_after_cutoff(last_hours, last_days, after);
    let cutoff_before = compute_before_cutoff(before);

    let filtered_sessions: Vec<_> = sessions
        .into_iter()
        .filter(|s| {
            if let Some(ref after_dt) = cutoff_after {
                if s.updated_at < *after_dt {
                    return false;
                }
            }
            if let Some(ref before_dt) = cutoff_before {
                if s.updated_at > *before_dt {
                    return false;
                }
            }
            true
        })
        .collect();

    // Search across all messages in filtered sessions
    let mut groups: Vec<SearchMatchGroup> = Vec::new();
    let mut total_matches = 0;

    for session in &filtered_sessions {
        if total_matches >= max_matches {
            break;
        }

        let messages = match get_session_messages_full(session) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let mut session_matches: Vec<SearchMatch> = Vec::new();

        for (turn_index, msg) in messages.iter().enumerate() {
            if total_matches >= max_matches {
                break;
            }

            // Resolve blob content for searching
            let content = resolve_message_content(msg);

            if matcher.is_match(content.as_bytes()).unwrap_or(false) {
                // Extract a preview around the match
                let preview = extract_match_preview_ripgrep(&content, &matcher);

                session_matches.push(SearchMatch {
                    session_id: session.id.to_string(),
                    session_name: session.name.clone(),
                    role: msg.role.clone(),
                    turn_index,
                    timestamp: msg.created_at,
                    matched_content: preview,
                    project: session.project.to_string_lossy().to_string(),
                });
                total_matches += 1;
            }
        }

        if !session_matches.is_empty() {
            // Build context if requested
            let context = if let Some(ctx_turns) = context_turns {
                if ctx_turns > 0 {
                    Some(build_context_turns(&messages, &session_matches, ctx_turns))
                } else {
                    None
                }
            } else {
                None
            };

            groups.push(SearchMatchGroup {
                session_id: session.id.to_string(),
                session_name: session.name.clone(),
                project: session.project.to_string_lossy().to_string(),
                provider: non_empty_provider(&session.provider),
                message_count: messages.len(),
                matches: session_matches,
                context,
            });
        }
    }

    if total_matches == 0 {
        SessionSearchResult::NoMatches {
            query: query.to_string(),
            message: "No matches found".to_string(),
        }
    } else {
        SessionSearchResult::Search {
            query: query.to_string(),
            total_matches,
            groups,
        }
    }
}

// ============================================================================
// show action
// ============================================================================

fn handle_show(
    current_session_id: Uuid,
    show_id: Option<&str>,
    user_only: Option<bool>,
    max_turns: Option<usize>,
) -> SessionSearchResult {
    // Resolve session ID
    let target_id = match show_id {
        None | Some("current") => current_session_id,
        Some(id_str) => match Uuid::parse_str(id_str) {
            Ok(id) => id,
            Err(e) => {
                return SessionSearchResult::Error {
                    message: format!("Invalid session ID '{id_str}': {e}"),
                };
            }
        },
    };

    // Load session manifest
    let session = match load_session(target_id) {
        Ok(s) => s,
        Err(_) => {
            return SessionSearchResult::Error {
                message: format!("Session {target_id} not found"),
            };
        }
    };

    // Load all messages (ignoring compaction — we want full history for show)
    let messages = match get_session_messages_full(&session) {
        Ok(m) => m,
        Err(e) => {
            return SessionSearchResult::Error {
                message: format!("Failed to load session messages: {e}"),
            };
        }
    };

    // Apply filters and transform
    let user_only_flag = user_only.unwrap_or(false);
    let mut result_messages: Vec<SessionMessage> = Vec::new();

    for (turn_index, msg) in messages.iter().enumerate() {
        if user_only_flag && msg.role != "user" {
            continue;
        }

        // Resolve blob content
        let mut content = resolve_message_content(msg);

        // Reassemble streaming chunks for assistant messages
        if msg.role == "assistant" {
            let sections = reassemble_content(&content);
            if !sections.is_empty() {
                content = format_sections_plain(&sections);
            }
        }

        // Truncate if needed
        let truncated = content.len() > MESSAGE_TRUNCATION_LIMIT;
        if truncated {
            let boundary = floor_char_boundary(&content, MESSAGE_TRUNCATION_LIMIT);
            content = format!("{}...", &content[..boundary]);
        }

        result_messages.push(SessionMessage {
            turn_index,
            role: msg.role.clone(),
            content,
            timestamp: msg.created_at,
            truncated,
        });
    }

    // Apply max_turns limit (take from the end)
    if let Some(max) = max_turns {
        if result_messages.len() > max {
            let skip = result_messages.len() - max;
            result_messages = result_messages.into_iter().skip(skip).collect();
        }
    }

    SessionSearchResult::Session {
        session_id: target_id.to_string(),
        session_name: session.name.clone(),
        messages: result_messages,
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Get first and last user message previews for a session
fn get_user_message_previews(
    session: &crate::persistence::SessionManifest,
) -> (Option<String>, Option<String>) {
    let messages = match get_session_messages_full(session) {
        Ok(m) => m,
        Err(_) => return (None, None),
    };

    let user_messages: Vec<&StoredMessage> = messages
        .iter()
        .filter(|m| m.role == "user")
        .collect();

    let first = user_messages.first().map(|m| {
        let content = resolve_message_content(m);
        truncate_preview(&content, USER_MESSAGE_PREVIEW_LEN)
    });

    let last = if user_messages.len() > 1 {
        user_messages.last().map(|m| {
            let content = resolve_message_content(m);
            truncate_preview(&content, USER_MESSAGE_PREVIEW_LEN)
        })
    } else {
        None
    };

    (first, last)
}

/// Extract work unit ID from session name (convention: "Work on AMGR-001" or similar)
///
/// Uses the ripgrep engine (grep-regex) consistent with all regex matching in
/// this module — Rule [6].
fn extract_work_unit_id(
    session: &crate::persistence::SessionManifest,
) -> Option<String> {
    // Work unit IDs follow the pattern: PREFIX-NNN (e.g., AMGR-001, AUTH-003)
    // Ripgrep returns byte offsets — safe here since [A-Z]+-\d+ only matches ASCII,
    // so start/end are always valid char boundaries.
    let matcher = RegexMatcherBuilder::new()
        .build(r"[A-Z]+-\d+")
        .ok()?;
    let (start, end) = ripgrep_find(&matcher, &session.name)?;
    Some(session.name[start..end].to_string())
}

/// Resolve message content, handling blob references
fn resolve_message_content(msg: &StoredMessage) -> String {
    // If the content is a blob reference, try to resolve it
    if is_blob_reference(&msg.content) {
        if let Some(hash) = crate::persistence::extract_blob_hash(&msg.content) {
            if let Ok(blob_data) = persistence::get_blob(hash) {
                return String::from_utf8_lossy(&blob_data).to_string();
            }
        }
    }

    // Also check blob_refs for additional content
    if !msg.blob_refs.is_empty() {
        let mut parts = vec![msg.content.clone()];
        for blob_ref in &msg.blob_refs {
            if let Ok(blob_data) = persistence::get_blob(blob_ref) {
                parts.push(String::from_utf8_lossy(&blob_data).to_string());
            }
        }
        return parts.join("\n");
    }

    msg.content.clone()
}

/// Convert an empty provider string to None, non-empty to Some
fn non_empty_provider(provider: &str) -> Option<String> {
    if provider.is_empty() {
        None
    } else {
        Some(provider.to_string())
    }
}

/// Find the nearest char boundary at or before `target` byte index.
/// Returns 0 if no valid boundary is found before `target`.
fn floor_char_boundary(s: &str, target: usize) -> usize {
    if target >= s.len() {
        return s.len();
    }
    let mut pos = target;
    while pos > 0 && !s.is_char_boundary(pos) {
        pos -= 1;
    }
    pos
}

/// Find the nearest char boundary at or after `target` byte index.
/// Returns s.len() if no valid boundary is found after `target`.
fn ceil_char_boundary(s: &str, target: usize) -> usize {
    if target >= s.len() {
        return s.len();
    }
    let mut pos = target;
    while pos < s.len() && !s.is_char_boundary(pos) {
        pos += 1;
    }
    pos
}

/// Truncate content to a max length, adding "..." if truncated.
/// Safe with multi-byte UTF-8 characters.
fn truncate_preview(content: &str, max_len: usize) -> String {
    if content.len() <= max_len {
        content.to_string()
    } else {
        let boundary = floor_char_boundary(content, max_len);
        format!("{}...", &content[..boundary])
    }
}

/// Extract a preview around the first ripgrep match.
/// Safe with multi-byte UTF-8 — all byte offsets are snapped to char boundaries.
fn extract_match_preview_ripgrep(content: &str, matcher: &grep_regex::RegexMatcher) -> String {
    const PREVIEW_CONTEXT: usize = 100;
    const MAX_PREVIEW: usize = 300;

    if let Ok(Some(m)) = matcher.find(content.as_bytes()) {
        let start = floor_char_boundary(content, m.start().saturating_sub(PREVIEW_CONTEXT));
        let end = ceil_char_boundary(content, (m.end() + PREVIEW_CONTEXT).min(content.len()));

        let mut preview = String::new();
        if start > 0 {
            preview.push_str("...");
        }
        preview.push_str(&content[start..end]);
        if end < content.len() {
            preview.push_str("...");
        }

        if preview.len() > MAX_PREVIEW {
            let boundary = floor_char_boundary(&preview, MAX_PREVIEW);
            format!("{}...", &preview[..boundary])
        } else {
            preview
        }
    } else {
        truncate_preview(content, MAX_PREVIEW)
    }
}

/// Build context turns around matched turns within a session
fn build_context_turns(
    messages: &[StoredMessage],
    matches: &[SearchMatch],
    context_turns: usize,
) -> Vec<ContextTurn> {
    let match_indices: Vec<usize> = matches.iter().map(|m| m.turn_index).collect();

    // Collect all turn indices that should be included
    let mut included: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    for &idx in &match_indices {
        let start = idx.saturating_sub(context_turns);
        let end = (idx + context_turns).min(messages.len().saturating_sub(1));
        for i in start..=end {
            included.insert(i);
        }
    }

    included
        .iter()
        .filter_map(|&i| {
            messages.get(i).map(|msg| {
                let content = resolve_message_content(msg);
                let truncated = truncate_preview(&content, MESSAGE_TRUNCATION_LIMIT);

                ContextTurn {
                    turn_index: i,
                    role: msg.role.clone(),
                    content: truncated,
                    is_match: match_indices.contains(&i),
                }
            })
        })
        .collect()
}

/// Compute the "after" cutoff from relative or absolute time parameters
fn compute_after_cutoff(
    last_hours: Option<u64>,
    last_days: Option<u64>,
    after: Option<&str>,
) -> Option<DateTime<Utc>> {
    // Relative takes priority
    if let Some(hours) = last_hours {
        return Some(Utc::now() - Duration::hours(hours as i64));
    }
    if let Some(days) = last_days {
        return Some(Utc::now() - Duration::days(days as i64));
    }
    // Absolute
    if let Some(after_str) = after {
        return after_str.parse::<DateTime<Utc>>().ok();
    }
    None
}

/// Compute the "before" cutoff from absolute time parameter
fn compute_before_cutoff(before: Option<&str>) -> Option<DateTime<Utc>> {
    before.and_then(|s| s.parse::<DateTime<Utc>>().ok())
}

/// Build a ripgrep matcher from a query string.
///
/// Public for testing — this is the exact same engine used by the Grep tool.
pub fn build_ripgrep_matcher(query: &str) -> Result<grep_regex::RegexMatcher, String> {
    RegexMatcherBuilder::new()
        .build(query)
        .map_err(|e| format!("Invalid regex pattern: {e}"))
}

/// Check if content matches using the ripgrep matcher.
///
/// Public for testing — proves we use grep-regex/grep-matcher, not regex::Regex.
pub fn ripgrep_is_match(matcher: &grep_regex::RegexMatcher, content: &str) -> bool {
    matcher.is_match(content.as_bytes()).unwrap_or(false)
}

/// Find the first match in content using the ripgrep matcher.
///
/// Returns (start, end) byte offsets or None.
pub fn ripgrep_find(
    matcher: &grep_regex::RegexMatcher,
    content: &str,
) -> Option<(usize, usize)> {
    matcher
        .find(content.as_bytes())
        .ok()
        .flatten()
        .map(|m| (m.start(), m.end()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    /// Scenario: Search uses ripgrep regex matching
    ///
    /// This test proves the handler uses grep-regex (ripgrep) for matching,
    /// NOT the regex crate directly. The build_ripgrep_matcher and ripgrep_is_match
    /// functions are the exact code paths used by handle_search.
    // @step Given a session contains messages with "DeepSearch", "deep_search", and "DEEPSEARCH"
    // @step When the agent calls SessionSearch with action "search" and query "(?i)deep.?search"
    // @step Then all three variations are matched
    #[test]
    fn test_ripgrep_matcher_case_insensitive_regex() {
        let matcher = build_ripgrep_matcher("(?i)deep.?search").unwrap();

        assert!(ripgrep_is_match(&matcher, "Using DeepSearch tool"));
        assert!(ripgrep_is_match(&matcher, "deep_search function"));
        assert!(ripgrep_is_match(&matcher, "DEEPSEARCH results"));
        assert!(!ripgrep_is_match(&matcher, "nothing here"));
    }

    #[test]
    fn test_ripgrep_matcher_literal_query() {
        let matcher = build_ripgrep_matcher("RLM-001").unwrap();

        assert!(ripgrep_is_match(&matcher, "Working on RLM-001 today"));
        assert!(ripgrep_is_match(&matcher, "RLM-001"));
        assert!(!ripgrep_is_match(&matcher, "RLM-002 is different"));
    }

    #[test]
    fn test_ripgrep_find_returns_offsets() {
        let matcher = build_ripgrep_matcher("hello").unwrap();
        let (start, end) = ripgrep_find(&matcher, "say hello world").unwrap();
        assert_eq!(start, 4);
        assert_eq!(end, 9);
    }

    #[test]
    fn test_ripgrep_find_no_match() {
        let matcher = build_ripgrep_matcher("hello").unwrap();
        assert!(ripgrep_find(&matcher, "goodbye world").is_none());
    }

    #[test]
    fn test_ripgrep_invalid_regex_returns_error() {
        let result = build_ripgrep_matcher("[invalid");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid regex"));
    }

    #[test]
    fn test_extract_match_preview_with_ripgrep() {
        let matcher = build_ripgrep_matcher("compaction").unwrap();
        let content = "x".repeat(200) + "compaction" + &"y".repeat(200);
        let preview = extract_match_preview_ripgrep(&content, &matcher);

        assert!(preview.contains("compaction"));
        // Preview should be truncated, not the full 410 chars
        assert!(preview.len() <= 310); // 300 max + "..." prefixes
    }

    #[test]
    fn test_work_unit_id_extraction_via_ripgrep() {
        let matcher = build_ripgrep_matcher(r"[A-Z]+-\d+").unwrap();

        let (start, end) = ripgrep_find(&matcher, "Work on AMGR-001").unwrap();
        assert_eq!(&"Work on AMGR-001"[start..end], "AMGR-001");

        let (start, end) = ripgrep_find(&matcher, "Session AUTH-003 fix").unwrap();
        assert_eq!(&"Session AUTH-003 fix"[start..end], "AUTH-003");

        assert!(ripgrep_find(&matcher, "no work unit here").is_none());
    }

    #[test]
    fn test_floor_char_boundary_ascii() {
        let s = "hello world";
        assert_eq!(floor_char_boundary(s, 5), 5);
        assert_eq!(floor_char_boundary(s, 0), 0);
        assert_eq!(floor_char_boundary(s, 100), s.len());
    }

    #[test]
    fn test_floor_char_boundary_multibyte() {
        // ✅ is 3 bytes (E2 9C 85), 🔴 is 4 bytes (F0 9F 94 B4)
        let s = "a✅b🔴c";
        // a=0, ✅=1..4, b=4, 🔴=5..9, c=9
        assert_eq!(floor_char_boundary(s, 0), 0); // 'a'
        assert_eq!(floor_char_boundary(s, 1), 1); // start of ✅
        assert_eq!(floor_char_boundary(s, 2), 1); // inside ✅ → back to 1
        assert_eq!(floor_char_boundary(s, 3), 1); // inside ✅ → back to 1
        assert_eq!(floor_char_boundary(s, 4), 4); // 'b'
        assert_eq!(floor_char_boundary(s, 5), 5); // start of 🔴
        assert_eq!(floor_char_boundary(s, 6), 5); // inside 🔴 → back to 5
        assert_eq!(floor_char_boundary(s, 7), 5); // inside 🔴
        assert_eq!(floor_char_boundary(s, 8), 5); // inside 🔴
        assert_eq!(floor_char_boundary(s, 9), 9); // 'c'
    }

    #[test]
    fn test_ceil_char_boundary_multibyte() {
        let s = "a✅b🔴c";
        assert_eq!(ceil_char_boundary(s, 0), 0);
        assert_eq!(ceil_char_boundary(s, 1), 1); // start of ✅ — already valid
        assert_eq!(ceil_char_boundary(s, 2), 4); // inside ✅ → forward to 'b'
        assert_eq!(ceil_char_boundary(s, 3), 4);
        assert_eq!(ceil_char_boundary(s, 6), 9); // inside 🔴 → forward to 'c'
        assert_eq!(ceil_char_boundary(s, 100), s.len());
    }

    #[test]
    fn test_truncate_preview_multibyte_safe() {
        // 🔴 is 4 bytes. Create string where max_len lands inside an emoji.
        let s = "abc🔴def";
        // a=0, b=1, c=2, 🔴=3..7, d=7, e=8, f=9
        let result = truncate_preview(s, 5); // byte 5 is inside 🔴
        assert!(result.ends_with("..."));
        // Should truncate to "abc" (boundary at 3) not panic
        assert!(result.starts_with("abc"));
    }

    #[test]
    fn test_extract_match_preview_multibyte() {
        // Content with emojis around the match
        let matcher = build_ripgrep_matcher("target").unwrap();
        let content = format!("{}target{}", "🔴".repeat(30), "✅".repeat(30));
        let preview = extract_match_preview_ripgrep(&content, &matcher);
        // Should not panic and should contain the match
        assert!(preview.contains("target"));
    }

    #[test]
    fn test_truncate_preview_all_multibyte() {
        // String of only 4-byte emojis
        let s = "🔴🔴🔴🔴🔴"; // 20 bytes
        let result = truncate_preview(s, 6); // lands inside 2nd emoji
        assert!(result.ends_with("..."));
        assert!(result.starts_with("🔴")); // got at least the first emoji
    }
}
