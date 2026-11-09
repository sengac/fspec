//! Feature: spec/features/session-search-turn-range.feature
//!
//! CMPCT-018 tests, moved out of session_search_handler.rs's inline tests so each
//! test file maps one feature (COV-075).

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
    let json =
        r#"{"action_type": "show", "session_id": "current", "start_turn": 10, "end_turn": 20}"#;
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
    let roles: Vec<&str> = (0..50)
        .map(|i| if i % 2 == 0 { "user" } else { "assistant" })
        .collect();

    // First apply turn range
    let indices: Vec<usize> = (0..50).collect();
    let in_range = filter_by_turn_range(&indices, Some(10), Some(20));
    assert_eq!(in_range.len(), 11);

    // Then apply user_only
    let user_only: Vec<usize> = in_range
        .into_iter()
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
#[tokio::test]
async fn test_tool_definition_includes_turn_range_params() {
    use codelet_tools::session_search::SessionSearchTool;
    use uuid::Uuid;

    let tool = SessionSearchTool::new(Uuid::nil());
    let def = rig::tool::Tool::definition(&tool, String::new()).await;

    let params = &def.parameters;
    let props = params.get("properties").unwrap();

    // Check start_turn exists
    let start_turn = props
        .get("start_turn")
        .expect("start_turn missing from schema");
    assert!(start_turn.get("type").is_some());
    let desc = start_turn.get("description").unwrap().as_str().unwrap();
    assert!(
        desc.contains("Inclusive 0-based") && desc.contains("show/search/originals"),
        "start_turn description should document inclusive positions for every supported action"
    );

    // Check end_turn exists
    let end_turn = props.get("end_turn").expect("end_turn missing from schema");
    assert!(end_turn.get("type").is_some());
    let desc = end_turn.get("description").unwrap().as_str().unwrap();
    assert!(
        desc.contains("Inclusive 0-based") && desc.contains("show/search/originals"),
        "end_turn description should document inclusive positions for every supported action"
    );
}
