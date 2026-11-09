//! Feature: spec/features/subordinate-agent-shift-arrow-cycling.feature
//!
//! COV-057 / BUG-124 regression: drives the production
//! `codelet_sessions::navigation::build_navigation_list` over a real
//! `IndexMap<Uuid, Arc<BackgroundSession>>` in which the supervisor is
//! inserted FIRST and its subordinates after it (the real-world spawn order
//! that exposed the duplication bug), with the supervisor relationships
//! registered in a real `ChainOfCommand`. Shift+Right / Shift+Left are the
//! production `get_next_target` / `get_prev_target` that
//! `SessionManager::get_next_session` / `get_prev_session` delegate to;
//! `None` as the active session is the board.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "cov057_support/session.rs"]
pub mod support;

use std::sync::Arc;

use codelet_sessions::background_session::BackgroundSession;
use codelet_sessions::navigation::{
    build_navigation_list, get_next_target, get_prev_target, NavigationTarget,
};
use codelet_sessions::ChainOfCommand;
use indexmap::IndexMap;
use support::{data_directory, stub_session, TestResult};
use uuid::Uuid;

/// The supervisor and its subordinates, in spawn order.
struct Spawned {
    _directory: tempfile::TempDir,
    sessions: IndexMap<Uuid, Arc<BackgroundSession>>,
    chain: ChainOfCommand,
    supervisor: Uuid,
    subordinates: Vec<Uuid>,
}

/// Insert the supervisor first, then spawn `count` subordinates under it.
fn supervisor_with_subordinates(count: usize) -> TestResult<Spawned> {
    let directory = data_directory()?;
    let mut sessions = IndexMap::new();
    let chain = ChainOfCommand::new();
    let (supervisor, _input) = stub_session(directory.path(), "supervisor")?;
    let supervisor_id = supervisor.id;
    sessions.insert(supervisor_id, supervisor);
    let mut subordinates = Vec::new();
    for index in 1..=count {
        let (subordinate, _input) = stub_session(directory.path(), &format!("s{index}"))?;
        chain.add_supervisor(subordinate.id, supervisor_id)?;
        subordinates.push(subordinate.id);
        sessions.insert(subordinate.id, subordinate);
    }
    Ok(Spawned {
        _directory: directory,
        sessions,
        chain,
        supervisor: supervisor_id,
        subordinates,
    })
}

impl Spawned {
    fn nav_list(&self) -> Vec<Uuid> {
        build_navigation_list(&self.sessions, &self.chain)
    }

    fn all(&self) -> Vec<Uuid> {
        std::iter::once(self.supervisor)
            .chain(self.subordinates.iter().copied())
            .collect()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_single_subordinate_appears_once_in_the_navigation_list() -> TestResult {
    // @step Given the session manager contains the supervisor inserted first
    // @step And the supervisor has spawned one subordinate "s1"
    let spawned = supervisor_with_subordinates(1)?;
    // @step When the navigation list is built
    let list = spawned.nav_list();
    // @step Then the navigation list contains the supervisor and "s1" exactly once each in insertion order
    assert_eq!(list, vec![spawned.supervisor, spawned.subordinates[0]]);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_five_subordinates_appear_once_each_in_spawn_order() -> TestResult {
    // @step Given the session manager contains the supervisor inserted first
    // @step And the supervisor has spawned subordinates "s1", "s2", "s3", "s4", "s5" in that order
    let spawned = supervisor_with_subordinates(5)?;
    // @step When the navigation list is built
    let list = spawned.nav_list();
    // @step Then the navigation list is exactly [supervisor, s1, s2, s3, s4, s5]
    assert_eq!(list, spawned.all());
    // @step And no UUID appears more than once
    let mut unique = list.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), list.len());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_shift_right_from_the_board_cycles_through_every_session_exactly_once(
) -> TestResult {
    // @step Given the session manager contains the supervisor inserted first
    // @step And the supervisor has spawned subordinates "s1", "s2", "s3", "s4", "s5" in that order
    let spawned = supervisor_with_subordinates(5)?;
    let list = spawned.nav_list();
    // @step And I am on the board
    let mut active: Option<Uuid> = None;
    // @step When I press Shift+Right repeatedly until I reach the create-session dialog
    let mut visited = Vec::new();
    let last = loop {
        match get_next_target(&list, active) {
            NavigationTarget::Session(id) if visited.len() <= list.len() => {
                visited.push(id);
                active = Some(id);
            }
            other => break other,
        }
    };
    // @step Then I visit each of [supervisor, s1, s2, s3, s4, s5] exactly once in that order
    assert_eq!(visited, spawned.all());
    // @step And the next press shows the create-session dialog
    assert_eq!(last, NavigationTarget::CreateDialog);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_shift_left_from_the_last_subordinate_cycles_back_to_the_board_exactly_once_per_session(
) -> TestResult {
    // @step Given the session manager contains the supervisor inserted first
    // @step And the supervisor has spawned subordinates "s1", "s2", "s3", "s4", "s5" in that order
    let spawned = supervisor_with_subordinates(5)?;
    let list = spawned.nav_list();
    // @step And I am viewing "s5"
    let mut active = Some(spawned.subordinates[4]);
    // @step When I press Shift+Left repeatedly until I reach the board
    let mut visited = Vec::new();
    let last = loop {
        match get_prev_target(&list, active) {
            NavigationTarget::Session(id) if visited.len() <= list.len() => {
                visited.push(id);
                active = Some(id);
            }
            other => break other,
        }
    };
    // @step Then I visit each of [s4, s3, s2, s1, supervisor] exactly once in that order
    let s = &spawned.subordinates;
    assert_eq!(visited, vec![s[3], s[2], s[1], s[0], spawned.supervisor]);
    // @step And the next press returns to the board
    assert_eq!(last, NavigationTarget::Board);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_two_subordinates_do_not_loop_on_the_first_subordinate() -> TestResult {
    // @step Given the session manager contains the supervisor inserted first
    // @step And the supervisor has spawned subordinates "s1" and "s2" in that order
    let spawned = supervisor_with_subordinates(2)?;
    let list = spawned.nav_list();
    // @step And I am viewing the supervisor
    let active = Some(spawned.supervisor);
    // @step When I press Shift+Right twice
    let first = get_next_target(&list, active);
    let NavigationTarget::Session(after_first) = first else {
        panic!("first Shift+Right left the sessions: {first:?}");
    };
    let second = get_next_target(&list, Some(after_first));
    // @step Then I am viewing "s1" after the first press
    assert_eq!(after_first, spawned.subordinates[0]);
    // @step And I am viewing "s2" after the second press
    assert_eq!(second, NavigationTarget::Session(spawned.subordinates[1]));
    Ok(())
}

#[test]
fn scenario_empty_session_manager_produces_an_empty_navigation_list() {
    // @step Given the session manager contains no sessions
    let sessions: IndexMap<Uuid, Arc<BackgroundSession>> = IndexMap::new();
    let chain = ChainOfCommand::new();
    // @step When the navigation list is built
    let list = build_navigation_list(&sessions, &chain);
    // @step Then the navigation list is empty
    assert!(list.is_empty(), "{list:?}");
    // @step And pressing Shift+Right from the board shows the create-session dialog
    assert_eq!(get_next_target(&list, None), NavigationTarget::CreateDialog);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_shift_right_from_the_last_session_shows_the_create_session_dialog() -> TestResult
{
    // @step Given the session manager contains the supervisor inserted first
    // @step And the supervisor has spawned subordinates "s1" and "s2" in that order
    let spawned = supervisor_with_subordinates(2)?;
    let list = spawned.nav_list();
    // @step And I am viewing "s2"
    let active = Some(spawned.subordinates[1]);
    assert_eq!(list.last().copied(), active, "s2 is the last session");
    // @step When I press Shift+Right
    let target = get_next_target(&list, active);
    // @step Then the create-session dialog appears
    assert_eq!(target, NavigationTarget::CreateDialog);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_shift_left_from_the_first_session_returns_to_the_board() -> TestResult {
    // @step Given the session manager contains the supervisor inserted first
    // @step And the supervisor has spawned subordinates "s1" and "s2" in that order
    let spawned = supervisor_with_subordinates(2)?;
    let list = spawned.nav_list();
    // @step And I am viewing the supervisor
    let active = Some(spawned.supervisor);
    assert_eq!(list.first().copied(), active, "the supervisor is first");
    // @step When I press Shift+Left
    let target = get_prev_target(&list, active);
    // @step Then I am on the board
    assert_eq!(target, NavigationTarget::Board);
    Ok(())
}
