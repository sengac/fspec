#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/unified-shift-arrow-navigation.feature
//!
//! Tests for VIEWNV-001: Unified Shift+Arrow Navigation
//!
//! These tests verify that session navigation (get_next, get_prev, get_first)
//! works correctly based on the active session tracking.

use indexmap::IndexMap;
use std::sync::RwLock;
use uuid::Uuid;

/// Minimal mock of SessionManager's navigation-relevant state
struct MockSessionManager {
    sessions: RwLock<IndexMap<Uuid, String>>, // id -> name (simplified)
    active_session_id: RwLock<Option<Uuid>>,
}

impl MockSessionManager {
    fn new() -> Self {
        Self {
            sessions: RwLock::new(IndexMap::new()),
            active_session_id: RwLock::new(None),
        }
    }

    fn create_session(&self, name: &str) -> Uuid {
        let id = Uuid::new_v4();
        self.sessions.write().unwrap().insert(id, name.to_string());
        id
    }

    fn set_active(&self, id: Uuid) {
        *self.active_session_id.write().unwrap() = Some(id);
    }

    fn clear_active(&self) {
        *self.active_session_id.write().unwrap() = None;
    }

    fn get_active(&self) -> Option<Uuid> {
        *self.active_session_id.read().unwrap()
    }

    fn get_next(&self) -> Option<Uuid> {
        let sessions = self.sessions.read().unwrap();
        let active = self.active_session_id.read().unwrap();

        if sessions.is_empty() {
            return None;
        }

        match *active {
            None => {
                // No active session - return first
                sessions.keys().next().copied()
            }
            Some(active_id) => {
                let keys: Vec<&Uuid> = sessions.keys().collect();
                let current_idx = keys.iter().position(|&&id| id == active_id);

                match current_idx {
                    Some(idx) if idx + 1 < keys.len() => Some(*keys[idx + 1]),
                    _ => None, // At last session or not found
                }
            }
        }
    }

    fn get_prev(&self) -> Option<Uuid> {
        let sessions = self.sessions.read().unwrap();
        let active = self.active_session_id.read().unwrap();

        match *active {
            None => None, // No active session - no previous
            Some(active_id) => {
                let keys: Vec<&Uuid> = sessions.keys().collect();
                let current_idx = keys.iter().position(|&&id| id == active_id);

                match current_idx {
                    Some(idx) if idx > 0 => Some(*keys[idx - 1]),
                    _ => None, // At first session or not found
                }
            }
        }
    }

    fn get_first(&self) -> Option<Uuid> {
        self.sessions.read().unwrap().keys().next().copied()
    }

    fn list_sessions(&self) -> Vec<Uuid> {
        self.sessions.read().unwrap().keys().copied().collect()
    }
}

// =============================================================================
// BASIC NAVIGATION TESTS
// =============================================================================

/// Test: get_next with no sessions returns None
///
/// @step Given an empty session manager
/// @step When get_next is called
/// @step Then it returns None
#[test]
fn test_get_next_no_sessions() {
    let manager = MockSessionManager::new();

    // No sessions exist
    let next = manager.get_next();
    assert!(next.is_none(), "get_next should return None when no sessions exist");
}

/// Test: get_next with no active session returns first session
///
/// @step Given sessions [A, B, C] and no active session
/// @step When get_next is called
/// @step Then it returns session A (the first)
#[test]
fn test_get_next_no_active_returns_first() {
    let manager = MockSessionManager::new();

    let a = manager.create_session("Session A");
    let _b = manager.create_session("Session B");
    let _c = manager.create_session("Session C");

    // No active session (simulating BoardView)
    assert!(manager.get_active().is_none());

    let next = manager.get_next();
    assert_eq!(next, Some(a), "get_next with no active should return first session");
}

/// Test: get_next from first session returns second session
///
/// @step Given sessions [A, B, C] with A active
/// @step When get_next is called
/// @step Then it returns session B
#[test]
fn test_get_next_from_first_returns_second() {
    let manager = MockSessionManager::new();

    let a = manager.create_session("Session A");
    let b = manager.create_session("Session B");
    let _c = manager.create_session("Session C");

    manager.set_active(a);

    let next = manager.get_next();
    assert_eq!(next, Some(b), "get_next from A should return B");
}

/// Test: get_next from middle session returns next session
///
/// @step Given sessions [A, B, C] with B active
/// @step When get_next is called
/// @step Then it returns session C
#[test]
fn test_get_next_from_middle_returns_next() {
    let manager = MockSessionManager::new();

    let _a = manager.create_session("Session A");
    let b = manager.create_session("Session B");
    let c = manager.create_session("Session C");

    manager.set_active(b);

    let next = manager.get_next();
    assert_eq!(next, Some(c), "get_next from B should return C");
}

/// Test: get_next from last session returns None
///
/// @step Given sessions [A, B, C] with C active
/// @step When get_next is called
/// @step Then it returns None (should show create dialog)
#[test]
fn test_get_next_from_last_returns_none() {
    let manager = MockSessionManager::new();

    let _a = manager.create_session("Session A");
    let _b = manager.create_session("Session B");
    let c = manager.create_session("Session C");

    manager.set_active(c);

    let next = manager.get_next();
    assert!(next.is_none(), "get_next from last session should return None");
}

/// Test: get_prev with no active session returns None
///
/// @step Given sessions [A, B, C] and no active session (BoardView)
/// @step When get_prev is called
/// @step Then it returns None (stay on board)
#[test]
fn test_get_prev_no_active_returns_none() {
    let manager = MockSessionManager::new();

    let _a = manager.create_session("Session A");
    let _b = manager.create_session("Session B");

    // No active session
    assert!(manager.get_active().is_none());

    let prev = manager.get_prev();
    assert!(prev.is_none(), "get_prev with no active should return None");
}

/// Test: get_prev from first session returns None (go to board)
///
/// @step Given sessions [A, B, C] with A active
/// @step When get_prev is called
/// @step Then it returns None (should go to board)
#[test]
fn test_get_prev_from_first_returns_none() {
    let manager = MockSessionManager::new();

    let a = manager.create_session("Session A");
    let _b = manager.create_session("Session B");
    let _c = manager.create_session("Session C");

    manager.set_active(a);

    let prev = manager.get_prev();
    assert!(prev.is_none(), "get_prev from first session should return None (go to board)");
}

/// Test: get_prev from second session returns first session
///
/// @step Given sessions [A, B, C] with B active
/// @step When get_prev is called
/// @step Then it returns session A
#[test]
fn test_get_prev_from_second_returns_first() {
    let manager = MockSessionManager::new();

    let a = manager.create_session("Session A");
    let b = manager.create_session("Session B");
    let _c = manager.create_session("Session C");

    manager.set_active(b);

    let prev = manager.get_prev();
    assert_eq!(prev, Some(a), "get_prev from B should return A");
}

/// Test: get_prev from last session returns previous session
///
/// @step Given sessions [A, B, C] with C active
/// @step When get_prev is called
/// @step Then it returns session B
#[test]
fn test_get_prev_from_last_returns_previous() {
    let manager = MockSessionManager::new();

    let _a = manager.create_session("Session A");
    let b = manager.create_session("Session B");
    let c = manager.create_session("Session C");

    manager.set_active(c);

    let prev = manager.get_prev();
    assert_eq!(prev, Some(b), "get_prev from C should return B");
}

// =============================================================================
// FULL NAVIGATION FLOW TESTS
// =============================================================================

/// Test: Complete navigation flow - Board → Sessions → Board
///
/// This simulates the user's exact scenario:
/// 1. Create session A (via typing)
/// 2. Create session B (via Shift+Right → create dialog → confirm → type)
/// 3. Navigate back to board using Shift+Left twice
/// 4. Navigate forward to session A using Shift+Right
///
/// @step Given I create session A from board
/// @step And I create session B by pressing Shift+Right and confirming
/// @step When I press Shift+Left twice to return to board
/// @step And I press Shift+Right
/// @step Then I should navigate to session A (not see create dialog)
#[test]
fn test_full_navigation_flow_board_sessions_board() {
    let manager = MockSessionManager::new();

    // === Step 1: User creates session A ===
    // User is on BoardView, types a message, session A is created
    let session_a = manager.create_session("Session A");
    manager.set_active(session_a); // sessionAttach sets active

    // Verify state
    assert_eq!(manager.get_active(), Some(session_a));
    assert_eq!(manager.list_sessions(), vec![session_a]);

    // === Step 2: User creates session B ===
    // User presses Shift+Right from session A
    // get_next(active=A) returns None (A is last)
    let next = manager.get_next();
    assert!(next.is_none(), "A is the last session, get_next should return None");

    // Create dialog shows, user confirms, types a message, session B is created
    let session_b = manager.create_session("Session B");
    manager.set_active(session_b); // sessionAttach sets active

    // Verify state
    assert_eq!(manager.get_active(), Some(session_b));
    assert_eq!(manager.list_sessions(), vec![session_a, session_b]);

    // === Step 3: User navigates back to board ===
    // User presses Shift+Left from session B
    let prev = manager.get_prev();
    assert_eq!(prev, Some(session_a), "get_prev from B should return A");

    // Simulate navigation to A
    // sessionDetach(B) clears active, then sessionAttach(A) sets active
    manager.clear_active();
    manager.set_active(session_a);

    assert_eq!(manager.get_active(), Some(session_a));

    // User presses Shift+Left from session A
    let prev = manager.get_prev();
    assert!(prev.is_none(), "get_prev from A (first) should return None");

    // Navigation goes to board
    // sessionDetach(A) clears active, clearActiveSession also clears
    manager.clear_active();

    assert!(manager.get_active().is_none(), "Active should be None when on board");

    // === Step 4: User presses Shift+Right from board ===
    let next = manager.get_next();
    assert_eq!(
        next,
        Some(session_a),
        "get_next from board should return first session (A), NOT show create dialog"
    );

    // Verify sessions still exist
    assert_eq!(manager.list_sessions(), vec![session_a, session_b]);
}

/// Test: IndexMap maintains insertion order
///
/// @step Given sessions created in order A, B, C
/// @step When listing sessions
/// @step Then they appear in insertion order [A, B, C]
#[test]
fn test_indexmap_insertion_order() {
    let manager = MockSessionManager::new();

    let a = manager.create_session("A");
    let b = manager.create_session("B");
    let c = manager.create_session("C");

    let sessions = manager.list_sessions();
    assert_eq!(sessions, vec![a, b, c], "Sessions should be in insertion order");
}

/// Test: get_first always returns first session regardless of active
///
/// @step Given sessions [A, B, C] with B active
/// @step When get_first is called
/// @step Then it returns A
#[test]
fn test_get_first_ignores_active() {
    let manager = MockSessionManager::new();

    let a = manager.create_session("A");
    let b = manager.create_session("B");
    let _c = manager.create_session("C");

    manager.set_active(b);

    let first = manager.get_first();
    assert_eq!(first, Some(a), "get_first should always return first session");
}

/// Test: Active session not found in list returns first on get_next
///
/// This tests the edge case where the active session ID doesn't exist
/// in the sessions list (shouldn't happen but we should handle it).
///
/// @step Given sessions [A, B] with an invalid active ID
/// @step When get_next is called
/// @step Then it returns None (or could return first - implementation choice)
#[test]
fn test_active_not_in_list() {
    let manager = MockSessionManager::new();

    let _a = manager.create_session("A");
    let _b = manager.create_session("B");

    // Set an active ID that doesn't exist in sessions
    let fake_id = Uuid::new_v4();
    manager.set_active(fake_id);

    let next = manager.get_next();
    // Current implementation returns None when active not found
    assert!(next.is_none(), "get_next with invalid active should return None");
}

/// Test: Navigation after clearing active session
///
/// @step Given sessions [A, B] with B active
/// @step When active is cleared
/// @step Then get_next returns A (first session)
#[test]
fn test_navigation_after_clear_active() {
    let manager = MockSessionManager::new();

    let a = manager.create_session("A");
    let b = manager.create_session("B");

    manager.set_active(b);
    assert_eq!(manager.get_active(), Some(b));

    // Clear active (like going to board)
    manager.clear_active();
    assert!(manager.get_active().is_none());

    // get_next should return first session
    let next = manager.get_next();
    assert_eq!(next, Some(a), "get_next after clear_active should return first session");
}

/// Test: Single session navigation
///
/// @step Given only session A exists
/// @step When A is active and get_next is called
/// @step Then it returns None (show create dialog)
/// @step And when get_prev is called, it returns None (go to board)
#[test]
fn test_single_session_navigation() {
    let manager = MockSessionManager::new();

    let a = manager.create_session("A");
    manager.set_active(a);

    // Can't go next (only session)
    assert!(manager.get_next().is_none(), "get_next from only session should return None");

    // Can't go prev (first session)
    assert!(manager.get_prev().is_none(), "get_prev from first/only session should return None");
}

// =============================================================================
// STRESS TESTS
// =============================================================================

/// Test: Navigation with many sessions
///
/// @step Given 100 sessions
/// @step When navigating forward from first to last
/// @step Then each get_next returns the correct next session
#[test]
fn test_navigation_many_sessions() {
    let manager = MockSessionManager::new();

    // Create 100 sessions
    let mut session_ids = Vec::new();
    for i in 0..100 {
        let id = manager.create_session(&format!("Session {}", i));
        session_ids.push(id);
    }

    // Start at first session
    manager.set_active(session_ids[0]);

    // Navigate forward through all sessions
    for i in 0..99 {
        let next = manager.get_next();
        assert_eq!(
            next,
            Some(session_ids[i + 1]),
            "get_next from session {} should return session {}",
            i,
            i + 1
        );
        manager.set_active(session_ids[i + 1]);
    }

    // At last session, get_next should return None
    assert!(manager.get_next().is_none(), "get_next from last session should return None");
}
