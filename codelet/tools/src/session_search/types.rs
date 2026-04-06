//! SessionSearch types — action enum, args, and result structures
//!
//! Feature: spec/features/session-search.feature
//!
//! Defines the data model for the SessionSearch tool's three actions:
//! - `recent`: List recent sessions for discovery
//! - `search`: Keyword search across all session content with context
//! - `show`: Load and display a specific session's conversation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// SessionSearch action types — discriminated union via serde tag
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "action_type", rename_all = "snake_case")]
pub enum SessionSearchAction {
    /// List recent sessions for discovery
    Recent {
        /// Number of sessions to return (default: 10)
        #[serde(default)]
        count: Option<usize>,
    },
    /// Search across all session content by keyword
    Search {
        /// The search query (supports ripgrep regex)
        query: String,
        /// Number of context turns around each match (default: 0)
        #[serde(default)]
        context_turns: Option<usize>,
        /// Maximum number of matches to return (default: 20)
        #[serde(default)]
        limit: Option<usize>,
        /// Search across all projects, not just current (default: false)
        #[serde(default)]
        all_projects: Option<bool>,
        /// Only search sessions updated in the last N hours
        #[serde(default)]
        last_hours: Option<u64>,
        /// Only search sessions updated in the last N days
        #[serde(default)]
        last_days: Option<u64>,
        /// Only search sessions updated after this ISO timestamp
        #[serde(default)]
        after: Option<String>,
        /// Only search sessions updated before this ISO timestamp
        #[serde(default)]
        before: Option<String>,
        /// Start of turn range (inclusive, 0-based) to restrict search results
        #[serde(default)]
        start_turn: Option<usize>,
        /// End of turn range (inclusive, 0-based) to restrict search results
        #[serde(default)]
        end_turn: Option<usize>,
    },
    /// Show a specific session's full conversation
    Show {
        /// Session ID to show (UUID string, or "current" / omitted for current session)
        #[serde(default)]
        session_id: Option<String>,
        /// Only include user messages (default: false)
        #[serde(default)]
        user_only: Option<bool>,
        /// Maximum number of turns to include (from the end)
        #[serde(default)]
        max_turns: Option<usize>,
        /// Start of turn range (inclusive, 0-based) to restrict results
        #[serde(default)]
        start_turn: Option<usize>,
        /// End of turn range (inclusive, 0-based) to restrict results
        #[serde(default)]
        end_turn: Option<usize>,
    },
}

/// Top-level args for the SessionSearch tool
#[derive(Debug, Deserialize, Serialize)]
pub struct SessionSearchArgs {
    /// The action to perform
    #[serde(flatten)]
    pub action: SessionSearchAction,
}

/// A session summary returned by the `recent` action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    /// Session UUID
    pub session_id: String,
    /// Human-readable session name
    pub name: String,
    /// Attached work unit ID (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_unit_id: Option<String>,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// When the session was last updated
    pub updated_at: DateTime<Utc>,
    /// Total number of messages in the session
    pub message_count: usize,
    /// Project path this session belongs to
    pub project: String,
    /// Provider used for this session (e.g., "claude", "openai")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Preview of the first user message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_user_message: Option<String>,
    /// Preview of the last user message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_user_message: Option<String>,
}

/// A single search match returned by the `search` action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatch {
    /// Session UUID where the match was found
    pub session_id: String,
    /// Session name
    pub session_name: String,
    /// Message role (user/assistant)
    pub role: String,
    /// Turn index within the session
    pub turn_index: usize,
    /// Timestamp of the matched message
    pub timestamp: DateTime<Utc>,
    /// Preview of the matched content (with match highlighted)
    pub matched_content: String,
    /// Project path
    pub project: String,
}

/// A group of search matches from the same session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatchGroup {
    /// Session UUID
    pub session_id: String,
    /// Session name
    pub session_name: String,
    /// Project path
    pub project: String,
    /// Provider used for this session
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Total messages in this session
    pub message_count: usize,
    /// All matches in this session
    pub matches: Vec<SearchMatch>,
    /// Context turns around matches (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<ContextTurn>>,
}

/// A context turn around a search match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextTurn {
    /// Turn index
    pub turn_index: usize,
    /// Message role
    pub role: String,
    /// Message content (may be truncated)
    pub content: String,
    /// Whether this turn contains the actual match
    pub is_match: bool,
}

/// A message in a shown session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    /// Turn index
    pub turn_index: usize,
    /// Message role (user/assistant)
    pub role: String,
    /// Message content (truncated to 5000 chars)
    pub content: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Whether content was truncated
    pub truncated: bool,
}

/// Result from any SessionSearch action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "result_type", rename_all = "snake_case")]
pub enum SessionSearchResult {
    /// Result from the `recent` action
    Recent {
        sessions: Vec<SessionSummary>,
    },
    /// Result from the `search` action
    Search {
        query: String,
        total_matches: usize,
        groups: Vec<SearchMatchGroup>,
    },
    /// Result from the `search` action with no matches
    NoMatches {
        query: String,
        message: String,
    },
    /// Result from the `show` action
    Session {
        session_id: String,
        session_name: String,
        messages: Vec<SessionMessage>,
    },
    /// Error result
    Error {
        message: String,
    },
}

/// Message truncation limit for show action (matching existing tool conventions)
pub const MESSAGE_TRUNCATION_LIMIT: usize = 5000;

/// Default number of recent sessions to return
pub const DEFAULT_RECENT_COUNT: usize = 10;

/// Default search result limit
pub const DEFAULT_SEARCH_LIMIT: usize = 20;

/// Preview length for user message summaries
pub const USER_MESSAGE_PREVIEW_LEN: usize = 200;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    // @step Given the SessionSearch args schema supports three action types
    #[test]
    fn test_recent_action_deserializes() {
        let json = r#"{"action_type": "recent", "count": 5}"#;
        let args: SessionSearchArgs = serde_json::from_str(json).unwrap();
        match args.action {
            SessionSearchAction::Recent { count } => {
                assert_eq!(count, Some(5));
            }
            _ => panic!("Expected Recent action"),
        }
    }

    // @step Given the SessionSearch args schema supports three action types
    #[test]
    fn test_recent_action_defaults_count() {
        let json = r#"{"action_type": "recent"}"#;
        let args: SessionSearchArgs = serde_json::from_str(json).unwrap();
        match args.action {
            SessionSearchAction::Recent { count } => {
                assert!(count.is_none());
            }
            _ => panic!("Expected Recent action"),
        }
    }

    // @step Given the SessionSearch args schema supports three action types
    #[test]
    fn test_search_action_deserializes() {
        let json = r#"{"action_type": "search", "query": "RLM-001", "context_turns": 3}"#;
        let args: SessionSearchArgs = serde_json::from_str(json).unwrap();
        match args.action {
            SessionSearchAction::Search {
                query,
                context_turns,
                ..
            } => {
                assert_eq!(query, "RLM-001");
                assert_eq!(context_turns, Some(3));
            }
            _ => panic!("Expected Search action"),
        }
    }

    // @step Given the SessionSearch args schema supports three action types
    #[test]
    fn test_search_action_with_time_filters() {
        let json = r#"{"action_type": "search", "query": "test", "last_hours": 24, "all_projects": true}"#;
        let args: SessionSearchArgs = serde_json::from_str(json).unwrap();
        match args.action {
            SessionSearchAction::Search {
                query,
                last_hours,
                all_projects,
                ..
            } => {
                assert_eq!(query, "test");
                assert_eq!(last_hours, Some(24));
                assert_eq!(all_projects, Some(true));
            }
            _ => panic!("Expected Search action"),
        }
    }

    // @step Given the SessionSearch args schema supports three action types
    #[test]
    fn test_show_action_deserializes() {
        let json = r#"{"action_type": "show", "session_id": "current", "user_only": true}"#;
        let args: SessionSearchArgs = serde_json::from_str(json).unwrap();
        match args.action {
            SessionSearchAction::Show {
                session_id,
                user_only,
                ..
            } => {
                assert_eq!(session_id, Some("current".to_string()));
                assert_eq!(user_only, Some(true));
            }
            _ => panic!("Expected Show action"),
        }
    }

    // @step Given the SessionSearch args schema supports three action types
    #[test]
    fn test_show_action_defaults_to_current() {
        let json = r#"{"action_type": "show"}"#;
        let args: SessionSearchArgs = serde_json::from_str(json).unwrap();
        match args.action {
            SessionSearchAction::Show { session_id, .. } => {
                assert!(session_id.is_none());
            }
            _ => panic!("Expected Show action"),
        }
    }

    // @step Then the result is valid structured JSON that can be parsed programmatically
    #[test]
    fn test_result_serializes_as_json() {
        let result = SessionSearchResult::Recent {
            sessions: vec![SessionSummary {
                session_id: "test-id".to_string(),
                name: "Test Session".to_string(),
                work_unit_id: Some("AMGR-001".to_string()),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                message_count: 42,
                project: "/test/project".to_string(),
                provider: Some("claude".to_string()),
                first_user_message: Some("Hello".to_string()),
                last_user_message: Some("Goodbye".to_string()),
            }],
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("test-id"));
        assert!(json.contains("AMGR-001"));
        assert!(json.contains("recent"));
        assert!(json.contains("claude"));
    }

    // @step And the result is a valid structured response, not an error
    #[test]
    fn test_no_matches_result_is_not_error() {
        let result = SessionSearchResult::NoMatches {
            query: "nonexistent".to_string(),
            message: "No matches found".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("no_matches"));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn test_error_result() {
        let result = SessionSearchResult::Error {
            message: "Session not found".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("error"));
        assert!(json.contains("Session not found"));
    }

    #[test]
    fn test_constants() {
        assert_eq!(MESSAGE_TRUNCATION_LIMIT, 5000);
        assert_eq!(DEFAULT_RECENT_COUNT, 10);
        assert_eq!(DEFAULT_SEARCH_LIMIT, 20);
        assert_eq!(USER_MESSAGE_PREVIEW_LEN, 200);
    }

    // ========================================================================
    // CMPCT-018: Turn range parameter deserialization tests
    // Feature: spec/features/session-search-turn-range.feature
    // ========================================================================

    // @step Given a JSON payload with action_type "show" and start_turn 10 and end_turn 20
    // @step When the payload is deserialized into SessionSearchArgs
    // @step Then the Show variant contains start_turn=10 and end_turn=20
    #[test]
    fn test_show_action_deserializes_with_turn_range() {
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

    // @step Given a JSON payload with action_type "search" and query "test" and start_turn 0 and end_turn 50
    // @step When the payload is deserialized into SessionSearchArgs
    // @step Then the Search variant contains start_turn=0 and end_turn=50
    #[test]
    fn test_search_action_deserializes_with_turn_range() {
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

    // Verify turn range fields default to None when omitted
    #[test]
    fn test_show_action_defaults_turn_range_to_none() {
        let json = r#"{"action_type": "show"}"#;
        let args: SessionSearchArgs = serde_json::from_str(json).unwrap();
        match args.action {
            SessionSearchAction::Show {
                start_turn,
                end_turn,
                ..
            } => {
                assert!(start_turn.is_none());
                assert!(end_turn.is_none());
            }
            _ => panic!("Expected Show action"),
        }
    }

    #[test]
    fn test_search_action_defaults_turn_range_to_none() {
        let json = r#"{"action_type": "search", "query": "test"}"#;
        let args: SessionSearchArgs = serde_json::from_str(json).unwrap();
        match args.action {
            SessionSearchAction::Search {
                start_turn,
                end_turn,
                ..
            } => {
                assert!(start_turn.is_none());
                assert!(end_turn.is_none());
            }
            _ => panic!("Expected Search action"),
        }
    }
}
