#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/unified-shift-arrow-navigation-across-boardview-agentview-and-splitpaneview.feature
//!
//! Tests for VIEWNV-001: Hierarchy-aware session navigation
//!
//! These tests verify that the navigation logic correctly traverses the session
//! hierarchy: Board → Session → Watchers → Session → Watchers → ... → Create Dialog
//!
//! The navigation should respect the parent-watcher relationships stored in WatchGraph.

use indexmap::IndexMap;
use std::sync::Arc;
use uuid::Uuid;

/// Mock BackgroundSession for testing (minimal struct)
struct MockBackgroundSession {
    id: Uuid,
    name: String,
}

impl MockBackgroundSession {
    fn new(id: Uuid, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
        }
    }
}

/// Mock WatchGraph for testing
struct MockWatchGraph {
    parent_to_watchers: std::collections::HashMap<Uuid, Vec<Uuid>>,
    watcher_to_parent: std::collections::HashMap<Uuid, Uuid>,
}

impl MockWatchGraph {
    fn new() -> Self {
        Self {
            parent_to_watchers: std::collections::HashMap::new(),
            watcher_to_parent: std::collections::HashMap::new(),
        }
    }

    fn add_watcher(&mut self, parent_id: Uuid, watcher_id: Uuid) {
        self.watcher_to_parent.insert(watcher_id, parent_id);
        self.parent_to_watchers
            .entry(parent_id)
            .or_default()
            .push(watcher_id);
    }

    fn get_parent(&self, watcher_id: Uuid) -> Option<Uuid> {
        self.watcher_to_parent.get(&watcher_id).copied()
    }

    fn get_watchers(&self, parent_id: Uuid) -> Vec<Uuid> {
        self.watcher_to_parent
            .iter()
            .filter(|(_, &p)| p == parent_id)
            .map(|(&w, _)| w)
            .collect()
    }
}

/// Build navigation list (same logic as in navigation.rs)
fn build_navigation_list(
    sessions: &IndexMap<Uuid, Arc<MockBackgroundSession>>,
    watch_graph: &MockWatchGraph,
) -> Vec<Uuid> {
    let mut result = Vec::new();

    // Iterate through sessions in insertion order
    for session_id in sessions.keys() {
        // Check if this session is a watcher (has a parent)
        if watch_graph.get_parent(*session_id).is_some() {
            // Skip watchers in the top-level iteration
            // They'll be added after their parent
            continue;
        }

        // Add the parent session
        result.push(*session_id);

        // Add all watchers for this session (in insertion order)
        // We need to iterate sessions to maintain insertion order
        for watcher_id in sessions.keys() {
            if watch_graph.get_parent(*watcher_id) == Some(*session_id) {
                result.push(*watcher_id);
            }
        }
    }

    result
}

/// Navigation target result
#[derive(Debug, Clone, PartialEq)]
enum NavigationTarget {
    Session(Uuid),
    Board,
    CreateDialog,
}

/// Get the next navigation target
fn get_next_target(nav_list: &[Uuid], active_session: Option<Uuid>) -> NavigationTarget {
    if nav_list.is_empty() {
        return NavigationTarget::CreateDialog;
    }

    match active_session {
        None => NavigationTarget::Session(nav_list[0]),
        Some(active_id) => {
            let current_idx = nav_list.iter().position(|&id| id == active_id);
            match current_idx {
                Some(idx) if idx + 1 < nav_list.len() => {
                    NavigationTarget::Session(nav_list[idx + 1])
                }
                _ => NavigationTarget::CreateDialog,
            }
        }
    }
}

/// Get the previous navigation target
fn get_prev_target(nav_list: &[Uuid], active_session: Option<Uuid>) -> NavigationTarget {
    match active_session {
        None => NavigationTarget::Board,
        Some(active_id) => {
            if nav_list.is_empty() {
                return NavigationTarget::Board;
            }
            let current_idx = nav_list.iter().position(|&id| id == active_id);
            match current_idx {
                Some(0) => NavigationTarget::Board,
                Some(idx) => NavigationTarget::Session(nav_list[idx - 1]),
                None => NavigationTarget::Board,
            }
        }
    }
}

// =============================================================================
// VIEWNV-001: Hierarchy Navigation Tests
// =============================================================================

/// Test: Build navigation list with no watchers
///
/// @step Given sessions [A, B, C] with no watchers
/// @step When building navigation list
/// @step Then the list is [A, B, C]
#[test]
fn test_build_nav_list_no_watchers() {
    let mut sessions = IndexMap::new();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();

    sessions.insert(a, Arc::new(MockBackgroundSession::new(a, "A")));
    sessions.insert(b, Arc::new(MockBackgroundSession::new(b, "B")));
    sessions.insert(c, Arc::new(MockBackgroundSession::new(c, "C")));

    let watch_graph = MockWatchGraph::new();
    let nav_list = build_navigation_list(&sessions, &watch_graph);

    assert_eq!(nav_list, vec![a, b, c]);
}

/// Test: Build navigation list with watchers after their parent
///
/// @step Given sessions [A, W1, W2, B] where W1 and W2 are watchers of A
/// @step When building navigation list
/// @step Then the list is [A, W1, W2, B] (watchers grouped after parent)
#[test]
fn test_build_nav_list_with_watchers() {
    let mut sessions = IndexMap::new();
    let a = Uuid::new_v4();
    let w1 = Uuid::new_v4();
    let w2 = Uuid::new_v4();
    let b = Uuid::new_v4();

    // Insert in order: A, W1, W2, B
    sessions.insert(a, Arc::new(MockBackgroundSession::new(a, "A")));
    sessions.insert(w1, Arc::new(MockBackgroundSession::new(w1, "W1")));
    sessions.insert(w2, Arc::new(MockBackgroundSession::new(w2, "W2")));
    sessions.insert(b, Arc::new(MockBackgroundSession::new(b, "B")));

    let mut watch_graph = MockWatchGraph::new();
    watch_graph.add_watcher(a, w1);
    watch_graph.add_watcher(a, w2);

    let nav_list = build_navigation_list(&sessions, &watch_graph);

    // Navigation order: A → W1 → W2 → B
    assert_eq!(nav_list, vec![a, w1, w2, b]);
}

/// Test: Build navigation list with multiple parent sessions and watchers
///
/// @step Given sessions [A, W1, B, W2, C] where W1 is watcher of A and W2 is watcher of B
/// @step When building navigation list
/// @step Then the list is [A, W1, B, W2, C]
#[test]
fn test_build_nav_list_multiple_parents_with_watchers() {
    let mut sessions = IndexMap::new();
    let a = Uuid::new_v4();
    let w1 = Uuid::new_v4();
    let b = Uuid::new_v4();
    let w2 = Uuid::new_v4();
    let c = Uuid::new_v4();

    // Insert in order: A, W1, B, W2, C
    sessions.insert(a, Arc::new(MockBackgroundSession::new(a, "A")));
    sessions.insert(w1, Arc::new(MockBackgroundSession::new(w1, "W1")));
    sessions.insert(b, Arc::new(MockBackgroundSession::new(b, "B")));
    sessions.insert(w2, Arc::new(MockBackgroundSession::new(w2, "W2")));
    sessions.insert(c, Arc::new(MockBackgroundSession::new(c, "C")));

    let mut watch_graph = MockWatchGraph::new();
    watch_graph.add_watcher(a, w1);
    watch_graph.add_watcher(b, w2);

    let nav_list = build_navigation_list(&sessions, &watch_graph);

    // Navigation order: A → W1 → B → W2 → C
    assert_eq!(nav_list, vec![a, w1, b, w2, c]);
}

/// Test: Full navigation cycle with watchers
///
/// VIEWNV-001 Scenario: Shift+Right cycles through all sessions and watchers
///
/// @step Given Session A with watchers W1, W2 and Session B
/// @step When starting from Board
/// @step Then Shift+Right navigates: A → W1 → W2 → B → CreateDialog
#[test]
fn test_full_shift_right_navigation_with_watchers() {
    let mut sessions = IndexMap::new();
    let a = Uuid::new_v4();
    let w1 = Uuid::new_v4();
    let w2 = Uuid::new_v4();
    let b = Uuid::new_v4();

    sessions.insert(a, Arc::new(MockBackgroundSession::new(a, "A")));
    sessions.insert(w1, Arc::new(MockBackgroundSession::new(w1, "W1")));
    sessions.insert(w2, Arc::new(MockBackgroundSession::new(w2, "W2")));
    sessions.insert(b, Arc::new(MockBackgroundSession::new(b, "B")));

    let mut watch_graph = MockWatchGraph::new();
    watch_graph.add_watcher(a, w1);
    watch_graph.add_watcher(a, w2);

    let nav_list = build_navigation_list(&sessions, &watch_graph);

    // Start from board (no active session)
    let mut active: Option<Uuid> = None;
    
    // Shift+Right → A
    let next = get_next_target(&nav_list, active);
    assert_eq!(next, NavigationTarget::Session(a), "From board, should go to A");
    active = Some(a);

    // Shift+Right → W1
    let next = get_next_target(&nav_list, active);
    assert_eq!(next, NavigationTarget::Session(w1), "From A, should go to W1");
    active = Some(w1);

    // Shift+Right → W2
    let next = get_next_target(&nav_list, active);
    assert_eq!(next, NavigationTarget::Session(w2), "From W1, should go to W2");
    active = Some(w2);

    // Shift+Right → B
    let next = get_next_target(&nav_list, active);
    assert_eq!(next, NavigationTarget::Session(b), "From W2, should go to B");
    active = Some(b);

    // Shift+Right → CreateDialog
    let next = get_next_target(&nav_list, active);
    assert_eq!(next, NavigationTarget::CreateDialog, "From B (last), should show create dialog");
}

/// Test: Full backward navigation cycle with watchers
///
/// VIEWNV-001 Scenario: Shift+Left cycles backwards through all sessions and watchers
///
/// @step Given Session A with watchers W1, W2 and Session B
/// @step When starting from B
/// @step Then Shift+Left navigates: B → W2 → W1 → A → Board
#[test]
fn test_full_shift_left_navigation_with_watchers() {
    let mut sessions = IndexMap::new();
    let a = Uuid::new_v4();
    let w1 = Uuid::new_v4();
    let w2 = Uuid::new_v4();
    let b = Uuid::new_v4();

    sessions.insert(a, Arc::new(MockBackgroundSession::new(a, "A")));
    sessions.insert(w1, Arc::new(MockBackgroundSession::new(w1, "W1")));
    sessions.insert(w2, Arc::new(MockBackgroundSession::new(w2, "W2")));
    sessions.insert(b, Arc::new(MockBackgroundSession::new(b, "B")));

    let mut watch_graph = MockWatchGraph::new();
    watch_graph.add_watcher(a, w1);
    watch_graph.add_watcher(a, w2);

    let nav_list = build_navigation_list(&sessions, &watch_graph);

    // Start from B
    let mut active: Option<Uuid> = Some(b);
    
    // Shift+Left → W2
    let prev = get_prev_target(&nav_list, active);
    assert_eq!(prev, NavigationTarget::Session(w2), "From B, should go to W2");
    active = Some(w2);

    // Shift+Left → W1
    let prev = get_prev_target(&nav_list, active);
    assert_eq!(prev, NavigationTarget::Session(w1), "From W2, should go to W1");
    active = Some(w1);

    // Shift+Left → A
    let prev = get_prev_target(&nav_list, active);
    assert_eq!(prev, NavigationTarget::Session(a), "From W1, should go to A");
    active = Some(a);

    // Shift+Left → Board
    let prev = get_prev_target(&nav_list, active);
    assert_eq!(prev, NavigationTarget::Board, "From A (first), should go to board");
}

/// Test: Navigation from watcher to next session (crossing parent boundary)
///
/// VIEWNV-001 Rule [6]: Shift+Right from a watcher navigates to the next sibling watcher,
/// or to the next session if no more siblings
///
/// @step Given Session A with watcher W1, and Session B
/// @step When on W1 (last watcher of A)
/// @step Then Shift+Right navigates to B (next session)
#[test]
fn test_shift_right_from_last_watcher_goes_to_next_session() {
    let mut sessions = IndexMap::new();
    let a = Uuid::new_v4();
    let w1 = Uuid::new_v4();
    let b = Uuid::new_v4();

    sessions.insert(a, Arc::new(MockBackgroundSession::new(a, "A")));
    sessions.insert(w1, Arc::new(MockBackgroundSession::new(w1, "W1")));
    sessions.insert(b, Arc::new(MockBackgroundSession::new(b, "B")));

    let mut watch_graph = MockWatchGraph::new();
    watch_graph.add_watcher(a, w1);

    let nav_list = build_navigation_list(&sessions, &watch_graph);
    assert_eq!(nav_list, vec![a, w1, b], "Navigation list should be A → W1 → B");

    // From W1, should go to B
    let next = get_next_target(&nav_list, Some(w1));
    assert_eq!(next, NavigationTarget::Session(b), "From W1 (last watcher), should go to B");
}

/// Test: Navigation from first watcher to parent
///
/// VIEWNV-001 Rule [5]: Shift+Left from the first watcher of a session navigates to the parent session
///
/// @step Given Session A with watchers W1, W2
/// @step When on W1 (first watcher of A)
/// @step Then Shift+Left navigates to A (parent)
#[test]
fn test_shift_left_from_first_watcher_goes_to_parent() {
    let mut sessions = IndexMap::new();
    let a = Uuid::new_v4();
    let w1 = Uuid::new_v4();
    let w2 = Uuid::new_v4();

    sessions.insert(a, Arc::new(MockBackgroundSession::new(a, "A")));
    sessions.insert(w1, Arc::new(MockBackgroundSession::new(w1, "W1")));
    sessions.insert(w2, Arc::new(MockBackgroundSession::new(w2, "W2")));

    let mut watch_graph = MockWatchGraph::new();
    watch_graph.add_watcher(a, w1);
    watch_graph.add_watcher(a, w2);

    let nav_list = build_navigation_list(&sessions, &watch_graph);
    assert_eq!(nav_list, vec![a, w1, w2], "Navigation list should be A → W1 → W2");

    // From W1, should go to A
    let prev = get_prev_target(&nav_list, Some(w1));
    assert_eq!(prev, NavigationTarget::Session(a), "From W1 (first watcher), should go to A (parent)");
}

/// Test: Last watcher of last session shows create dialog
///
/// VIEWNV-001 Example [8]: From last watcher of last session at right edge: Shift+Right shows create session dialog
///
/// @step Given only Session A with watchers W1, W2
/// @step When on W2 (last watcher of last session)
/// @step Then Shift+Right shows create dialog
#[test]
fn test_shift_right_from_last_watcher_of_last_session_shows_create_dialog() {
    let mut sessions = IndexMap::new();
    let a = Uuid::new_v4();
    let w1 = Uuid::new_v4();
    let w2 = Uuid::new_v4();

    sessions.insert(a, Arc::new(MockBackgroundSession::new(a, "A")));
    sessions.insert(w1, Arc::new(MockBackgroundSession::new(w1, "W1")));
    sessions.insert(w2, Arc::new(MockBackgroundSession::new(w2, "W2")));

    let mut watch_graph = MockWatchGraph::new();
    watch_graph.add_watcher(a, w1);
    watch_graph.add_watcher(a, w2);

    let nav_list = build_navigation_list(&sessions, &watch_graph);
    assert_eq!(nav_list, vec![a, w1, w2], "Navigation list should be A → W1 → W2");

    // From W2 (last watcher of last session), should show create dialog
    let next = get_next_target(&nav_list, Some(w2));
    assert_eq!(next, NavigationTarget::CreateDialog, "From W2 (last), should show create dialog");
}
