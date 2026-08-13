//! ChainOfCommand — supervisor/subordinate relationships between sessions.
//!
//! Lifted to `codelet-sessions` by **RPC-040** from
//! `rust/napi/src/session_manager.rs` (former lines 354-512). The
//! napi side now re-exports this type via
//! `pub use codelet_sessions::chain_of_command::ChainOfCommand;` so all
//! pre-existing call sites and unit tests in `codelet-napi` continue
//! to compile unchanged.

#![allow(clippy::expect_used)]

use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

/// ChainOfCommand enables supervisor sessions to observe subordinate sessions.
/// FIX-7: One supervisor can now spawn multiple subordinates (1:N from supervisor side)
/// - One subordinate can have multiple supervisors (1:N from subordinate side)
/// - Circular supervision is prevented via BFS cycle detection
pub struct ChainOfCommand {
    /// Subordinate session ID → list of supervisor session IDs
    subordinate_to_supervisors: RwLock<HashMap<Uuid, Vec<Uuid>>>,
    /// Supervisor session ID → list of subordinate session IDs (FIX-7: changed from Uuid to Vec<Uuid>)
    supervisor_to_subordinates: RwLock<HashMap<Uuid, Vec<Uuid>>>,
}

impl Default for ChainOfCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl ChainOfCommand {
    /// Create a new empty ChainOfCommand
    pub fn new() -> Self {
        Self {
            subordinate_to_supervisors: RwLock::new(HashMap::new()),
            supervisor_to_subordinates: RwLock::new(HashMap::new()),
        }
    }

    /// Register a supervisor for a subordinate session
    pub fn add_supervisor(
        &self,
        subordinate_id: Uuid,
        supervisor_id: Uuid,
    ) -> std::result::Result<(), String> {
        let mut sup2subs = self
            .supervisor_to_subordinates
            .write()
            .expect("supervisor_to_subordinates lock poisoned");

        if let Some(existing) = sup2subs.get(&supervisor_id) {
            if existing.contains(&subordinate_id) {
                return Err("subordinate already registered under this supervisor".to_string());
            }
        }

        {
            let mut visited = std::collections::HashSet::new();
            let mut queue = std::collections::VecDeque::new();
            queue.push_back(subordinate_id);
            visited.insert(subordinate_id);

            while let Some(current) = queue.pop_front() {
                if let Some(subordinates) = sup2subs.get(&current) {
                    for &sub in subordinates {
                        if sub == supervisor_id {
                            return Err("circular supervision not allowed".to_string());
                        }
                        if visited.insert(sub) {
                            queue.push_back(sub);
                        }
                    }
                }
            }
        }

        sup2subs
            .entry(supervisor_id)
            .or_default()
            .push(subordinate_id);

        let mut sub2sup = self
            .subordinate_to_supervisors
            .write()
            .expect("subordinate_to_supervisors lock poisoned");
        sub2sup
            .entry(subordinate_id)
            .or_default()
            .push(supervisor_id);

        Ok(())
    }

    /// Remove a supervisor relationship
    pub fn remove_supervisor(&self, supervisor_id: Uuid) {
        let subordinate_ids = {
            let mut sup2subs = self
                .supervisor_to_subordinates
                .write()
                .expect("supervisor_to_subordinates lock poisoned");
            sup2subs.remove(&supervisor_id).unwrap_or_default()
        };

        if !subordinate_ids.is_empty() {
            let mut sub2sup = self
                .subordinate_to_supervisors
                .write()
                .expect("subordinate_to_supervisors lock poisoned");
            for subordinate_id in subordinate_ids {
                if let Some(supervisors) = sub2sup.get_mut(&subordinate_id) {
                    supervisors.retain(|&id| id != supervisor_id);
                    if supervisors.is_empty() {
                        sub2sup.remove(&subordinate_id);
                    }
                }
            }
        }
    }

    /// Get all supervisors for a subordinate session
    pub fn get_supervisors(&self, subordinate_id: Uuid) -> Vec<Uuid> {
        let sub2sup = self
            .subordinate_to_supervisors
            .read()
            .expect("subordinate_to_supervisors lock poisoned");
        sub2sup.get(&subordinate_id).cloned().unwrap_or_default()
    }

    /// Get the first subordinate for a supervisor session (backward compat)
    pub fn get_subordinate(&self, supervisor_id: Uuid) -> Option<Uuid> {
        let sup2subs = self
            .supervisor_to_subordinates
            .read()
            .expect("supervisor_to_subordinates lock poisoned");
        sup2subs
            .get(&supervisor_id)
            .and_then(|v| v.first().copied())
    }

    /// Get all subordinates for a supervisor session (FIX-7)
    pub fn get_subordinates(&self, supervisor_id: Uuid) -> Vec<Uuid> {
        let sup2subs = self
            .supervisor_to_subordinates
            .read()
            .expect("supervisor_to_subordinates lock poisoned");
        sup2subs.get(&supervisor_id).cloned().unwrap_or_default()
    }

    /// Clean up all supervisor relationships when a subordinate session is removed
    pub fn cleanup_subordinate(&self, subordinate_id: Uuid) {
        let supervisors = {
            let mut sub2sup = self
                .subordinate_to_supervisors
                .write()
                .expect("subordinate_to_supervisors lock poisoned");
            sub2sup.remove(&subordinate_id).unwrap_or_default()
        };

        {
            let mut sup2subs = self
                .supervisor_to_subordinates
                .write()
                .expect("supervisor_to_subordinates lock poisoned");
            for supervisor_id in supervisors {
                if let Some(subordinates) = sup2subs.get_mut(&supervisor_id) {
                    subordinates.retain(|&id| id != subordinate_id);
                    if subordinates.is_empty() {
                        sup2subs.remove(&supervisor_id);
                    }
                }
            }
        }
    }

    /// Check if the ChainOfCommand has no entries
    pub fn is_empty(&self) -> bool {
        let sub2sup = self
            .subordinate_to_supervisors
            .read()
            .expect("subordinate_to_supervisors lock poisoned");
        let sup2subs = self
            .supervisor_to_subordinates
            .read()
            .expect("supervisor_to_subordinates lock poisoned");
        sub2sup.is_empty() && sup2subs.is_empty()
    }
}
