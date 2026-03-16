/**
 * VIEWNV-001: Session Navigation Module
 *
 * Provides hierarchy-aware navigation through sessions and supervisors.
 *
 * Navigation order:
 * Board → Session1 → S1.Supervisor1 → S1.Supervisor2 → Session2 → S2.Supervisor1 → ... → Create Dialog
 *
 * Rules:
 * - Shift+Right from session with supervisors → first supervisor
 * - Shift+Right from session without supervisors → next session
 * - Shift+Right from supervisor → next sibling, or next session if last sibling
 * - Shift+Left from first supervisor → subordinate session
 * - Shift+Left from supervisor → prev sibling, or subordinate if first sibling
 * - Shift+Left from first session → board
 */
use indexmap::IndexMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::session_manager::{BackgroundSession, ChainOfCommand};

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

/// Build a flattened navigation list from sessions and supervisors.
///
/// The list is ordered: Session1 → S1.Supervisors → Session2 → S2.Supervisors → ...
/// Each session is followed by its supervisors (in creation order).
pub fn build_navigation_list(
    sessions: &IndexMap<Uuid, Arc<BackgroundSession>>,
    chain_of_command: &ChainOfCommand,
) -> Vec<Uuid> {
    let mut result = Vec::new();

    // Iterate through sessions in insertion order
    for session_id in sessions.keys() {
        // Check if this session is a supervisor (has a subordinate)
        let subordinate = chain_of_command.get_subordinate(*session_id);

        if subordinate.is_some() {
            continue;
        }

        // Add the subordinate session
        result.push(*session_id);

        // Add all supervisors for this session
        let supervisors = chain_of_command.get_supervisors(*session_id);
        for supervisor_id in supervisors {
            // Only add if the supervisor exists in sessions
            if sessions.contains_key(&supervisor_id) {
                result.push(supervisor_id);
            }
        }
    }

    result
}

/// Get the next navigation target from the current position.
///
/// - If no active session (BoardView), returns first session
/// - If at a session, returns first supervisor or next session
/// - If at a supervisor, returns next sibling or next session
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
/// - If at a supervisor, returns prev sibling or subordinate session
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

    /// Test: Navigation list with supervisors
    #[test]
    fn test_get_next_through_supervisors() {
        let session_a = Uuid::new_v4();
        let supervisor_w1 = Uuid::new_v4();
        let supervisor_w2 = Uuid::new_v4();
        let session_b = Uuid::new_v4();

        // Navigation list: A → W1 → W2 → B
        let nav_list = vec![session_a, supervisor_w1, supervisor_w2, session_b];

        // From A, go to W1
        assert_eq!(
            get_next_target(&nav_list, Some(session_a)),
            NavigationTarget::Session(supervisor_w1)
        );

        // From W1, go to W2
        assert_eq!(
            get_next_target(&nav_list, Some(supervisor_w1)),
            NavigationTarget::Session(supervisor_w2)
        );

        // From W2, go to B
        assert_eq!(
            get_next_target(&nav_list, Some(supervisor_w2)),
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

    /// Test: Navigation backwards through supervisors
    #[test]
    fn test_get_prev_through_supervisors() {
        let session_a = Uuid::new_v4();
        let supervisor_w1 = Uuid::new_v4();
        let supervisor_w2 = Uuid::new_v4();
        let session_b = Uuid::new_v4();

        // Navigation list: A → W1 → W2 → B
        let nav_list = vec![session_a, supervisor_w1, supervisor_w2, session_b];

        // From B, go to W2
        assert_eq!(
            get_prev_target(&nav_list, Some(session_b)),
            NavigationTarget::Session(supervisor_w2)
        );

        // From W2, go to W1
        assert_eq!(
            get_prev_target(&nav_list, Some(supervisor_w2)),
            NavigationTarget::Session(supervisor_w1)
        );

        // From W1, go to A
        assert_eq!(
            get_prev_target(&nav_list, Some(supervisor_w1)),
            NavigationTarget::Session(session_a)
        );

        // From A, go to Board
        assert_eq!(
            get_prev_target(&nav_list, Some(session_a)),
            NavigationTarget::Board
        );
    }
}
