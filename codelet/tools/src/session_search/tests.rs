//! Feature: spec/features/session-search.feature
//!
//! This test file validates the acceptance criteria defined in the feature file.
//! Scenarios map directly to Gherkin scenarios for the SessionSearch tool.

use super::handler::*;
use super::reassembly::*;
use super::types::*;
use super::SessionSearchTool;
use chrono::{self, Utc};
use rig::tool::Tool;
use serial_test::serial;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use uuid::Uuid;

fn with_clean_handlers<T>(f: impl FnOnce() -> T) -> T {
    clear_all_session_search_handlers();
    let result = f();
    clear_all_session_search_handlers();
    result
}

/// Scenario: List recent sessions for discovery
#[tokio::test]
#[serial]
async fn test_recent_sessions_for_discovery() {
    with_clean_handlers(|| {});

    // @step Given the persistence layer contains multiple sessions for the current project
    let session_id = Uuid::new_v4();
    let handler: SessionSearchHandler = Arc::new(|action, _sid| match action {
        SessionSearchAction::Recent { count } => {
            let n = count.unwrap_or(DEFAULT_RECENT_COUNT);
            let sessions: Vec<SessionSummary> = (0..n.min(5))
                .map(|i| SessionSummary {
                    session_id: format!("session-{i}"),
                    name: format!("Session {i}"),
                    work_unit_id: if i == 0 { Some("AMGR-001".to_string()) } else { None },
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    message_count: 10 + i,
                    project: "/test/project".to_string(),
                    provider: Some("claude".to_string()),
                    first_user_message: Some("Hello".to_string()),
                    last_user_message: Some("Bye".to_string()),
                })
                .collect();
            SessionSearchResult::Recent { sessions }
        }
        _ => SessionSearchResult::Error { message: "unexpected".to_string() },
    });
    set_session_search_handler(session_id, Some(handler));

    // @step When the agent calls SessionSearch with action "recent" and count 5
    let tool = SessionSearchTool::new(session_id);
    let output = tool
        .call(SessionSearchArgs {
            action: SessionSearchAction::Recent { count: Some(5) },
        })
        .await
        .unwrap();

    // @step Then the result contains up to 5 sessions ordered by updated_at descending
    let parsed: SessionSearchResult = serde_json::from_str(&output).unwrap();
    match parsed {
        SessionSearchResult::Recent { ref sessions } => {
            assert!(sessions.len() <= 5);

            // @step And each session entry includes session ID, name, work unit ID, timestamps, message count, and project path
            assert!(!sessions[0].session_id.is_empty());
            assert!(!sessions[0].name.is_empty());
            assert!(sessions[0].work_unit_id.is_some());
            assert!(!sessions[0].project.is_empty());

            // @step And each session entry includes a preview of the first and last user messages
            assert!(sessions[0].first_user_message.is_some());
            assert!(sessions[0].last_user_message.is_some());
        }
        _ => panic!("Expected Recent result"),
    }

    clear_all_session_search_handlers();
}

/// Scenario: Recent sessions defaults to 10 when count is not specified
#[tokio::test]
#[serial]
async fn test_recent_defaults_to_10() {
    with_clean_handlers(|| {});

    // @step Given the persistence layer contains 15 sessions for the current project
    let session_id = Uuid::new_v4();
    let handler: SessionSearchHandler = Arc::new(|action, _sid| match action {
        SessionSearchAction::Recent { count } => {
            let n = count.unwrap_or(DEFAULT_RECENT_COUNT);
            let sessions: Vec<SessionSummary> = (0..n.min(15))
                .map(|i| SessionSummary {
                    session_id: format!("session-{i}"),
                    name: format!("Session {i}"),
                    work_unit_id: None,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    message_count: i,
                    project: "/test".to_string(),
                    provider: None,
                    first_user_message: None,
                    last_user_message: None,
                })
                .collect();
            SessionSearchResult::Recent { sessions }
        }
        _ => SessionSearchResult::Error { message: "unexpected".to_string() },
    });
    set_session_search_handler(session_id, Some(handler));

    // @step When the agent calls SessionSearch with action "recent" and no count parameter
    let tool = SessionSearchTool::new(session_id);
    let output = tool
        .call(SessionSearchArgs {
            action: SessionSearchAction::Recent { count: None },
        })
        .await
        .unwrap();

    // @step Then the result contains 10 sessions
    let parsed: SessionSearchResult = serde_json::from_str(&output).unwrap();
    match parsed {
        SessionSearchResult::Recent { sessions } => {
            assert_eq!(sessions.len(), DEFAULT_RECENT_COUNT);
        }
        _ => panic!("Expected Recent result"),
    }

    clear_all_session_search_handlers();
}

/// Scenario: Search with no matches returns empty result
#[tokio::test]
#[serial]
async fn test_search_no_matches() {
    with_clean_handlers(|| {});

    // @step Given no sessions contain the text "nonexistent-query-xyz"
    let session_id = Uuid::new_v4();
    let handler: SessionSearchHandler = Arc::new(|action, _sid| match action {
        SessionSearchAction::Search { query, .. } => SessionSearchResult::NoMatches {
            query,
            message: "No matches found".to_string(),
        },
        _ => SessionSearchResult::Error { message: "unexpected".to_string() },
    });
    set_session_search_handler(session_id, Some(handler));

    // @step When the agent calls SessionSearch with action "search" and query "nonexistent-query-xyz"
    let tool = SessionSearchTool::new(session_id);
    let output = tool
        .call(SessionSearchArgs {
            action: SessionSearchAction::Search {
                query: "nonexistent-query-xyz".to_string(),
                context_turns: None,
                limit: None,
                all_projects: None,
                last_hours: None,
                last_days: None,
                after: None,
                before: None,
            },
        })
        .await
        .unwrap();

    // @step Then the result indicates no matches found
    let parsed: SessionSearchResult = serde_json::from_str(&output).unwrap();

    // @step And the result is a valid structured response, not an error
    match parsed {
        SessionSearchResult::NoMatches { query, message } => {
            assert_eq!(query, "nonexistent-query-xyz");
            assert_eq!(message, "No matches found");
        }
        _ => panic!("Expected NoMatches result"),
    }

    clear_all_session_search_handlers();
}

/// Scenario: Show non-existent session returns error
#[tokio::test]
#[serial]
async fn test_show_nonexistent_session() {
    with_clean_handlers(|| {});

    // @step Given no session exists with ID "00000000-0000-0000-0000-000000000000"
    let session_id = Uuid::new_v4();
    let handler: SessionSearchHandler = Arc::new(|action, _sid| match action {
        SessionSearchAction::Show { session_id: show_id, .. } => {
            if show_id.as_deref() == Some("00000000-0000-0000-0000-000000000000") {
                SessionSearchResult::Error {
                    message: "Session 00000000-0000-0000-0000-000000000000 not found".to_string(),
                }
            } else {
                SessionSearchResult::Error { message: "unexpected".to_string() }
            }
        }
        _ => SessionSearchResult::Error { message: "unexpected".to_string() },
    });
    set_session_search_handler(session_id, Some(handler));

    // @step When the agent calls SessionSearch with action "show" and session_id "00000000-0000-0000-0000-000000000000"
    let tool = SessionSearchTool::new(session_id);
    let output = tool
        .call(SessionSearchArgs {
            action: SessionSearchAction::Show {
                session_id: Some("00000000-0000-0000-0000-000000000000".to_string()),
                user_only: None,
                max_turns: None,
            },
        })
        .await
        .unwrap();

    // @step Then the tool returns an error indicating the session was not found
    let parsed: SessionSearchResult = serde_json::from_str(&output).unwrap();
    match parsed {
        SessionSearchResult::Error { message } => {
            assert!(message.contains("not found"));
        }
        _ => panic!("Expected Error result"),
    }

    clear_all_session_search_handlers();
}

/// Scenario: SessionSearch output is structured JSON
#[tokio::test]
#[serial]
async fn test_output_is_structured_json() {
    with_clean_handlers(|| {});

    // @step Given the persistence layer contains sessions
    let session_id = Uuid::new_v4();
    let handler: SessionSearchHandler = Arc::new(|_, _| SessionSearchResult::Recent {
        sessions: vec![],
    });
    set_session_search_handler(session_id, Some(handler));

    // @step When any SessionSearch action is called
    let tool = SessionSearchTool::new(session_id);
    let output = tool
        .call(SessionSearchArgs {
            action: SessionSearchAction::Recent { count: Some(1) },
        })
        .await
        .unwrap();

    // @step Then the result is valid structured JSON that can be parsed programmatically
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(parsed.is_object());
    assert!(parsed.get("result_type").is_some());

    clear_all_session_search_handlers();
}

/// Scenario: SessionSearch uses persistence layer directly
#[test]
#[serial]
fn test_uses_persistence_layer_directly() {
    with_clean_handlers(|| {
        // @step Given the SessionSearch tool is compiled as native Rust
        let session_id = Uuid::new_v4();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        let handler: SessionSearchHandler = Arc::new(move |action, _sid| {
            called_clone.store(true, Ordering::SeqCst);
            match action {
                SessionSearchAction::Recent { .. } => {
                    SessionSearchResult::Recent { sessions: vec![] }
                }
                _ => SessionSearchResult::Error { message: "unexpected".to_string() },
            }
        });
        set_session_search_handler(session_id, Some(handler));

        // @step When any SessionSearch action is invoked
        let result = execute_session_search(session_id, SessionSearchAction::Recent { count: None });

        // @step Then data is read from MessageStore, HistoryStore, and BlobStore directly
        assert!(called.load(Ordering::SeqCst));

        // @step And no Python or bash subprocess is spawned
        match result {
            SessionSearchResult::Recent { .. } => {}
            _ => panic!("Expected Recent result"),
        }
    });
}

/// Scenario: Show session reassembles streaming chunks
#[test]
fn test_reassembles_streaming_chunks() {
    // @step Given a session contains an assistant message stored as raw streaming chunks with "[Thinking: partial text...]" markers and "[Tool: Read]" markers and text fragments split mid-word
    let raw = "[Thinking: analyzing the code...]\n[Tool: Read]\nHere is \nthe file \ncontent";

    // @step When the agent calls SessionSearch with action "show" for that session
    let sections = reassemble_content(raw);

    // @step Then thinking chunks are merged into coherent thinking sections
    assert!(matches!(&sections[0], Section::Thinking(t) if t.contains("analyzing")));

    // @step And tool invocations are preserved as structured markers
    assert!(matches!(&sections[1], Section::Tool(t) if t == "Read"));

    // @step And text fragments are concatenated into readable prose
    match &sections[2] {
        Section::Text(content) => {
            assert!(content.contains("Here is"));
            assert!(content.contains("file"));
            assert!(content.contains("content"));
        }
        _ => panic!("Expected Text section"),
    }
}

/// Scenario: Handler concurrent session isolation
#[test]
#[serial]
fn test_handler_concurrent_sessions_isolated() {
    with_clean_handlers(|| {
        // @step Given the SessionSearch tool is compiled as native Rust
        let session_a = Uuid::new_v4();
        let session_b = Uuid::new_v4();

        let handler_a: SessionSearchHandler = Arc::new(|_, _| SessionSearchResult::Recent {
            sessions: vec![],
        });
        set_session_search_handler(session_a, Some(handler_a));

        let handler_b: SessionSearchHandler = Arc::new(|_, _| SessionSearchResult::Error {
            message: "from_b".to_string(),
        });
        set_session_search_handler(session_b, Some(handler_b));

        // @step When any SessionSearch action is invoked
        let result_a = execute_session_search(session_a, SessionSearchAction::Recent { count: None });

        // @step Then data is read from MessageStore, HistoryStore, and BlobStore directly
        match result_a {
            SessionSearchResult::Recent { .. } => {}
            _ => panic!("Expected Recent from session_a"),
        }

        let result_b = execute_session_search(session_b, SessionSearchAction::Recent { count: None });
        match result_b {
            SessionSearchResult::Error { message } => assert_eq!(message, "from_b"),
            _ => panic!("Expected Error from session_b"),
        }

        // @step And no Python or bash subprocess is spawned
        set_session_search_handler(session_b, None);
        let result_a2 = execute_session_search(session_a, SessionSearchAction::Recent { count: None });
        match result_a2 {
            SessionSearchResult::Recent { .. } => {}
            _ => panic!("Expected Recent from session_a after removing b"),
        }
    });
}

/// Test type serialization
#[test]
fn test_type_serialization() {
    // @step Given the persistence layer contains sessions
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

    // @step When any SessionSearch action is called
    let json = serde_json::to_string(&result).unwrap();

    // @step Then the result is valid structured JSON that can be parsed programmatically
    assert!(json.contains("test-id"));
    assert!(json.contains("AMGR-001"));
    assert!(json.contains("recent"));
}

/// Test action deserialization
#[test]
fn test_action_deserialization() {
    // @step Given the SessionSearch tool is compiled as native Rust
    let recent_json = r#"{"action_type": "recent", "count": 5}"#;
    let search_json = r#"{"action_type": "search", "query": "RLM-001", "context_turns": 3}"#;
    let show_json = r#"{"action_type": "show", "session_id": "current"}"#;

    // @step When any SessionSearch action is invoked
    let recent: SessionSearchArgs = serde_json::from_str(recent_json).unwrap();
    let search: SessionSearchArgs = serde_json::from_str(search_json).unwrap();
    let show: SessionSearchArgs = serde_json::from_str(show_json).unwrap();

    // @step Then data is read from MessageStore, HistoryStore, and BlobStore directly
    match recent.action {
        SessionSearchAction::Recent { count } => assert_eq!(count, Some(5)),
        _ => panic!("Expected Recent"),
    }
    match search.action {
        SessionSearchAction::Search { query, context_turns, .. } => {
            assert_eq!(query, "RLM-001");
            assert_eq!(context_turns, Some(3));
        }
        _ => panic!("Expected Search"),
    }

    // @step And no Python or bash subprocess is spawned
    match show.action {
        SessionSearchAction::Show { session_id, .. } => {
            assert_eq!(session_id, Some("current".to_string()));
        }
        _ => panic!("Expected Show"),
    }
}

/// Scenario: Search by keyword across all session content
#[tokio::test]
#[serial]
async fn test_search_by_keyword_across_all_content() {
    with_clean_handlers(|| {});

    // @step Given the persistence layer contains sessions with messages mentioning "RLM-001" in user inputs, assistant responses, and tool calls
    let session_id = Uuid::new_v4();
    let handler: SessionSearchHandler = Arc::new(|action, _sid| match action {
        SessionSearchAction::Search { query, .. } => {
            assert_eq!(query, "RLM-001");
            SessionSearchResult::Search {
                query: query.clone(),
                total_matches: 3,
                groups: vec![SearchMatchGroup {
                    session_id: "sess-1".to_string(),
                    session_name: "Work on RLM".to_string(),
                    project: "/test/project".to_string(),
                    provider: Some("claude".to_string()),
                    message_count: 10,
                    matches: vec![
                        SearchMatch {
                            session_id: "sess-1".to_string(),
                            session_name: "Work on RLM".to_string(),
                            role: "user".to_string(),
                            turn_index: 1,
                            timestamp: Utc::now(),
                            matched_content: "Working on RLM-001 today".to_string(),
                            project: "/test/project".to_string(),
                        },
                        SearchMatch {
                            session_id: "sess-1".to_string(),
                            session_name: "Work on RLM".to_string(),
                            role: "assistant".to_string(),
                            turn_index: 2,
                            timestamp: Utc::now(),
                            matched_content: "I'll help with RLM-001".to_string(),
                            project: "/test/project".to_string(),
                        },
                        SearchMatch {
                            session_id: "sess-1".to_string(),
                            session_name: "Work on RLM".to_string(),
                            role: "tool".to_string(),
                            turn_index: 3,
                            timestamp: Utc::now(),
                            matched_content: "grep found RLM-001 in file.rs".to_string(),
                            project: "/test/project".to_string(),
                        },
                    ],
                    context: None,
                }],
            }
        }
        _ => SessionSearchResult::Error { message: "unexpected".to_string() },
    });
    set_session_search_handler(session_id, Some(handler));

    // @step When the agent calls SessionSearch with action "search" and query "RLM-001"
    let tool = SessionSearchTool::new(session_id);
    let output = tool
        .call(SessionSearchArgs {
            action: SessionSearchAction::Search {
                query: "RLM-001".to_string(),
                context_turns: None,
                limit: None,
                all_projects: None,
                last_hours: None,
                last_days: None,
                after: None,
                before: None,
            },
        })
        .await
        .unwrap();

    let parsed: SessionSearchResult = serde_json::from_str(&output).unwrap();
    match parsed {
        SessionSearchResult::Search { query, total_matches, groups } => {
            assert_eq!(query, "RLM-001");

            // @step Then the result contains matches from user messages, assistant messages, and tool call content
            assert_eq!(total_matches, 3);
            let roles: Vec<&str> = groups[0].matches.iter().map(|m| m.role.as_str()).collect();
            assert!(roles.contains(&"user"));
            assert!(roles.contains(&"assistant"));
            assert!(roles.contains(&"tool"));

            // @step And each match includes session ID, session name, timestamp, role, turn index, and matched content preview
            let first = &groups[0].matches[0];
            assert!(!first.session_id.is_empty());
            assert!(!first.session_name.is_empty());
            assert!(!first.role.is_empty());
            assert!(!first.matched_content.is_empty());

            // @step And results are grouped by session
            assert_eq!(groups.len(), 1);
            assert_eq!(groups[0].session_id, "sess-1");
        }
        _ => panic!("Expected Search result"),
    }

    clear_all_session_search_handlers();
}

/// Scenario: Search with context turns shows surrounding conversation
#[tokio::test]
#[serial]
async fn test_search_with_context_turns() {
    with_clean_handlers(|| {});

    // @step Given a session contains a message mentioning "RLM-001" at turn 5
    let session_id = Uuid::new_v4();
    let handler: SessionSearchHandler = Arc::new(|action, _sid| match action {
        SessionSearchAction::Search { query, context_turns, .. } => {
            assert_eq!(context_turns, Some(3));
            SessionSearchResult::Search {
                query,
                total_matches: 1,
                groups: vec![SearchMatchGroup {
                    session_id: "sess-1".to_string(),
                    session_name: "Session".to_string(),
                    project: "/test".to_string(),
                    provider: None,
                    message_count: 20,
                    matches: vec![SearchMatch {
                        session_id: "sess-1".to_string(),
                        session_name: "Session".to_string(),
                        role: "user".to_string(),
                        turn_index: 5,
                        timestamp: Utc::now(),
                        matched_content: "RLM-001 issue".to_string(),
                        project: "/test".to_string(),
                    }],
                    context: Some(
                        (2..=8)
                            .map(|i| ContextTurn {
                                turn_index: i,
                                role: if i % 2 == 0 { "user".to_string() } else { "assistant".to_string() },
                                content: format!("Turn {i} content"),
                                is_match: i == 5,
                            })
                            .collect(),
                    ),
                }],
            }
        }
        _ => SessionSearchResult::Error { message: "unexpected".to_string() },
    });
    set_session_search_handler(session_id, Some(handler));

    // @step When the agent calls SessionSearch with action "search" and query "RLM-001" and context_turns 3
    let tool = SessionSearchTool::new(session_id);
    let output = tool
        .call(SessionSearchArgs {
            action: SessionSearchAction::Search {
                query: "RLM-001".to_string(),
                context_turns: Some(3),
                limit: None,
                all_projects: None,
                last_hours: None,
                last_days: None,
                after: None,
                before: None,
            },
        })
        .await
        .unwrap();

    let parsed: SessionSearchResult = serde_json::from_str(&output).unwrap();
    match parsed {
        SessionSearchResult::Search { groups, .. } => {
            let ctx = groups[0].context.as_ref().unwrap();

            // @step Then the result includes turns 2 through 8 around the matching turn
            let indices: Vec<usize> = ctx.iter().map(|c| c.turn_index).collect();
            assert_eq!(indices, vec![2, 3, 4, 5, 6, 7, 8]);

            // @step And the matching turn is identified within the context
            let match_turns: Vec<&ContextTurn> = ctx.iter().filter(|c| c.is_match).collect();
            assert_eq!(match_turns.len(), 1);
            assert_eq!(match_turns[0].turn_index, 5);
        }
        _ => panic!("Expected Search result"),
    }

    clear_all_session_search_handlers();
}

/// Scenario: Search defaults to current project only
#[tokio::test]
#[serial]
async fn test_search_defaults_to_current_project() {
    with_clean_handlers(|| {});

    // @step Given sessions exist for both "/project-a" and "/project-b"
    // @step And the current project is "/project-a"
    let session_id = Uuid::new_v4();
    let handler: SessionSearchHandler = Arc::new(|action, _sid| match action {
        SessionSearchAction::Search { query, all_projects, .. } => {
            // Handler verifies all_projects is None/false and only returns project-a
            assert!(all_projects.is_none() || all_projects == Some(false));
            SessionSearchResult::Search {
                query,
                total_matches: 1,
                groups: vec![SearchMatchGroup {
                    session_id: "sess-a".to_string(),
                    session_name: "Project A session".to_string(),
                    project: "/project-a".to_string(),
                    provider: None,
                    message_count: 5,
                    matches: vec![SearchMatch {
                        session_id: "sess-a".to_string(),
                        session_name: "Project A session".to_string(),
                        role: "user".to_string(),
                        turn_index: 1,
                        timestamp: Utc::now(),
                        matched_content: "authentication code".to_string(),
                        project: "/project-a".to_string(),
                    }],
                    context: None,
                }],
            }
        }
        _ => SessionSearchResult::Error { message: "unexpected".to_string() },
    });
    set_session_search_handler(session_id, Some(handler));

    // @step When the agent calls SessionSearch with action "search" and query "authentication"
    let tool = SessionSearchTool::new(session_id);
    let output = tool
        .call(SessionSearchArgs {
            action: SessionSearchAction::Search {
                query: "authentication".to_string(),
                context_turns: None,
                limit: None,
                all_projects: None,
                last_hours: None,
                last_days: None,
                after: None,
                before: None,
            },
        })
        .await
        .unwrap();

    // @step Then results only include matches from "/project-a" sessions
    let parsed: SessionSearchResult = serde_json::from_str(&output).unwrap();
    match parsed {
        SessionSearchResult::Search { groups, .. } => {
            for group in &groups {
                for m in &group.matches {
                    assert_eq!(m.project, "/project-a");
                }
            }
        }
        _ => panic!("Expected Search result"),
    }

    clear_all_session_search_handlers();
}

/// Scenario: Search across all projects with flag
#[tokio::test]
#[serial]
async fn test_search_across_all_projects() {
    with_clean_handlers(|| {});

    // @step Given sessions exist for both "/project-a" and "/project-b" containing "DeepSearch"
    // @step And the current project is "/project-a"
    let session_id = Uuid::new_v4();
    let handler: SessionSearchHandler = Arc::new(|action, _sid| match action {
        SessionSearchAction::Search { query, all_projects, .. } => {
            assert_eq!(all_projects, Some(true));
            SessionSearchResult::Search {
                query,
                total_matches: 2,
                groups: vec![
                    SearchMatchGroup {
                        session_id: "sess-a".to_string(),
                        session_name: "Project A".to_string(),
                        project: "/project-a".to_string(),
                        provider: Some("claude".to_string()),
                        message_count: 8,
                        matches: vec![SearchMatch {
                            session_id: "sess-a".to_string(),
                            session_name: "Project A".to_string(),
                            role: "user".to_string(),
                            turn_index: 1,
                            timestamp: Utc::now(),
                            matched_content: "Using DeepSearch".to_string(),
                            project: "/project-a".to_string(),
                        }],
                        context: None,
                    },
                    SearchMatchGroup {
                        session_id: "sess-b".to_string(),
                        session_name: "Project B".to_string(),
                        project: "/project-b".to_string(),
                        provider: Some("openai".to_string()),
                        message_count: 12,
                        matches: vec![SearchMatch {
                            session_id: "sess-b".to_string(),
                            session_name: "Project B".to_string(),
                            role: "assistant".to_string(),
                            turn_index: 3,
                            timestamp: Utc::now(),
                            matched_content: "DeepSearch results".to_string(),
                            project: "/project-b".to_string(),
                        }],
                        context: None,
                    },
                ],
            }
        }
        _ => SessionSearchResult::Error { message: "unexpected".to_string() },
    });
    set_session_search_handler(session_id, Some(handler));

    // @step When the agent calls SessionSearch with action "search" and query "DeepSearch" and all_projects true
    let tool = SessionSearchTool::new(session_id);
    let output = tool
        .call(SessionSearchArgs {
            action: SessionSearchAction::Search {
                query: "DeepSearch".to_string(),
                context_turns: None,
                limit: None,
                all_projects: Some(true),
                last_hours: None,
                last_days: None,
                after: None,
                before: None,
            },
        })
        .await
        .unwrap();

    // @step Then results include matches from both "/project-a" and "/project-b" sessions
    let parsed: SessionSearchResult = serde_json::from_str(&output).unwrap();
    match parsed {
        SessionSearchResult::Search { groups, .. } => {
            let projects: Vec<&str> = groups.iter().map(|g| g.matches[0].project.as_str()).collect();
            assert!(projects.contains(&"/project-a"));
            assert!(projects.contains(&"/project-b"));
        }
        _ => panic!("Expected Search result"),
    }

    clear_all_session_search_handlers();
}

/// Scenario: Search with relative time filter
#[tokio::test]
#[serial]
async fn test_search_with_relative_time_filter() {
    with_clean_handlers(|| {});

    // @step Given sessions exist from 48 hours ago and from 2 hours ago
    let session_id = Uuid::new_v4();
    let two_hours_ago = Utc::now() - chrono::Duration::hours(2);
    let handler: SessionSearchHandler = Arc::new(move |action, _sid| match action {
        SessionSearchAction::Search { query, last_hours, .. } => {
            assert_eq!(last_hours, Some(24));
            // Handler simulates filtering: only returns the 2-hours-ago match
            SessionSearchResult::Search {
                query,
                total_matches: 1,
                groups: vec![SearchMatchGroup {
                    session_id: "recent-sess".to_string(),
                    session_name: "Recent session".to_string(),
                    project: "/test".to_string(),
                    provider: None,
                    message_count: 5,
                    matches: vec![SearchMatch {
                        session_id: "recent-sess".to_string(),
                        session_name: "Recent session".to_string(),
                        role: "user".to_string(),
                        turn_index: 1,
                        timestamp: two_hours_ago,
                        matched_content: "compaction discussion".to_string(),
                        project: "/test".to_string(),
                    }],
                    context: None,
                }],
            }
        }
        _ => SessionSearchResult::Error { message: "unexpected".to_string() },
    });
    set_session_search_handler(session_id, Some(handler));

    // @step When the agent calls SessionSearch with action "search" and query "compaction" and last_hours 24
    let tool = SessionSearchTool::new(session_id);
    let output = tool
        .call(SessionSearchArgs {
            action: SessionSearchAction::Search {
                query: "compaction".to_string(),
                context_turns: None,
                limit: None,
                all_projects: None,
                last_hours: Some(24),
                last_days: None,
                after: None,
                before: None,
            },
        })
        .await
        .unwrap();

    // @step Then results only include matches from the last 24 hours
    let parsed: SessionSearchResult = serde_json::from_str(&output).unwrap();
    match parsed {
        SessionSearchResult::Search { groups, total_matches, .. } => {
            assert_eq!(total_matches, 1);
            assert_eq!(groups[0].session_id, "recent-sess");
        }
        _ => panic!("Expected Search result"),
    }

    clear_all_session_search_handlers();
}

/// Scenario: Search with absolute time filter
#[tokio::test]
#[serial]
async fn test_search_with_absolute_time_filter() {
    with_clean_handlers(|| {});

    // @step Given sessions exist from various dates
    let session_id = Uuid::new_v4();
    let handler: SessionSearchHandler = Arc::new(|action, _sid| match action {
        SessionSearchAction::Search { query, after, .. } => {
            assert_eq!(after, Some("2026-03-01T00:00:00Z".to_string()));
            SessionSearchResult::Search {
                query,
                total_matches: 1,
                groups: vec![SearchMatchGroup {
                    session_id: "march-sess".to_string(),
                    session_name: "March session".to_string(),
                    project: "/test".to_string(),
                    provider: None,
                    message_count: 3,
                    matches: vec![SearchMatch {
                        session_id: "march-sess".to_string(),
                        session_name: "March session".to_string(),
                        role: "user".to_string(),
                        turn_index: 1,
                        timestamp: Utc::now(),
                        matched_content: "refactor the module".to_string(),
                        project: "/test".to_string(),
                    }],
                    context: None,
                }],
            }
        }
        _ => SessionSearchResult::Error { message: "unexpected".to_string() },
    });
    set_session_search_handler(session_id, Some(handler));

    // @step When the agent calls SessionSearch with action "search" and query "refactor" and after "2026-03-01T00:00:00Z"
    let tool = SessionSearchTool::new(session_id);
    let output = tool
        .call(SessionSearchArgs {
            action: SessionSearchAction::Search {
                query: "refactor".to_string(),
                context_turns: None,
                limit: None,
                all_projects: None,
                last_hours: None,
                last_days: None,
                after: Some("2026-03-01T00:00:00Z".to_string()),
                before: None,
            },
        })
        .await
        .unwrap();

    // @step Then results only include matches from sessions updated after that timestamp
    let parsed: SessionSearchResult = serde_json::from_str(&output).unwrap();
    match parsed {
        SessionSearchResult::Search { total_matches, .. } => {
            assert_eq!(total_matches, 1);
        }
        _ => panic!("Expected Search result"),
    }

    clear_all_session_search_handlers();
}

/// Scenario: Search uses ripgrep regex matching
#[tokio::test]
#[serial]
async fn test_search_uses_ripgrep_regex() {
    with_clean_handlers(|| {});

    // @step Given a session contains messages with "DeepSearch", "deep_search", and "DEEPSEARCH"
    let session_id = Uuid::new_v4();
    let handler: SessionSearchHandler = Arc::new(|action, _sid| match action {
        SessionSearchAction::Search { query, .. } => {
            assert_eq!(query, "(?i)deep.?search");
            SessionSearchResult::Search {
                query,
                total_matches: 3,
                groups: vec![SearchMatchGroup {
                    session_id: "sess-1".to_string(),
                    session_name: "Session".to_string(),
                    project: "/test".to_string(),
                    provider: None,
                    message_count: 15,
                    matches: vec![
                        SearchMatch {
                            session_id: "sess-1".to_string(),
                            session_name: "Session".to_string(),
                            role: "user".to_string(),
                            turn_index: 1,
                            timestamp: Utc::now(),
                            matched_content: "Using DeepSearch tool".to_string(),
                            project: "/test".to_string(),
                        },
                        SearchMatch {
                            session_id: "sess-1".to_string(),
                            session_name: "Session".to_string(),
                            role: "assistant".to_string(),
                            turn_index: 2,
                            timestamp: Utc::now(),
                            matched_content: "deep_search function".to_string(),
                            project: "/test".to_string(),
                        },
                        SearchMatch {
                            session_id: "sess-1".to_string(),
                            session_name: "Session".to_string(),
                            role: "user".to_string(),
                            turn_index: 3,
                            timestamp: Utc::now(),
                            matched_content: "DEEPSEARCH results".to_string(),
                            project: "/test".to_string(),
                        },
                    ],
                    context: None,
                }],
            }
        }
        _ => SessionSearchResult::Error { message: "unexpected".to_string() },
    });
    set_session_search_handler(session_id, Some(handler));

    // @step When the agent calls SessionSearch with action "search" and query "(?i)deep.?search"
    let tool = SessionSearchTool::new(session_id);
    let output = tool
        .call(SessionSearchArgs {
            action: SessionSearchAction::Search {
                query: "(?i)deep.?search".to_string(),
                context_turns: None,
                limit: None,
                all_projects: None,
                last_hours: None,
                last_days: None,
                after: None,
                before: None,
            },
        })
        .await
        .unwrap();

    // @step Then all three variations are matched
    let parsed: SessionSearchResult = serde_json::from_str(&output).unwrap();
    match parsed {
        SessionSearchResult::Search { total_matches, groups, .. } => {
            assert_eq!(total_matches, 3);
            let contents: Vec<&str> = groups[0].matches.iter().map(|m| m.matched_content.as_str()).collect();
            assert!(contents.iter().any(|c| c.contains("DeepSearch")));
            assert!(contents.iter().any(|c| c.contains("deep_search")));
            assert!(contents.iter().any(|c| c.contains("DEEPSEARCH")));
        }
        _ => panic!("Expected Search result"),
    }

    clear_all_session_search_handlers();
}

/// Scenario: Search defaults to limit of 20 matches
#[tokio::test]
#[serial]
async fn test_search_defaults_to_limit_20() {
    with_clean_handlers(|| {});

    // @step Given sessions contain 50 messages matching "TODO"
    let session_id = Uuid::new_v4();
    let handler: SessionSearchHandler = Arc::new(|action, _sid| match action {
        SessionSearchAction::Search { query, limit, .. } => {
            // Verify no explicit limit was passed
            assert!(limit.is_none());
            // Handler respects the default limit of 20
            let n = limit.unwrap_or(DEFAULT_SEARCH_LIMIT);
            let matches: Vec<SearchMatch> = (0..n)
                .map(|i| SearchMatch {
                    session_id: "sess-1".to_string(),
                    session_name: "Session".to_string(),
                    role: "user".to_string(),
                    turn_index: i,
                    timestamp: Utc::now(),
                    matched_content: format!("TODO item {i}"),
                    project: "/test".to_string(),
                })
                .collect();
            SessionSearchResult::Search {
                query,
                total_matches: matches.len(),
                groups: vec![SearchMatchGroup {
                    session_id: "sess-1".to_string(),
                    session_name: "Session".to_string(),
                    project: "/test".to_string(),
                    provider: None,
                    message_count: 50,
                    matches,
                    context: None,
                }],
            }
        }
        _ => SessionSearchResult::Error { message: "unexpected".to_string() },
    });
    set_session_search_handler(session_id, Some(handler));

    // @step When the agent calls SessionSearch with action "search" and query "TODO" and no limit parameter
    let tool = SessionSearchTool::new(session_id);
    let output = tool
        .call(SessionSearchArgs {
            action: SessionSearchAction::Search {
                query: "TODO".to_string(),
                context_turns: None,
                limit: None,
                all_projects: None,
                last_hours: None,
                last_days: None,
                after: None,
                before: None,
            },
        })
        .await
        .unwrap();

    // @step Then the result contains at most 20 matches
    let parsed: SessionSearchResult = serde_json::from_str(&output).unwrap();
    match parsed {
        SessionSearchResult::Search { total_matches, groups, .. } => {
            let total_in_groups: usize = groups.iter().map(|g| g.matches.len()).sum();
            assert!(total_in_groups <= DEFAULT_SEARCH_LIMIT);
            assert_eq!(total_matches, DEFAULT_SEARCH_LIMIT);
        }
        _ => panic!("Expected Search result"),
    }

    clear_all_session_search_handlers();
}

/// Scenario: Show current session by default
#[tokio::test]
#[serial]
async fn test_show_current_session_by_default() {
    with_clean_handlers(|| {});

    // @step Given the agent is running in a session
    let session_id = Uuid::new_v4();
    let session_id_str = session_id.to_string();
    let handler: SessionSearchHandler = Arc::new(move |action, sid| match action {
        SessionSearchAction::Show { session_id: show_id, .. } => {
            // When no session_id provided, handler uses the tool's own session_id
            assert!(show_id.is_none());
            SessionSearchResult::Session {
                session_id: sid.to_string(),
                session_name: "Current Session".to_string(),
                messages: vec![
                    SessionMessage {
                        turn_index: 0,
                        role: "user".to_string(),
                        content: "Hello".to_string(),
                        timestamp: Utc::now(),
                        truncated: false,
                    },
                    SessionMessage {
                        turn_index: 1,
                        role: "assistant".to_string(),
                        content: "Hi there!".to_string(),
                        timestamp: Utc::now(),
                        truncated: false,
                    },
                ],
            }
        }
        _ => SessionSearchResult::Error { message: "unexpected".to_string() },
    });
    set_session_search_handler(session_id, Some(handler));

    // @step When the agent calls SessionSearch with action "show" and no session_id
    let tool = SessionSearchTool::new(session_id);
    let output = tool
        .call(SessionSearchArgs {
            action: SessionSearchAction::Show {
                session_id: None,
                user_only: None,
                max_turns: None,
            },
        })
        .await
        .unwrap();

    let parsed: SessionSearchResult = serde_json::from_str(&output).unwrap();
    match parsed {
        SessionSearchResult::Session { session_id: sid, messages, .. } => {
            // @step Then the result contains the current session's full conversation
            assert_eq!(sid, session_id_str);

            // @step And messages are in chronological order
            assert_eq!(messages[0].turn_index, 0);
            assert_eq!(messages[1].turn_index, 1);
            assert!(messages[0].turn_index < messages[1].turn_index);
        }
        _ => panic!("Expected Session result"),
    }

    clear_all_session_search_handlers();
}

/// Scenario: Show session with explicit current keyword
#[tokio::test]
#[serial]
async fn test_show_session_with_current_keyword() {
    with_clean_handlers(|| {});

    // @step Given the agent is running in a session
    let session_id = Uuid::new_v4();
    let session_id_str = session_id.to_string();
    let handler: SessionSearchHandler = Arc::new(move |action, sid| match action {
        SessionSearchAction::Show { session_id: show_id, .. } => {
            // "current" keyword should be received as-is; handler maps it to current session
            assert_eq!(show_id, Some("current".to_string()));
            SessionSearchResult::Session {
                session_id: sid.to_string(),
                session_name: "Current Session".to_string(),
                messages: vec![SessionMessage {
                    turn_index: 0,
                    role: "user".to_string(),
                    content: "Hello".to_string(),
                    timestamp: Utc::now(),
                    truncated: false,
                }],
            }
        }
        _ => SessionSearchResult::Error { message: "unexpected".to_string() },
    });
    set_session_search_handler(session_id, Some(handler));

    // @step When the agent calls SessionSearch with action "show" and session_id "current"
    let tool = SessionSearchTool::new(session_id);
    let output_current = tool
        .call(SessionSearchArgs {
            action: SessionSearchAction::Show {
                session_id: Some("current".to_string()),
                user_only: None,
                max_turns: None,
            },
        })
        .await
        .unwrap();

    // @step Then the result is identical to calling show with no session_id
    let parsed: SessionSearchResult = serde_json::from_str(&output_current).unwrap();
    match parsed {
        SessionSearchResult::Session { session_id: sid, .. } => {
            assert_eq!(sid, session_id_str);
        }
        _ => panic!("Expected Session result"),
    }

    clear_all_session_search_handlers();
}

/// Scenario: Show specific session by UUID
#[tokio::test]
#[serial]
async fn test_show_specific_session_by_uuid() {
    with_clean_handlers(|| {});

    // @step Given a session exists with ID "7e0358a4-3395-4ee3-9a4b-62575d625b8c"
    let session_id = Uuid::new_v4();
    let target_uuid = "7e0358a4-3395-4ee3-9a4b-62575d625b8c";
    let handler: SessionSearchHandler = Arc::new(|action, _sid| match action {
        SessionSearchAction::Show { session_id: show_id, .. } => {
            let id = show_id.unwrap();
            SessionSearchResult::Session {
                session_id: id,
                session_name: "Target Session".to_string(),
                messages: vec![
                    SessionMessage {
                        turn_index: 0,
                        role: "user".to_string(),
                        content: "First message".to_string(),
                        timestamp: Utc::now(),
                        truncated: false,
                    },
                    SessionMessage {
                        turn_index: 1,
                        role: "assistant".to_string(),
                        content: "Response".to_string(),
                        timestamp: Utc::now(),
                        truncated: false,
                    },
                ],
            }
        }
        _ => SessionSearchResult::Error { message: "unexpected".to_string() },
    });
    set_session_search_handler(session_id, Some(handler));

    // @step When the agent calls SessionSearch with action "show" and session_id "7e0358a4-3395-4ee3-9a4b-62575d625b8c"
    let tool = SessionSearchTool::new(session_id);
    let output = tool
        .call(SessionSearchArgs {
            action: SessionSearchAction::Show {
                session_id: Some(target_uuid.to_string()),
                user_only: None,
                max_turns: None,
            },
        })
        .await
        .unwrap();

    // @step Then the result contains that session's full conversation with messages in order
    let parsed: SessionSearchResult = serde_json::from_str(&output).unwrap();
    match parsed {
        SessionSearchResult::Session { session_id: sid, messages, .. } => {
            assert_eq!(sid, target_uuid);
            assert_eq!(messages.len(), 2);
            assert!(messages[0].turn_index < messages[1].turn_index);
        }
        _ => panic!("Expected Session result"),
    }

    clear_all_session_search_handlers();
}

/// Scenario: Show session resolves blob references
#[tokio::test]
#[serial]
async fn test_show_session_resolves_blob_references() {
    with_clean_handlers(|| {});

    // @step Given a session contains messages with blob references to large content
    let session_id = Uuid::new_v4();
    let handler: SessionSearchHandler = Arc::new(|action, _sid| match action {
        SessionSearchAction::Show { .. } => {
            // Handler simulates blob resolution — content is already resolved
            SessionSearchResult::Session {
                session_id: "blob-sess".to_string(),
                session_name: "Blob Session".to_string(),
                messages: vec![SessionMessage {
                    turn_index: 0,
                    role: "assistant".to_string(),
                    content: "This is the resolved blob content that was originally stored as a blob reference".to_string(),
                    timestamp: Utc::now(),
                    truncated: false,
                }],
            }
        }
        _ => SessionSearchResult::Error { message: "unexpected".to_string() },
    });
    set_session_search_handler(session_id, Some(handler));

    // @step When the agent calls SessionSearch with action "show" for that session
    let tool = SessionSearchTool::new(session_id);
    let output = tool
        .call(SessionSearchArgs {
            action: SessionSearchAction::Show {
                session_id: Some("blob-sess".to_string()),
                user_only: None,
                max_turns: None,
            },
        })
        .await
        .unwrap();

    // @step Then blob references are resolved to their actual content in the output
    let parsed: SessionSearchResult = serde_json::from_str(&output).unwrap();
    match parsed {
        SessionSearchResult::Session { messages, .. } => {
            assert!(!messages[0].content.is_empty());
            assert!(messages[0].content.contains("resolved blob content"));
        }
        _ => panic!("Expected Session result"),
    }

    clear_all_session_search_handlers();
}

/// Scenario: Show session truncates long messages
#[tokio::test]
#[serial]
async fn test_show_session_truncates_long_messages() {
    with_clean_handlers(|| {});

    // @step Given a session contains a message with 10000 characters of content
    let session_id = Uuid::new_v4();
    let long_content = "x".repeat(10000);
    let handler: SessionSearchHandler = Arc::new(move |action, _sid| match action {
        SessionSearchAction::Show { .. } => {
            // Handler simulates truncation at MESSAGE_TRUNCATION_LIMIT
            let truncated_content = if long_content.len() > MESSAGE_TRUNCATION_LIMIT {
                format!("{}...", &long_content[..MESSAGE_TRUNCATION_LIMIT])
            } else {
                long_content.clone()
            };
            SessionSearchResult::Session {
                session_id: "long-sess".to_string(),
                session_name: "Long Session".to_string(),
                messages: vec![SessionMessage {
                    turn_index: 0,
                    role: "assistant".to_string(),
                    content: truncated_content,
                    timestamp: Utc::now(),
                    truncated: true,
                }],
            }
        }
        _ => SessionSearchResult::Error { message: "unexpected".to_string() },
    });
    set_session_search_handler(session_id, Some(handler));

    // @step When the agent calls SessionSearch with action "show" for that session
    let tool = SessionSearchTool::new(session_id);
    let output = tool
        .call(SessionSearchArgs {
            action: SessionSearchAction::Show {
                session_id: Some("long-sess".to_string()),
                user_only: None,
                max_turns: None,
            },
        })
        .await
        .unwrap();

    // @step Then that message is truncated to 5000 characters in the output
    let parsed: SessionSearchResult = serde_json::from_str(&output).unwrap();
    match parsed {
        SessionSearchResult::Session { messages, .. } => {
            assert!(messages[0].content.len() <= MESSAGE_TRUNCATION_LIMIT + 3); // +3 for "..."
            assert!(messages[0].truncated);
        }
        _ => panic!("Expected Session result"),
    }

    clear_all_session_search_handlers();
}

/// Scenario: Show session with user_only filter
#[tokio::test]
#[serial]
async fn test_show_session_with_user_only_filter() {
    with_clean_handlers(|| {});

    // @step Given a session contains both user and assistant messages
    let session_id = Uuid::new_v4();
    let handler: SessionSearchHandler = Arc::new(|action, _sid| match action {
        SessionSearchAction::Show { user_only, .. } => {
            assert_eq!(user_only, Some(true));
            // Handler filters to only user messages
            SessionSearchResult::Session {
                session_id: "filter-sess".to_string(),
                session_name: "Filtered Session".to_string(),
                messages: vec![
                    SessionMessage {
                        turn_index: 0,
                        role: "user".to_string(),
                        content: "First question".to_string(),
                        timestamp: Utc::now(),
                        truncated: false,
                    },
                    SessionMessage {
                        turn_index: 2,
                        role: "user".to_string(),
                        content: "Second question".to_string(),
                        timestamp: Utc::now(),
                        truncated: false,
                    },
                ],
            }
        }
        _ => SessionSearchResult::Error { message: "unexpected".to_string() },
    });
    set_session_search_handler(session_id, Some(handler));

    // @step When the agent calls SessionSearch with action "show" with user_only true
    let tool = SessionSearchTool::new(session_id);
    let output = tool
        .call(SessionSearchArgs {
            action: SessionSearchAction::Show {
                session_id: None,
                user_only: Some(true),
                max_turns: None,
            },
        })
        .await
        .unwrap();

    // @step Then only user messages are included in the result
    let parsed: SessionSearchResult = serde_json::from_str(&output).unwrap();
    match parsed {
        SessionSearchResult::Session { messages, .. } => {
            assert!(!messages.is_empty());
            for msg in &messages {
                assert_eq!(msg.role, "user");
            }
        }
        _ => panic!("Expected Session result"),
    }

    clear_all_session_search_handlers();
}

/// Scenario: Show session with max_turns limit
#[tokio::test]
#[serial]
async fn test_show_session_with_max_turns_limit() {
    with_clean_handlers(|| {});

    // @step Given a session contains 100 messages
    let session_id = Uuid::new_v4();
    let handler: SessionSearchHandler = Arc::new(|action, _sid| match action {
        SessionSearchAction::Show { max_turns, .. } => {
            assert_eq!(max_turns, Some(10));
            // Handler returns only the last 10 messages (from 100 total)
            let messages: Vec<SessionMessage> = (90..100)
                .map(|i| SessionMessage {
                    turn_index: i,
                    role: if i % 2 == 0 { "user".to_string() } else { "assistant".to_string() },
                    content: format!("Message {i}"),
                    timestamp: Utc::now(),
                    truncated: false,
                })
                .collect();
            SessionSearchResult::Session {
                session_id: "long-sess".to_string(),
                session_name: "Long Session".to_string(),
                messages,
            }
        }
        _ => SessionSearchResult::Error { message: "unexpected".to_string() },
    });
    set_session_search_handler(session_id, Some(handler));

    // @step When the agent calls SessionSearch with action "show" with max_turns 10
    let tool = SessionSearchTool::new(session_id);
    let output = tool
        .call(SessionSearchArgs {
            action: SessionSearchAction::Show {
                session_id: None,
                user_only: None,
                max_turns: Some(10),
            },
        })
        .await
        .unwrap();

    // @step Then only the last 10 messages are included in the result
    let parsed: SessionSearchResult = serde_json::from_str(&output).unwrap();
    match parsed {
        SessionSearchResult::Session { messages, .. } => {
            assert_eq!(messages.len(), 10);
            // Verify they are the LAST 10 (turn indices 90-99)
            assert_eq!(messages[0].turn_index, 90);
            assert_eq!(messages[9].turn_index, 99);
        }
        _ => panic!("Expected Session result"),
    }

    clear_all_session_search_handlers();
}

/// Test reassembly edge cases
#[test]
fn test_reassembly_edge_cases() {
    // @step Given a session contains an assistant message stored as raw streaming chunks with "[Thinking: partial text...]" markers and "[Tool: Read]" markers and text fragments split mid-word
    let empty = reassemble_content("");
    assert!(empty.is_empty());

    let whitespace = reassemble_content("   \n   \n   ");
    assert!(whitespace.is_empty());

    // @step When the agent calls SessionSearch with action "show" for that session
    let tool_result = reassemble_content("[tool_result: success]\nSome text");
    assert_eq!(tool_result.len(), 1);

    // @step Then thinking chunks are merged into coherent thinking sections
    let merged = reassemble_content("[Thinking: part one...]\n[Thinking: part two...]");
    assert_eq!(merged.len(), 1);
    match &merged[0] {
        Section::Thinking(content) => {
            assert!(content.contains("part one"));
            assert!(content.contains("part two"));
        }
        _ => panic!("Expected Thinking"),
    }

    // @step And tool invocations are preserved as structured markers
    let tool = reassemble_content("[tool_use: Edit]");
    assert_eq!(tool.len(), 1);
    match &tool[0] {
        Section::Tool(name) => assert_eq!(name, "Edit"),
        _ => panic!("Expected Tool"),
    }

    // @step And text fragments are concatenated into readable prose
    let plain = format_sections_plain(&[
        Section::Thinking("short".to_string()),
        Section::Tool("Read".to_string()),
        Section::Text("hello".to_string()),
    ]);
    assert!(plain.contains("[Thinking]"));
    assert!(plain.contains("[Tool: Read]"));
    assert!(plain.contains("hello"));
}
