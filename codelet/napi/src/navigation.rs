/**
 * VIEWNV-001: Session Navigation Module
 *
 * Provides hierarchy-aware navigation through sessions and watchers.
 *
 * Navigation order:
 * Board → Session1 → S1.Watcher1 → S1.Watcher2 → Session2 → S2.Watcher1 → ... → Create Dialog
 *
 * Rules:
 * - Shift+Right from session with watchers → first watcher
 * - Shift+Right from session without watchers → next session
 * - Shift+Right from watcher → next sibling, or next session if last sibling
 * - Shift+Left from first watcher → parent session
 * - Shift+Left from watcher → prev sibling, or parent if first sibling
 * - Shift+Left from first session → board
 */
use indexmap::IndexMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::session_manager::{BackgroundSession, WatchGraph};

/// Navigation target result
#[derive(Debug, Clone, PartialEq)]
pub enum NavigationTarget {
    /// Navigate to a specific session
    Session(Uuid),
    /// Navigate to board (go back from first session)
    Board,
    /// Show create session dialog (at the end)
    CreateDialog,
    /// No navigation (stay where you are)
    None,
}

/// Build a flattened navigation list from sessions and watchers.
///
/// The list is ordered: Session1 → S1.Watchers → Session2 → S2.Watchers → ...
/// Each session is followed by its watchers (in creation order).
pub fn build_navigation_list(
    sessions: &IndexMap<Uuid, Arc<BackgroundSession>>,
    watch_graph: &WatchGraph,
) -> Vec<Uuid> {
    let mut result = Vec::new();

    // Iterate through sessions in insertion order
    for session_id in sessions.keys() {
        // Check if this session is a watcher (has a parent)
        let parent = watch_graph.get_parent(*session_id);

        if parent.is_some() {
            continue;
        }

        // Add the parent session
        result.push(*session_id);

        // Add all watchers for this session
        let watchers = watch_graph.get_watchers(*session_id);
        for watcher_id in watchers {
            // Only add if the watcher exists in sessions
            if sessions.contains_key(&watcher_id) {
                result.push(watcher_id);
            }
        }
    }

    result
}

/// Get the next navigation target from the current position.
///
/// - If no active session (BoardView), returns first session
/// - If at a session, returns first watcher or next session
/// - If at a watcher, returns next sibling or next session
/// - If at the end, returns CreateDialog
pub fn get_next_target(
    nav_list: &[Uuid],
    active_session: Option<Uuid>,
) -> NavigationTarget {
    if nav_list.is_empty() {
        return NavigationTarget::CreateDialog;
    }

    match active_session {
        None => {
            // No active session (BoardView) - return first session
            NavigationTarget::Session(nav_list[0])
        }
        Some(active_id) => {
            // Find current position in the navigation list
            let current_idx = nav_list.iter().position(|&id| id == active_id);

            match current_idx {
                Some(idx) if idx + 1 < nav_list.len() => {
                    // There's a next item in the list
                    NavigationTarget::Session(nav_list[idx + 1])
                }
                Some(_) => {
                    // At the last item - show create dialog
                    NavigationTarget::CreateDialog
                }
                None => {
                    // Active session not found in list - shouldn't happen
                    // but be safe and show create dialog
                    NavigationTarget::CreateDialog
                }
            }
        }
    }
}

/// Get the previous navigation target from the current position.
///
/// - If no active session (BoardView), returns None (stay on board)
/// - If at first session, returns Board
/// - If at a watcher, returns prev sibling or parent session
/// - Otherwise returns previous item in list
pub fn get_prev_target(
    nav_list: &[Uuid],
    active_session: Option<Uuid>,
) -> NavigationTarget {
    match active_session {
        None => {
            // No active session (BoardView) - no previous
            NavigationTarget::Board
        }
        Some(active_id) => {
            if nav_list.is_empty() {
                return NavigationTarget::Board;
            }

            // Find current position in the navigation list
            let current_idx = nav_list.iter().position(|&id| id == active_id);

            match current_idx {
                Some(0) => {
                    // At the first item - go to board
                    NavigationTarget::Board
                }
                Some(idx) => {
                    // There's a previous item
                    NavigationTarget::Session(nav_list[idx - 1])
                }
                None => {
                    // Active session not found - go to board
                    NavigationTarget::Board
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test: Empty session list returns CreateDialog
    #[test]
    fn test_get_next_empty_returns_create_dialog() {
        let nav_list: Vec<Uuid> = vec![];
        let result = get_next_target(&nav_list, None);
        assert_eq!(result, NavigationTarget::CreateDialog);
    }

    /// Test: From board with sessions returns first session
    #[test]
    fn test_get_next_from_board_returns_first() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let nav_list = vec![a, b];

        let result = get_next_target(&nav_list, None);
        assert_eq!(result, NavigationTarget::Session(a));
    }

    /// Test: From first session returns second
    #[test]
    fn test_get_next_from_first_returns_second() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let nav_list = vec![a, b];

        let result = get_next_target(&nav_list, Some(a));
        assert_eq!(result, NavigationTarget::Session(b));
    }

    /// Test: From last session returns CreateDialog
    #[test]
    fn test_get_next_from_last_returns_create_dialog() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let nav_list = vec![a, b];

        let result = get_next_target(&nav_list, Some(b));
        assert_eq!(result, NavigationTarget::CreateDialog);
    }

    /// Test: Navigation list with watchers
    #[test]
    fn test_get_next_through_watchers() {
        let session_a = Uuid::new_v4();
        let watcher_w1 = Uuid::new_v4();
        let watcher_w2 = Uuid::new_v4();
        let session_b = Uuid::new_v4();

        // Navigation list: A → W1 → W2 → B
        let nav_list = vec![session_a, watcher_w1, watcher_w2, session_b];

        // From A, go to W1
        assert_eq!(
            get_next_target(&nav_list, Some(session_a)),
            NavigationTarget::Session(watcher_w1)
        );

        // From W1, go to W2
        assert_eq!(
            get_next_target(&nav_list, Some(watcher_w1)),
            NavigationTarget::Session(watcher_w2)
        );

        // From W2, go to B
        assert_eq!(
            get_next_target(&nav_list, Some(watcher_w2)),
            NavigationTarget::Session(session_b)
        );

        // From B, show create dialog
        assert_eq!(
            get_next_target(&nav_list, Some(session_b)),
            NavigationTarget::CreateDialog
        );
    }

    /// Test: Get prev from board returns Board
    #[test]
    fn test_get_prev_from_board_returns_board() {
        let a = Uuid::new_v4();
        let nav_list = vec![a];

        let result = get_prev_target(&nav_list, None);
        assert_eq!(result, NavigationTarget::Board);
    }

    /// Test: Get prev from first session returns Board
    #[test]
    fn test_get_prev_from_first_returns_board() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let nav_list = vec![a, b];

        let result = get_prev_target(&nav_list, Some(a));
        assert_eq!(result, NavigationTarget::Board);
    }

    /// Test: Get prev from second session returns first
    #[test]
    fn test_get_prev_from_second_returns_first() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let nav_list = vec![a, b];

        let result = get_prev_target(&nav_list, Some(b));
        assert_eq!(result, NavigationTarget::Session(a));
    }

    /// Test: Navigation backwards through watchers
    #[test]
    fn test_get_prev_through_watchers() {
        let session_a = Uuid::new_v4();
        let watcher_w1 = Uuid::new_v4();
        let watcher_w2 = Uuid::new_v4();
        let session_b = Uuid::new_v4();

        // Navigation list: A → W1 → W2 → B
        let nav_list = vec![session_a, watcher_w1, watcher_w2, session_b];

        // From B, go to W2
        assert_eq!(
            get_prev_target(&nav_list, Some(session_b)),
            NavigationTarget::Session(watcher_w2)
        );

        // From W2, go to W1
        assert_eq!(
            get_prev_target(&nav_list, Some(watcher_w2)),
            NavigationTarget::Session(watcher_w1)
        );

        // From W1, go to A
        assert_eq!(
            get_prev_target(&nav_list, Some(watcher_w1)),
            NavigationTarget::Session(session_a)
        );

        // From A, go to Board
        assert_eq!(
            get_prev_target(&nav_list, Some(session_a)),
            NavigationTarget::Board
        );
    }
}
