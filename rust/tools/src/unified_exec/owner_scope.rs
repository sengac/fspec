use super::{global_store, ChildHandle};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError, Weak};
use uuid::Uuid;

pub(super) type SharedOwner = Arc<Mutex<OwnerState>>;

static OWNERS: Lazy<Mutex<HashMap<Uuid, Weak<Mutex<OwnerState>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Default)]
pub(super) struct OwnerState {
    stopped: bool,
    processes: HashMap<String, ChildHandle>,
}

pub struct ProcessOwnerScope {
    owner: Uuid,
    state: SharedOwner,
    runtime: tokio::runtime::Handle,
}

impl ProcessOwnerScope {
    pub fn new(owner: Uuid) -> Self {
        let state = Arc::new(Mutex::new(OwnerState::default()));
        OWNERS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(owner, Arc::downgrade(&state));
        Self {
            owner,
            state,
            runtime: tokio::runtime::Handle::current(),
        }
    }
}

impl Drop for ProcessOwnerScope {
    fn drop(&mut self) {
        OWNERS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&self.owner);
        let processes = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            state.stopped = true;
            std::mem::take(&mut state.processes)
        };
        for child in processes.values() {
            child.kill();
        }
        let owner = self.state.clone();
        self.runtime.spawn(async move {
            for session in processes.keys() {
                global_store().close_owned_session(session, &owner).await;
            }
        });
    }
}

pub(super) fn capture_owner(owner: Uuid) -> Option<SharedOwner> {
    OWNERS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&owner)
        .and_then(Weak::upgrade)
}

pub(super) struct ProcessRegistration {
    state: SharedOwner,
    session: String,
}

impl ProcessRegistration {
    pub(super) fn new(state: SharedOwner, session: String, child: ChildHandle) -> Self {
        {
            let mut owner = state.lock().unwrap_or_else(PoisonError::into_inner);
            if owner.stopped {
                child.kill();
            } else {
                owner.processes.insert(session.clone(), child);
            }
        }
        Self { state, session }
    }

    pub(super) fn stopped(&self) -> bool {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .stopped
    }

    pub(super) fn belongs_to(&self, owner: &SharedOwner) -> bool {
        Arc::ptr_eq(&self.state, owner)
    }
}

impl Drop for ProcessRegistration {
    fn drop(&mut self) {
        let child = self
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .processes
            .remove(&self.session);
        if let Some(child) = child {
            child.kill();
        }
    }
}
