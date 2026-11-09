//! RIG-016: per-session inbox of input that arrived while a turn was running.
//!
//! Sessions write here and the stream loop and agent loop read from here, so
//! steering and queueing need no new crate dependency. Steers are delivered at
//! the running turn's next safe boundary; queued entries start as new turns
//! when a turn completes, and stay parked after an interrupt.

use std::collections::{HashMap, VecDeque};
use std::sync::{LazyLock, Mutex, PoisonError};
use uuid::Uuid;

/// One input waiting in a session's inbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxEntry {
    pub text: String,
    pub thinking_config: Option<String>,
}

#[derive(Default)]
struct Inbox {
    steers: VecDeque<InboxEntry>,
    queued: VecDeque<InboxEntry>,
}

static INBOXES: LazyLock<Mutex<HashMap<Uuid, Inbox>>> = LazyLock::new(Mutex::default);

fn with<T>(session: Uuid, apply: impl FnOnce(&mut Inbox) -> T) -> T {
    let mut inboxes = INBOXES.lock().unwrap_or_else(PoisonError::into_inner);
    apply(inboxes.entry(session).or_default())
}

/// Hold `entry` for delivery at the running turn's next safe boundary.
pub fn push_steer(session: Uuid, entry: InboxEntry) {
    with(session, |inbox| inbox.steers.push_back(entry));
}

/// Take every pending steer, oldest first.
pub fn take_steers(session: Uuid) -> Vec<InboxEntry> {
    with(session, |inbox| inbox.steers.drain(..).collect())
}

/// Whether a steer is waiting.
pub fn has_steers(session: Uuid) -> bool {
    with(session, |inbox| !inbox.steers.is_empty())
}

/// Queue `entry` to start after the running turn.
pub fn push_queued(session: Uuid, entry: InboxEntry) {
    with(session, |inbox| inbox.queued.push_back(entry));
}

/// Park `entry` ahead of every queued entry.
pub fn push_queued_front(session: Uuid, entry: InboxEntry) {
    with(session, |inbox| inbox.queued.push_front(entry));
}

/// Move undelivered steers to the front of the queue, keeping their order,
/// so an interrupted turn parks them instead of dropping them.
pub fn park_steers(session: Uuid) {
    with(session, |inbox| {
        while let Some(entry) = inbox.steers.pop_back() {
            inbox.queued.push_front(entry);
        }
    });
}

/// Take the next queued entry.
pub fn pop_queued(session: Uuid) -> Option<InboxEntry> {
    with(session, |inbox| inbox.queued.pop_front())
}

/// The queued texts, oldest first, without removing them.
pub fn queued_texts(session: Uuid) -> Vec<String> {
    with(session, |inbox| {
        inbox
            .queued
            .iter()
            .map(|entry| entry.text.clone())
            .collect()
    })
}

/// Forget a session's inbox (session removed or cleared).
pub fn clear(session: Uuid) {
    let mut inboxes = INBOXES.lock().unwrap_or_else(PoisonError::into_inner);
    inboxes.remove(&session);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(text: &str) -> InboxEntry {
        InboxEntry {
            text: text.into(),
            thinking_config: None,
        }
    }

    #[test]
    fn steers_park_ahead_of_queued_inputs_in_order() {
        let session = Uuid::new_v4();
        push_queued(session, entry("queued"));
        push_steer(session, entry("steer one"));
        push_steer(session, entry("steer two"));
        park_steers(session);
        assert!(!has_steers(session));
        assert_eq!(queued_texts(session), ["steer one", "steer two", "queued"]);
        assert_eq!(
            pop_queued(session).map(|e| e.text).as_deref(),
            Some("steer one")
        );
        clear(session);
        assert!(queued_texts(session).is_empty());
    }
}
