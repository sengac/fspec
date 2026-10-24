use super::{
    CodexMetadataStatus, CodexPresentationEvent, Observer, PresentationObserver, Scope, State,
};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

enum Callback {
    Diagnostic(Observer, CodexMetadataStatus),
    Presentation(PresentationObserver, CodexPresentationEvent),
}

struct Notification {
    scope: Scope,
    observer: Observer,
    status: CodexMetadataStatus,
}

#[derive(Default)]
pub(super) struct Queue {
    pending: VecDeque<Notification>,
    dispatching: bool,
    waiting: Option<bool>,
    verified: bool,
}

pub(super) fn replace_observer(state: &mut State, observer: PresentationObserver) {
    state.presentation_observer = Some(observer);
    state.delivery.waiting = None;
}

pub(super) fn enqueue(state: &mut State, previous: CodexMetadataStatus) -> bool {
    if previous != state.status {
        if let Some(observer) = &state.observer {
            state.delivery.pending.push_back(Notification {
                scope: state.scope,
                observer: observer.clone(),
                status: state.status.clone(),
            });
        }
    }
    if !state.status.verification_required {
        state.delivery.verified = false;
    }
    let presentation = state.presentation_observer.is_some()
        && (state.delivery.waiting != Some(state.waiting)
            || (state.status.verification_required && !state.delivery.verified));
    if state.delivery.dispatching || (state.delivery.pending.is_empty() && !presentation) {
        return false;
    }
    state.delivery.dispatching = true;
    true
}

pub(super) fn dispatch(shared: &Arc<Mutex<State>>) {
    loop {
        let callback = shared.lock().ok().and_then(|mut state| {
            while let Some(notification) = state.delivery.pending.pop_front() {
                if notification.scope == state.scope {
                    return Some(Callback::Diagnostic(
                        notification.observer,
                        notification.status,
                    ));
                }
            }
            if let Some(observer) = state.presentation_observer.clone() {
                if state.delivery.waiting != Some(state.waiting) {
                    state.delivery.waiting = Some(state.waiting);
                    return Some(Callback::Presentation(
                        observer,
                        CodexPresentationEvent::Waiting(state.waiting),
                    ));
                }
                if state.status.verification_required && !state.delivery.verified {
                    state.delivery.verified = true;
                    return Some(Callback::Presentation(
                        observer,
                        CodexPresentationEvent::Verification,
                    ));
                }
            }
            state.delivery.dispatching = false;
            None
        });
        match callback {
            Some(Callback::Diagnostic(observer, status)) => observer(status),
            Some(Callback::Presentation(observer, event)) => observer(event),
            None => return,
        }
    }
}
