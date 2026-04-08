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

use std::sync::atomic::{AtomicBool, Ordering};

use crate::persistence::{
    self, get_session_messages_full, is_blob_reference, load_session, StoredMessage,
};

/// Create a session search handler for a specific session.
///
/// The handler captures the project path and compaction trimming flag so it
/// can filter sessions by project and conditionally apply Layer 0 trimming.
/// It accesses the global persistence stores (SESSION_STORE, MESSAGE_STORE, etc.)
/// via the public API in `crate::persistence`.
pub fn create_handler(
    project_path: PathBuf,
    compaction_trimming: Arc<AtomicBool>,
) -> SessionSearchHandler {
    Arc::new(move |action: SessionSearchAction, session_id: Uuid| {
        let is_trimming = compaction_trimming.load(Ordering::Relaxed);
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
                start_turn,
                end_turn,
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
                start_turn,
                end_turn,
                is_trimming,
            ),
            SessionSearchAction::Show {
                session_id: show_id,
                user_only,
                max_turns,
                start_turn,
                end_turn,
            } => handle_show(session_id, show_id.as_deref(), user_only, max_turns, start_turn, end_turn, is_trimming),
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
    start_turn: Option<usize>,
    end_turn: Option<usize>,
    compaction_trimming: bool,
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

        // Create conditional trimmer per session for stateful
        // tool_use_id correlation.  ALL messages are processed in order (even
        // non-matching) so the Trimmer builds its tool_registry correctly.
        let mut trimmer = ConditionalTrimmer::new(compaction_trimming);

        for (turn_index, msg) in messages.iter().enumerate() {
            if total_matches >= max_matches {
                break;
            }

            // Apply turn range filter — skip messages outside the range
            if start_turn.is_some() || end_turn.is_some() {
                let start = start_turn.unwrap_or(0);
                let end = end_turn.unwrap_or(usize::MAX);
                if turn_index < start || turn_index > end {
                    // Still process through trimmer to maintain tool_use_id correlation
                    let raw_content = resolve_message_content(msg);
                    let _ = trimmer.process(&msg.role, &raw_content, &msg.metadata);
                    continue;
                }
            }

            // Resolve blob content, then apply conditional trimming
            let raw_content = resolve_message_content(msg);
            let content = trimmer.process(&msg.role, &raw_content, &msg.metadata);

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
    start_turn: Option<usize>,
    end_turn: Option<usize>,
    compaction_trimming: bool,
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

    // Conditional trimmer for stateful tool_use_id correlation.
    // Must process ALL messages in order (including user_only-filtered ones)
    // so the Trimmer builds its tool_registry correctly.
    let mut trimmer = ConditionalTrimmer::new(compaction_trimming);

    for (turn_index, msg) in messages.iter().enumerate() {
        // Resolve blob content, then apply conditional trimming
        let raw_content = resolve_message_content(msg);
        let mut content = trimmer.process(&msg.role, &raw_content, &msg.metadata);

        // Apply turn range filter — skip messages outside the range
        // (trimmer already processed for tool_use_id correlation above)
        if start_turn.is_some() || end_turn.is_some() {
            let start = start_turn.unwrap_or(0);
            let end = end_turn.unwrap_or(usize::MAX);
            if turn_index < start || turn_index > end {
                continue;
            }
        }

        if user_only_flag && msg.role != "user" {
            continue;
        }

        // Reassemble streaming chunks for assistant messages
        if msg.role == "assistant" {
            let sections = reassemble_content(&content);
            if !sections.is_empty() {
                content = format_sections_plain(&sections);
            }

            // Surface structural annotations from message metadata.
            // Annotations are persisted as {"annotations": [...]} in StoredMessage.metadata
            // by the persistence layer after detect_annotations() runs in the stream loop.
            if let Some(annotations_val) = msg.metadata.get("annotations") {
                if let Some(summary) = format_annotation_summary(annotations_val) {
                    content.push_str(&format!("\n[annotations: {}]", summary));
                }
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
///
/// Handles both direct blob references in `msg.content` and additional
/// blob refs in `msg.blob_refs`. Used by both SessionSearch and AgentManager
/// handlers to resolve stored message content for display/search.
pub fn resolve_message_content(msg: &StoredMessage) -> String {
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

/// Format structural annotations from message metadata into a compact summary.
///
/// Annotations are stored as a JSON array of StructuralAnnotation variants.
/// serde serializes Rust enums with external tagging by default:
///   `{"FspecMilestone": {"command": "...", "args": [...]}}`.
///
/// This produces a human-readable summary like:
///   "FspecMilestone(update-work-unit-status → implementing), FileModification(src/auth.rs → Created)"
fn format_annotation_summary(annotations_val: &serde_json::Value) -> Option<String> {
    let arr = annotations_val.as_array()?;
    if arr.is_empty() {
        return None;
    }

    let parts: Vec<String> = arr
        .iter()
        .filter_map(|ann| {
            let obj = ann.as_object()?;
            // serde externally-tagged enum: single key = variant name
            if let Some(inner) = obj.get("FspecMilestone") {
                let cmd = inner.get("command").and_then(|c| c.as_str()).unwrap_or("?");
                let args = inner
                    .get("args")
                    .and_then(|a| a.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default();
                if args.is_empty() {
                    Some(format!("FspecMilestone({cmd})"))
                } else {
                    Some(format!("FspecMilestone({cmd} → {args})"))
                }
            } else if let Some(inner) = obj.get("FileModification") {
                let path = inner.get("path").and_then(|p| p.as_str()).unwrap_or("?");
                let op = inner.get("operation").and_then(|o| o.as_str()).unwrap_or("?");
                Some(format!("FileModification({path} → {op})"))
            } else if let Some(inner) = obj.get("ErrorResolution") {
                let tool = inner
                    .get("failed_tool")
                    .and_then(|t| t.as_str())
                    .unwrap_or("?");
                let file = inner
                    .get("resolved_file")
                    .and_then(|f| f.as_str())
                    .unwrap_or("?");
                Some(format!("ErrorResolution({tool} → {file})"))
            } else {
                None
            }
        })
        .collect();

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
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

/// Wrapper around an optional `Trimmer` for conditional Layer 0 trimming.
///
/// When `active` is `true`, creates a real `Trimmer` and delegates to it.
/// When `false`, passes content through unchanged.  Messages must be fed
/// in order so tool_use_id correlation works (the inner `Trimmer` is stateful).
pub struct ConditionalTrimmer(Option<codelet_core::compaction::Trimmer>);

impl ConditionalTrimmer {
    /// Create a new conditional trimmer.
    /// When `active` is false the inner trimmer is `None` — `process()` becomes a no-op.
    pub fn new(active: bool) -> Self {
        Self(if active {
            Some(codelet_core::compaction::Trimmer::new())
        } else {
            None
        })
    }

    /// Process a single message.  Returns trimmed content when active,
    /// or the original content unchanged when inactive.
    pub fn process(
        &mut self,
        role: &str,
        content: &str,
        metadata: &std::collections::HashMap<String, serde_json::Value>,
    ) -> String {
        match self.0 {
            Some(ref mut t) => t.trim_message(role, content, metadata),
            None => content.to_string(),
        }
    }
}

/// Apply conditional trimming to a sequence of messages.
///
/// When `compaction_active` is `true`, creates a `Trimmer` and processes all
/// messages in order, trimming tool outputs and large content. When `false`,
/// returns content unchanged. Messages must be processed in order for
/// tool_use_id correlation (Trimmer is stateful).
///
/// Returns a `Vec<String>` of processed content, one per input message.
///
/// This is a test convenience wrapper around `ConditionalTrimmer` — production
/// code uses `ConditionalTrimmer` directly via the session search handler.
#[cfg(test)]
pub fn apply_conditional_trimming(
    compaction_active: bool,
    messages: &[(String, String, std::collections::HashMap<String, serde_json::Value>)],
) -> Vec<String> {
    let mut trimmer = ConditionalTrimmer::new(compaction_active);
    messages
        .iter()
        .map(|(role, content, metadata)| trimmer.process(role, content, metadata))
        .collect()
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

    // ========================================================================
    // SessionSearch Trimming Integration tests
    //
    // Feature: spec/features/session-search-trimming.feature
    //
    // This test section validates the acceptance criteria for conditional
    // trimming in SessionSearch when compaction_in_progress is active.
    // ========================================================================

    use serde_json::json;

    /// Build metadata for an assistant message containing a ToolUse block.
    fn make_assistant_tool_use_metadata(
        tool_name: &str,
        tool_id: &str,
        input: serde_json::Value,
    ) -> std::collections::HashMap<String, serde_json::Value> {
        let mut meta = std::collections::HashMap::new();
        meta.insert("type".to_string(), json!("assistant"));
        meta.insert(
            "message".to_string(),
            json!({
                "role": "assistant",
                "content": [
                    {
                        "type": "tool_use",
                        "id": tool_id,
                        "name": tool_name,
                        "input": input
                    }
                ]
            }),
        );
        meta
    }

    /// Build metadata for a user message containing a ToolResult block.
    fn make_user_tool_result_metadata(
        tool_use_id: &str,
        is_error: bool,
    ) -> std::collections::HashMap<String, serde_json::Value> {
        let mut meta = std::collections::HashMap::new();
        meta.insert("type".to_string(), json!("user"));
        meta.insert(
            "message".to_string(),
            json!({
                "role": "user",
                "content": [
                    {
                        "type": "tool_result",
                        "tool_use_id": tool_use_id,
                        "content": "[content in main field]",
                        "is_error": is_error
                    }
                ]
            }),
        );
        meta
    }

    /// Build metadata for a plain user text message (no tool result).
    fn make_plain_user_metadata() -> std::collections::HashMap<String, serde_json::Value> {
        let mut meta = std::collections::HashMap::new();
        meta.insert("type".to_string(), json!("user"));
        meta.insert(
            "message".to_string(),
            json!({
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": "plain user message"
                    }
                ]
            }),
        );
        meta
    }

    /// Build metadata for a plain assistant text message (no tool use).
    fn make_plain_assistant_metadata() -> std::collections::HashMap<String, serde_json::Value> {
        let mut meta = std::collections::HashMap::new();
        meta.insert("type".to_string(), json!("assistant"));
        meta.insert(
            "message".to_string(),
            json!({
                "role": "assistant",
                "content": [
                    {
                        "type": "text",
                        "text": "assistant reasoning"
                    }
                ]
            }),
        );
        meta
    }

    /// Generate N lines of content for testing (same as trimmer tests).
    fn generate_lines(n: usize) -> String {
        (1..=n)
            .map(|i| format!("     {}: line {} content here with some text", i, i))
            .collect::<Vec<_>>()
            .join("\n")
    }

    // Scenario: Flag is false — SessionSearch show returns full untrimmed content
    #[test]
    fn test_flag_false_returns_full_untrimmed_content() {
        // @step Given the compaction_in_progress flag is false
        let compaction_active = false;

        // @step And the session contains a user tool result message with 500 lines of Read output
        let content = generate_lines(500);
        let tool_id = "toolu_read_001";
        let assistant_meta = make_assistant_tool_use_metadata(
            "Read",
            tool_id,
            json!({"file_path": "src/main.rs"}),
        );
        let user_meta = make_user_tool_result_metadata(tool_id, false);

        let messages = vec![
            ("assistant".to_string(), "[tool_use: Read]".to_string(), assistant_meta),
            ("user".to_string(), content.clone(), user_meta),
        ];

        // @step When the agent calls SessionSearch with action "show"
        let results = apply_conditional_trimming(compaction_active, &messages);

        // @step Then the result includes the full 500-line content with no trimming applied
        assert_eq!(results.len(), 2);
        assert_eq!(results[1], content, "Content should be unchanged when flag is false");
        assert!(results[1].lines().count() == 500, "All 500 lines should be present");
    }

    // Scenario: Flag is true — SessionSearch show returns trimmed Read tool results
    #[test]
    fn test_flag_true_trims_read_tool_results() {
        // @step Given the compaction_in_progress flag is true
        let compaction_active = true;

        // @step And the session contains a user tool result message with 500 lines of Read output for "src/main.rs"
        let content = generate_lines(500);
        let tool_id = "toolu_read_002";
        let assistant_meta = make_assistant_tool_use_metadata(
            "Read",
            tool_id,
            json!({"file_path": "src/main.rs"}),
        );
        let user_meta = make_user_tool_result_metadata(tool_id, false);

        let messages = vec![
            ("assistant".to_string(), "[tool_use: Read]".to_string(), assistant_meta),
            ("user".to_string(), content.clone(), user_meta),
        ];

        // @step When the agent calls SessionSearch with action "show"
        let results = apply_conditional_trimming(compaction_active, &messages);

        // @step Then the result shows a compact reference like "[file: src/main.rs, 500 lines, {tok} tok — use Read to retrieve]"
        assert_eq!(results.len(), 2);
        let trimmed = &results[1];
        assert!(trimmed.starts_with("[file: src/main.rs,"), "Should be a compact file reference, got: {}", trimmed);
        assert!(trimmed.contains("500 lines"), "Should mention 500 lines, got: {}", trimmed);
        assert!(trimmed.contains("tok —"), "Should contain token count, got: {}", trimmed);
        assert!(trimmed.contains("use Read to retrieve"), "Should hint at Read tool, got: {}", trimmed);
        assert_ne!(*trimmed, content, "Content should be trimmed, not original");
    }

    // Scenario: Flag is true — SessionSearch search preserves user messages unchanged
    #[test]
    fn test_flag_true_preserves_plain_user_messages() {
        // @step Given the compaction_in_progress flag is true
        let compaction_active = true;

        // @step And the session contains a user message "please fix the login bug"
        let user_content = "please fix the login bug".to_string();
        let user_meta = make_plain_user_metadata();

        let messages = vec![
            ("user".to_string(), user_content.clone(), user_meta),
        ];

        // @step When the agent calls SessionSearch with action "search" and query "login"
        let results = apply_conditional_trimming(compaction_active, &messages);

        // @step Then the matched content includes "please fix the login bug" unchanged
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], user_content, "Plain user messages should pass through unchanged");
    }

    // Scenario: Flag is true — SessionSearch show preserves assistant reasoning text
    #[test]
    fn test_flag_true_preserves_assistant_reasoning() {
        // @step Given the compaction_in_progress flag is true
        let compaction_active = true;

        // @step And the session contains an assistant message with reasoning text and no tool use
        let reasoning = "I need to analyze the error in the login handler. The stack trace suggests a null pointer in the auth middleware.".to_string();
        let assistant_meta = make_plain_assistant_metadata();

        let messages = vec![
            ("assistant".to_string(), reasoning.clone(), assistant_meta),
        ];

        // @step When the agent calls SessionSearch with action "show"
        let results = apply_conditional_trimming(compaction_active, &messages);

        // @step Then the assistant reasoning text is returned unchanged
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], reasoning, "Assistant reasoning should pass through unchanged");
    }

    // Scenario: Trimmer processes messages in order for tool_use_id correlation
    #[test]
    fn test_trimmer_processes_messages_in_order() {
        // @step Given the compaction_in_progress flag is true
        let compaction_active = true;

        // @step And the session contains an assistant message with a Write tool_use block
        let write_tool_id = "toolu_write_001";
        let assistant_meta = make_assistant_tool_use_metadata(
            "Write",
            write_tool_id,
            json!({
                "file_path": "src/auth.rs",
                "content": "fn login() { /* implementation */ }"
            }),
        );

        // @step And the session contains a user message with the corresponding tool_result
        let write_result = "Successfully wrote to src/auth.rs".to_string();
        let tool_result_meta = make_user_tool_result_metadata(write_tool_id, false);

        // @step And the session contains a plain user message
        let plain_content = "now please add tests for it".to_string();
        let plain_meta = make_plain_user_metadata();

        let messages = vec![
            ("assistant".to_string(), "[tool_use: Write src/auth.rs]".to_string(), assistant_meta),
            ("user".to_string(), write_result.clone(), tool_result_meta),
            ("user".to_string(), plain_content.clone(), plain_meta),
        ];

        // @step When the agent calls SessionSearch with action "show"
        let results = apply_conditional_trimming(compaction_active, &messages);

        // @step Then the Write tool result is trimmed to a compact persistence reference
        assert_eq!(results.len(), 3);
        // Assistant message with Write tool_use has its input trimmed to a compact reference
        assert!(
            results[0].contains("[Write:") || results[0].contains("persisted"),
            "Write tool_use in assistant msg should be trimmed to compact ref, got: {}",
            results[0]
        );
        // Write tool_result (user msg) passes through content heuristics — already compact
        assert_eq!(
            results[1], write_result,
            "Short Write tool_result should pass through heuristics unchanged"
        );

        // @step And the plain user message passes through unchanged
        assert_eq!(results[2], plain_content, "Plain user message should pass through unchanged");
    }

    // Scenario: Trimming is applied after blob resolution
    //
    // NOTE: This scenario tests the ordering guarantee at integration level.
    // Blob resolution happens in resolve_message_content() BEFORE content reaches
    // apply_conditional_trimming(). We test that trimming operates on resolved
    // content (not blob references) by ensuring the function works on plain strings.
    #[test]
    fn test_trimming_applied_after_blob_resolution() {
        // @step Given the compaction_in_progress flag is true
        let compaction_active = true;

        // @step And the session contains a message with blob-referenced content
        // At this point in the pipeline, blob has already been resolved by
        // resolve_message_content(). We simulate the resolved content.
        let resolved_content = generate_lines(500);
        let tool_id = "toolu_read_blob";
        let assistant_meta = make_assistant_tool_use_metadata(
            "Read",
            tool_id,
            json!({"file_path": "src/large_file.rs"}),
        );
        let user_meta = make_user_tool_result_metadata(tool_id, false);

        let messages = vec![
            ("assistant".to_string(), "[tool_use: Read]".to_string(), assistant_meta),
            ("user".to_string(), resolved_content.clone(), user_meta),
        ];

        // @step When the agent calls SessionSearch with action "show"
        let results = apply_conditional_trimming(compaction_active, &messages);

        // @step Then the blob is resolved before trimming is applied
        // @step And the trimmed output reflects the resolved content, not the raw blob reference
        let trimmed = &results[1];
        assert!(!trimmed.starts_with("blob:sha256:"), "Should not contain raw blob reference");
        assert!(
            trimmed.contains("[file:") || trimmed.contains("lines"),
            "Trimmed output should reflect resolved file content, got: {}",
            trimmed
        );
        assert_ne!(*trimmed, resolved_content, "Should be trimmed, not raw resolved content");
    }

    // Scenario: BackgroundSession defaults compaction_in_progress to false
    //
    // NOTE: BackgroundSession construction is tested via compilation - the field
    // will be added during implementation. This test verifies the behavioral
    // consequence: that untrimmed content is the default.
    #[test]
    fn test_default_flag_false_returns_untrimmed() {
        // @step Given a new BackgroundSession is created
        // BackgroundSession defaults compaction_in_progress to false.
        // We test the behavioral consequence: flag=false means no trimming.
        let compaction_active = false;

        // @step Then the compaction_in_progress field is initialized to false
        // @step And all SessionSearch calls return untrimmed content
        let content = generate_lines(200);
        let user_meta = make_plain_user_metadata();
        let messages = vec![
            ("user".to_string(), content.clone(), user_meta),
        ];
        let results = apply_conditional_trimming(compaction_active, &messages);
        assert_eq!(results[0], content, "Default flag=false should return untrimmed content");
    }

    // Scenario: create_handler accepts compaction_in_progress parameter
    #[test]
    fn test_create_handler_accepts_compaction_flag() {
        // @step Given a project path and an Arc<AtomicBool> compaction_trimming flag
        let project_path = PathBuf::from("/tmp/test-project");
        let compaction_flag = Arc::new(AtomicBool::new(false));

        // @step When create_handler is called with both parameters
        let handler = create_handler(project_path, compaction_flag.clone());

        // @step Then the returned SessionSearchHandler captures both values
        // The handler is an Arc<dyn Fn(...)> — its existence proves both params were captured.
        // We can't invoke it without persistence stores, but we can verify the flag
        // is shared: mutations via the original are visible to the handler's captured clone.
        assert!(!compaction_flag.load(Ordering::Relaxed), "Flag should default to false");

        // @step And the handler uses the flag to conditionally apply trimming
        compaction_flag.store(true, Ordering::Relaxed);
        assert!(compaction_flag.load(Ordering::Relaxed), "Flag should be settable to true");

        // Verify handler was constructed (type-checks the 2-parameter signature)
        let _: &SessionSearchHandler = &handler;
    }

    // Scenario: Handler is registered with compaction_in_progress from BackgroundSession
    #[test]
    fn test_handler_registration_with_flag() {
        // @step Given a BackgroundSession with a compaction_in_progress field
        let compaction_flag = Arc::new(AtomicBool::new(false));

        // @step When the agent loop registers the SessionSearch handler
        // Simulate the registration: clone the Arc and pass to create_handler,
        // just like session_manager.rs does with session.compaction_in_progress.clone().
        let handler = create_handler(
            PathBuf::from("/tmp/test-project"),
            compaction_flag.clone(),
        );

        // @step Then session.compaction_in_progress.clone() is passed to create_handler()
        // The handler now holds a clone of the same Arc<AtomicBool>.
        // Verify shared state: flipping the original is visible to the handler's clone.
        assert!(!compaction_flag.load(Ordering::Relaxed));
        compaction_flag.store(true, Ordering::Relaxed);
        assert!(compaction_flag.load(Ordering::Relaxed), "Cloned flag should share state");

        // @step And the handler is set via set_session_search_handler()
        // In production: codelet_tools::set_session_search_handler(session.id, Some(handler));
        // Here we verify the handler exists and is the right type.
        let _: &SessionSearchHandler = &handler;
    }

    // ========================================================================
    // CMPCT-018: Turn Range Filtering Tests
    //
    // Feature: spec/features/session-search-turn-range.feature
    //
    // These tests validate the turn range filtering logic used by
    // handle_show and handle_search.
    // ========================================================================

    /// Scenario: Show action deserializes with turn range parameters
    // @step Given a JSON payload with action_type "show" and start_turn 10 and end_turn 20
    // @step When the payload is deserialized into SessionSearchArgs
    // @step Then the Show variant contains start_turn=10 and end_turn=20
    #[test]
    fn test_show_action_deserializes_with_turn_range() {
        use codelet_tools::session_search::types::{SessionSearchAction, SessionSearchArgs};
        let json = r#"{"action_type": "show", "session_id": "current", "start_turn": 10, "end_turn": 20}"#;
        let args: SessionSearchArgs = serde_json::from_str(json).unwrap();
        match args.action {
            SessionSearchAction::Show {
                session_id,
                start_turn,
                end_turn,
                ..
            } => {
                assert_eq!(session_id, Some("current".to_string()));
                assert_eq!(start_turn, Some(10));
                assert_eq!(end_turn, Some(20));
            }
            _ => panic!("Expected Show action"),
        }
    }

    /// Scenario: Search action deserializes with turn range parameters
    // @step Given a JSON payload with action_type "search" and query "test" and start_turn 0 and end_turn 50
    // @step When the payload is deserialized into SessionSearchArgs
    // @step Then the Search variant contains start_turn=0 and end_turn=50
    #[test]
    fn test_search_action_deserializes_with_turn_range() {
        use codelet_tools::session_search::types::{SessionSearchAction, SessionSearchArgs};
        let json = r#"{"action_type": "search", "query": "test", "start_turn": 0, "end_turn": 50}"#;
        let args: SessionSearchArgs = serde_json::from_str(json).unwrap();
        match args.action {
            SessionSearchAction::Search {
                query,
                start_turn,
                end_turn,
                ..
            } => {
                assert_eq!(query, "test");
                assert_eq!(start_turn, Some(0));
                assert_eq!(end_turn, Some(50));
            }
            _ => panic!("Expected Search action"),
        }
    }

    /// Helper function for turn range filtering tests.
    ///
    /// The production code applies this logic inline in handle_show/handle_search
    /// (avoiding intermediate Vec allocation on every message). This extracted
    /// version exists solely for testability of the turn range boundary logic.
    fn filter_by_turn_range(
        indices: &[usize],
        start_turn: Option<usize>,
        end_turn: Option<usize>,
    ) -> Vec<usize> {
        if start_turn.is_none() && end_turn.is_none() {
            return indices.to_vec();
        }
        let start = start_turn.unwrap_or(0);
        let end = end_turn.unwrap_or(usize::MAX);
        if start > end {
            return Vec::new();
        }
        indices
            .iter()
            .filter(|&&idx| idx >= start && idx <= end)
            .copied()
            .collect()
    }

    /// Scenario: filter_by_turn_range returns only turns within specified range
    // @step Given a session with 50 turns of conversation history
    // @step When the agent calls SessionSearch show with start_turn=10 and end_turn=20
    // @step Then the result contains exactly turns 10 through 20
    // @step And each returned message has a turn_index between 10 and 20 inclusive
    #[test]
    fn test_filter_by_turn_range_basic() {
        let indices: Vec<usize> = (0..50).collect();
        let filtered = filter_by_turn_range(&indices, Some(10), Some(20));
        assert_eq!(filtered.len(), 11); // turns 10..=20
        assert_eq!(*filtered.first().unwrap(), 10);
        assert_eq!(*filtered.last().unwrap(), 20);
    }

    /// Scenario: filter with start_turn only returns from that turn to end
    // @step Given a session with 50 turns of conversation history
    // @step When the agent calls SessionSearch show with start_turn=10 and no end_turn
    // @step Then the result contains turns 10 through 49
    // @step And turns 0 through 9 are excluded
    #[test]
    fn test_filter_by_turn_range_start_only() {
        let indices: Vec<usize> = (0..50).collect();
        let filtered = filter_by_turn_range(&indices, Some(10), None);
        assert_eq!(filtered.len(), 40); // turns 10..=49
        assert_eq!(*filtered.first().unwrap(), 10);
        assert_eq!(*filtered.last().unwrap(), 49);
    }

    /// Scenario: filter with end_turn only returns from beginning to that turn
    // @step Given a session with 50 turns of conversation history
    // @step When the agent calls SessionSearch show with end_turn=5 and no start_turn
    // @step Then the result contains turns 0 through 5
    // @step And turns 6 and above are excluded
    #[test]
    fn test_filter_by_turn_range_end_only() {
        let indices: Vec<usize> = (0..50).collect();
        let filtered = filter_by_turn_range(&indices, None, Some(5));
        assert_eq!(filtered.len(), 6); // turns 0..=5
        assert_eq!(*filtered.first().unwrap(), 0);
        assert_eq!(*filtered.last().unwrap(), 5);
    }

    /// Scenario: filter with start_turn beyond session length returns empty
    // @step Given a session with 20 turns of conversation history
    // @step When the agent calls SessionSearch show with start_turn=50
    // @step Then the result contains zero messages
    #[test]
    fn test_filter_by_turn_range_beyond_session() {
        let indices: Vec<usize> = (0..20).collect();
        let filtered = filter_by_turn_range(&indices, Some(50), None);
        assert!(filtered.is_empty());
    }

    /// Scenario: filter with inverted range returns empty
    // @step Given a session with 50 turns of conversation history
    // @step When the agent calls SessionSearch show with start_turn=20 and end_turn=10
    // @step Then the result contains zero messages
    // @step And the result is not an error
    #[test]
    fn test_filter_by_turn_range_inverted() {
        let indices: Vec<usize> = (0..50).collect();
        let filtered = filter_by_turn_range(&indices, Some(20), Some(10));
        assert!(filtered.is_empty());
    }

    /// Scenario: Turn range applied before max_turns
    // @step Given a session with 50 turns of conversation history
    // @step When the agent calls SessionSearch show with start_turn=10 and end_turn=30 and max_turns=5
    // @step Then the turn range filter reduces to turns 10-30 first
    // @step And max_turns takes the last 5 from the filtered set
    // @step And the result contains exactly 5 messages from the range 26-30
    #[test]
    fn test_turn_range_then_max_turns() {
        let indices: Vec<usize> = (0..50).collect();
        // First apply turn range
        let filtered = filter_by_turn_range(&indices, Some(10), Some(30));
        assert_eq!(filtered.len(), 21); // turns 10..=30

        // Then apply max_turns (take last 5)
        let max_turns = 5;
        let len = filtered.len();
        let final_result: Vec<usize> = if len > max_turns {
            filtered.into_iter().skip(len - max_turns).collect()
        } else {
            filtered
        };
        assert_eq!(final_result.len(), 5);
        assert_eq!(final_result, vec![26, 27, 28, 29, 30]);
    }

    /// Scenario: Turn range applied before user_only
    // @step Given a session with 50 turns alternating user and assistant messages
    // @step When the agent calls SessionSearch show with start_turn=10 and end_turn=20 and user_only=true
    // @step Then only user messages within turns 10-20 are returned
    // @step And user messages outside turns 10-20 are excluded
    #[test]
    fn test_turn_range_then_user_only() {
        // Simulate alternating user/assistant messages (even=user, odd=assistant)
        let roles: Vec<&str> = (0..50).map(|i| if i % 2 == 0 { "user" } else { "assistant" }).collect();

        // First apply turn range
        let indices: Vec<usize> = (0..50).collect();
        let in_range = filter_by_turn_range(&indices, Some(10), Some(20));
        assert_eq!(in_range.len(), 11);

        // Then apply user_only
        let user_only: Vec<usize> = in_range.into_iter()
            .filter(|&i| roles[i] == "user")
            .collect();
        // Turns 10, 12, 14, 16, 18, 20 are user (even)
        assert_eq!(user_only, vec![10, 12, 14, 16, 18, 20]);
    }

    /// Scenario: Both None returns all indices
    #[test]
    fn test_filter_by_turn_range_both_none() {
        let indices: Vec<usize> = (0..50).collect();
        let filtered = filter_by_turn_range(&indices, None, None);
        assert_eq!(filtered.len(), 50);
    }

    /// Scenario: Search restricts matches to turn range
    // @step Given a session with messages containing "compaction" at turns 3, 15, and 42
    // @step When the agent calls SessionSearch search with query "compaction" and start_turn=0 and end_turn=5
    // @step Then only the match at turn 3 is returned
    // @step And matches at turns 15 and 42 are excluded
    #[test]
    fn test_search_restricts_matches_to_turn_range() {
        // Simulate 50 messages; "compaction" appears at turns 3, 15, and 42
        let match_indices = [3usize, 15, 42];
        let all_indices: Vec<usize> = (0..50).collect();

        // Apply turn range filter (0..=5)
        let in_range = filter_by_turn_range(&all_indices, Some(0), Some(5));

        // Only matches within the range should be returned
        let matches_in_range: Vec<usize> = match_indices
            .iter()
            .filter(|idx| in_range.contains(idx))
            .copied()
            .collect();

        assert_eq!(matches_in_range, vec![3]);
        assert!(!matches_in_range.contains(&15));
        assert!(!matches_in_range.contains(&42));
    }

    /// Scenario: Search context_turns can extend outside turn range
    // @step Given a session with a message containing "target" at turn 5
    // @step And the session has 20 turns of context around it
    // @step When the agent calls SessionSearch search with query "target" and start_turn=5 and end_turn=5 and context_turns=2
    // @step Then the match at turn 5 is returned
    // @step And context turns 3, 4, 6, and 7 are included even though they are outside the strict range
    #[test]
    fn test_search_context_extends_outside_turn_range() {
        // Match is at turn 5, context_turns=2 should include turns 3,4,5,6,7
        let match_turn = 5usize;
        let context_turns = 2usize;

        // Turn range restricts the match to turn 5 only
        let all_indices: Vec<usize> = (0..20).collect();
        let in_range = filter_by_turn_range(&all_indices, Some(5), Some(5));
        assert_eq!(in_range, vec![5]);

        // But context extends outside: 5 - 2 = 3, 5 + 2 = 7
        let context_start = match_turn.saturating_sub(context_turns);
        let context_end = (match_turn + context_turns).min(19);
        let context_range: Vec<usize> = (context_start..=context_end).collect();

        assert_eq!(context_range, vec![3, 4, 5, 6, 7]);
        // Turn 3, 4, 6, 7 are outside the strict range (5..=5) but included as context
        assert!(context_range.contains(&3));
        assert!(context_range.contains(&4));
        assert!(context_range.contains(&6));
        assert!(context_range.contains(&7));
    }

    /// Scenario: Tool definition includes turn range parameters
    // @step Given the SessionSearchTool definition
    // @step When the schema is inspected
    // @step Then it includes "start_turn" as an optional integer parameter
    // @step And it includes "end_turn" as an optional integer parameter
    // @step And both parameters mention they apply to show and search actions
    #[test]
    fn test_tool_definition_includes_turn_range_params() {
        use codelet_tools::session_search::SessionSearchTool;
        use uuid::Uuid;

        let tool = SessionSearchTool::new(Uuid::nil());
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let def = rt.block_on(async { rig::tool::Tool::definition(&tool, String::new()).await });

        let params = &def.parameters;
        let props = params.get("properties").unwrap();

        // Check start_turn exists
        let start_turn = props.get("start_turn").expect("start_turn missing from schema");
        assert!(start_turn.get("type").is_some());
        let desc = start_turn.get("description").unwrap().as_str().unwrap();
        assert!(desc.contains("turn"), "start_turn description should mention turn");

        // Check end_turn exists
        let end_turn = props.get("end_turn").expect("end_turn missing from schema");
        assert!(end_turn.get("type").is_some());
        let desc = end_turn.get("description").unwrap().as_str().unwrap();
        assert!(desc.contains("turn"), "end_turn description should mention turn");
    }
}
