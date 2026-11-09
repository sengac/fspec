//! Feature: spec/features/watchgraph-and-session-relationship-tracking.feature
//!
//! COV-057: the WatchGraph of WATCH-002 is now
//! `codelet_sessions::ChainOfCommand`. Vocabulary mapping used below:
//! a *parent* session is the observed **subordinate** and a *watcher* is its
//! **supervisor**, so `add_watcher(parent, watcher)` is
//! `add_supervisor(parent, watcher)`, `get_watchers(parent)` is
//! `get_supervisors(parent)`, `get_parent(watcher)` is
//! `get_subordinate(watcher)`, `remove_watcher(watcher)` is
//! `remove_supervisor(watcher)` and removing a parent session runs
//! `cleanup_subordinate(parent)` (as `SessionManager::destroy_session` does).

use std::collections::HashMap;

use codelet_sessions::ChainOfCommand;
use uuid::Uuid;

/// Named session ids for one scenario ("abc", "xyz", ...).
#[derive(Default)]
struct Sessions(HashMap<&'static str, Uuid>);

impl Sessions {
    fn id(&mut self, name: &'static str) -> Uuid {
        *self.0.entry(name).or_insert_with(Uuid::new_v4)
    }
}

#[test]
fn scenario_register_a_watcher_for_a_parent_session() -> Result<(), String> {
    // @step Given a WatchGraph with no relationships
    let graph = ChainOfCommand::new();
    assert!(graph.is_empty());
    let mut sessions = Sessions::default();
    // @step And a parent session "abc" exists
    let abc = sessions.id("abc");
    // @step And a watcher session "xyz" exists
    let xyz = sessions.id("xyz");
    // @step When I call add_watcher with parent_id "abc" and watcher_id "xyz"
    graph.add_supervisor(abc, xyz)?;
    // @step Then get_watchers for "abc" should return ["xyz"]
    assert_eq!(graph.get_supervisors(abc), vec![xyz]);
    // @step And get_parent for "xyz" should return "abc"
    assert_eq!(graph.get_subordinate(xyz), Some(abc));
    Ok(())
}

#[test]
fn scenario_parent_with_multiple_watchers() -> Result<(), String> {
    // @step Given a WatchGraph with no relationships
    let graph = ChainOfCommand::new();
    let mut sessions = Sessions::default();
    // @step And a parent session "abc" exists
    let abc = sessions.id("abc");
    // @step And watcher sessions "xyz" and "def" exist
    let (xyz, def) = (sessions.id("xyz"), sessions.id("def"));
    // @step When I call add_watcher with parent_id "abc" and watcher_id "xyz"
    graph.add_supervisor(abc, xyz)?;
    // @step And I call add_watcher with parent_id "abc" and watcher_id "def"
    graph.add_supervisor(abc, def)?;
    // @step Then get_watchers for "abc" should return ["xyz", "def"]
    assert_eq!(graph.get_supervisors(abc), vec![xyz, def]);
    Ok(())
}

#[test]
fn scenario_query_parent_for_a_watcher() -> Result<(), String> {
    // @step Given a WatchGraph with no relationships
    let graph = ChainOfCommand::new();
    let mut sessions = Sessions::default();
    // @step And session "xyz" is watching session "abc"
    let (xyz, abc) = (sessions.id("xyz"), sessions.id("abc"));
    graph.add_supervisor(abc, xyz)?;
    // @step When I call get_parent with watcher_id "xyz"
    let parent = graph.get_subordinate(xyz);
    // @step Then it should return "abc"
    assert_eq!(parent, Some(abc));
    Ok(())
}

#[test]
fn scenario_remove_a_watcher_relationship() -> Result<(), String> {
    // @step Given a WatchGraph with no relationships
    let graph = ChainOfCommand::new();
    let mut sessions = Sessions::default();
    // @step And session "xyz" is watching session "abc"
    let (xyz, abc) = (sessions.id("xyz"), sessions.id("abc"));
    graph.add_supervisor(abc, xyz)?;
    // @step When I call remove_watcher with watcher_id "xyz"
    graph.remove_supervisor(xyz);
    // @step Then get_watchers for "abc" should return an empty list
    assert!(graph.get_supervisors(abc).is_empty());
    // @step And get_parent for "xyz" should return None
    assert_eq!(graph.get_subordinate(xyz), None);
    assert!(graph.is_empty());
    Ok(())
}

#[test]
fn scenario_circular_watching_is_prevented() -> Result<(), String> {
    // @step Given a WatchGraph with no relationships
    let graph = ChainOfCommand::new();
    let mut sessions = Sessions::default();
    // @step And session "B" is watching session "A"
    let (b, a) = (sessions.id("B"), sessions.id("A"));
    graph.add_supervisor(a, b)?;
    // @step When I call add_watcher with parent_id "B" and watcher_id "A"
    let result = graph.add_supervisor(b, a);
    // @step Then it should return an error "circular supervision not allowed"
    assert_eq!(result, Err("circular supervision not allowed".to_string()));
    assert_eq!(graph.get_supervisors(b), Vec::<Uuid>::new());
    assert_eq!(graph.get_subordinate(a), None);
    Ok(())
}

#[test]
fn scenario_regular_session_has_no_parent() {
    // @step Given a WatchGraph with no relationships
    let graph = ChainOfCommand::new();
    let mut sessions = Sessions::default();
    // @step And a regular session "abc" exists that is not a watcher
    let abc = sessions.id("abc");
    // @step When I call get_parent with session_id "abc"
    let parent = graph.get_subordinate(abc);
    // @step Then it should return None
    assert_eq!(parent, None);
}

#[test]
fn scenario_cleanup_watchers_when_parent_session_is_removed() -> Result<(), String> {
    // @step Given a WatchGraph with no relationships
    let graph = ChainOfCommand::new();
    let mut sessions = Sessions::default();
    // @step And session "xyz" is watching session "abc"
    let (xyz, abc) = (sessions.id("xyz"), sessions.id("abc"));
    graph.add_supervisor(abc, xyz)?;
    // @step And session "def" is watching session "abc"
    let def = sessions.id("def");
    graph.add_supervisor(abc, def)?;
    // @step When parent session "abc" is removed
    graph.cleanup_subordinate(abc);
    // @step Then get_parent for "xyz" should return None
    assert_eq!(graph.get_subordinate(xyz), None);
    // @step And get_parent for "def" should return None
    assert_eq!(graph.get_subordinate(def), None);
    // @step And the WatchGraph should have no entries
    assert!(graph.is_empty());
    Ok(())
}
