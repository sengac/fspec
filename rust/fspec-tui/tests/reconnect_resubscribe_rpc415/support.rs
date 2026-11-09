//! Shared helpers for `reconnect_resubscribe_rpc415.rs` (REFAC-025 split).

use codelet_fspec_tui::{Action, App};
use codelet_rpc_types::{SessionId, StreamChunk, WorkUnitInfo};

use super::common::MockBackend;

pub fn wu(id: &str, status: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: id.to_string(),
        work_type: "story".to_string(),
        status: status.to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

pub fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

/// Drain the App's action bus until a matching action arrives or the
/// deadline elapses. Drains every currently-queued action each tick so a
/// backlog does not starve the predicate before the deadline.
///
/// 2s deadline (raised from 500ms): the respawned subscriber tasks must be
/// scheduled AND polled at least once after `Action::Reconnected` before
/// they observe the post-reconnect events. Under CI/host load a fresh
/// tokio task can be starved well past 500ms, which turns a correct
/// respawn into a false-negative "event must reach the App" failure.
pub async fn wait_for_action<F: Fn(&Action) -> bool>(app: &mut App, pred: F) -> Option<Action> {
    // TEST-042: wait for the action itself under the deadlock guard (None
    // only when the guard expires), never for a fixed window.
    codelet_test_helpers::deadlock_guard("the awaited action", async {
        loop {
            while let Some(action) = app.try_recv_action() {
                if pred(&action) {
                    return action;
                }
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .ok()
}

/// The post-reconnect events observed on the App's bus, one per stream.
pub struct PostReconnect {
    pub work_units: Option<Action>,
    pub chunk: Option<Action>,
    pub status: Option<Action>,
    pub created: Option<Action>,
}

/// Push one event on each of the work_units / chunks / status / created
/// streams (re-emitting whatever has not been observed yet, 5s cap) and
/// collect the matching actions.
pub async fn collect_post_reconnect_events(app: &mut App, mock: &MockBackend) -> PostReconnect {
    // Determinism harness: each RESPAWNED subscriber only binds to the
    // new broadcast channel on its FIRST poll, and a broadcast send to
    // a channel that currently has ZERO live receivers is silently
    // dropped. So a push right after `Reconnected` can, under host
    // load, land on a stream whose respawned subscriber has not yet
    // polled — the EVENT is lost, not the subscriber (the observed
    // flake: the failure hops between the chunk / status /
    // session_created assertions depending on which stream's task got
    // scheduled first). The loop below re-emits every not-yet-observed
    // event each round (bounded at 5s total): a genuinely broken
    // respawn (subscriber bound to the OLD client's receivers, or a
    // stream that was never respawned) would never observe the
    // re-emitted events either, so the scenario's discriminating power
    // is unchanged — only the subscribe-vs-push race is removed.
    // TEST-042: re-push until each stream has delivered, under the deadlock
    // guard (no deadline; a missing stream fails loudly at the guard).
    let mut wu_action = None;
    let mut chunk_action = None;
    let mut status_action = None;
    let mut created_action = None;
    let _ = codelet_test_helpers::deadlock_guard("an event on every respawned stream", async {
    while wu_action.is_none()
        || chunk_action.is_none()
        || status_action.is_none()
        || created_action.is_none()
    {
        if wu_action.is_none() {
            mock.push_work_units(vec![wu("AUTH-LIVE", "backlog")]);
        }
        if chunk_action.is_none() {
            mock.push_chunk(sid("s-1"), StreamChunk::text("live".to_string()));
        }
        if status_action.is_none() {
            mock.push_status_change(sid("s-1"), codelet_rpc_types::SessionStatus::Running);
        }
        if created_action.is_none() {
            mock.push_session_created(sid("s-live"));
        }
        // Drain the bus for a few scheduler rounds, assigning each
        // post-reconnect action to its slot (first match wins; duplicates of
        // the re-emitted events are ignored), then re-push what is missing.
        for _ in 0..32 {
            while let Some(action) = app.try_recv_action() {
                if wu_action.is_none()
                    && matches!(
                        &action,
                        Action::WorkUnitsLoaded(units)
                            if units.iter().any(|u| u.id == "AUTH-LIVE")
                    )
                {
                    wu_action = Some(action);
                } else if chunk_action.is_none()
                    && matches!(
                        &action,
                        // Text chunks ride the live-transcript route
                        // (`transcript_push_action`); others stay ChunkReceived.
                        Action::ChunkReceived(id, _)
                            | Action::Transcript(
                                codelet_fspec_tui::app::transcript_sync::TranscriptAction::Live(id, _)
                            ) if id == &sid("s-1")
                    )
                {
                    chunk_action = Some(action);
                } else if status_action.is_none()
                    && matches!(&action, Action::SessionStatusChanged(id, _) if id == &sid("s-1"))
                {
                    status_action = Some(action);
                } else if created_action.is_none()
                    && matches!(&action, Action::SessionObserved(info) if info.id == "s-live")
                {
                    created_action = Some(action);
                }
            }
            if wu_action.is_some()
                && chunk_action.is_some()
                && status_action.is_some()
                && created_action.is_some()
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    }
    })
    .await;
    PostReconnect {
        work_units: wu_action,
        chunk: chunk_action,
        status: status_action,
        created: created_action,
    }
}
