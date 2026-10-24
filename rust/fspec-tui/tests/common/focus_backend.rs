use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Mutex,
    },
};

use codelet_fspec_tui::transport::BackendError;
use codelet_rpc_types::{SessionId, WorkUnitContext, WorkUnitFocus};
use tokio::sync::broadcast;

pub(super) struct FocusState {
    values: Mutex<HashMap<SessionId, WorkUnitFocus>>,
    sender: Mutex<Option<broadcast::Sender<(SessionId, WorkUnitFocus)>>>,
    fetches: AtomicUsize,
    clear_after_bind: AtomicBool,
}

impl super::MockBackend {
    pub fn clear_focus_after_binding(&self) {
        self.focus_state
            .clear_after_bind
            .store(true, Ordering::SeqCst);
    }
    pub fn focus_fetches(&self) -> usize {
        self.focus_state.fetches.load(Ordering::SeqCst)
    }
}

impl Default for FocusState {
    fn default() -> Self {
        Self {
            values: Mutex::new(HashMap::new()),
            sender: Mutex::new(Some(broadcast::channel(1).0)),
            fetches: AtomicUsize::new(0),
            clear_after_bind: AtomicBool::new(false),
        }
    }
}

impl FocusState {
    pub(super) fn bind(
        &self,
        session: SessionId,
        context: Option<WorkUnitContext>,
    ) -> Result<(), BackendError> {
        self.set(session.clone(), context, "board".into())?;
        if self.clear_after_bind.swap(false, Ordering::SeqCst) {
            self.set(session, None, "other-client-clear".into())?;
        }
        Ok(())
    }
    pub(super) fn get(&self, session: &SessionId) -> Result<WorkUnitFocus, BackendError> {
        self.fetches.fetch_add(1, Ordering::SeqCst);
        self.values
            .lock()
            .map_err(|error| BackendError::WorkUnitFocus(error.to_string()))?
            .get(session)
            .cloned()
            .ok_or_else(|| BackendError::WorkUnitFocus("uninitialized fixture".into()))
    }

    pub(super) fn set(
        &self,
        session: SessionId,
        context: Option<WorkUnitContext>,
        source: String,
    ) -> Result<WorkUnitFocus, BackendError> {
        let mut values = self
            .values
            .lock()
            .map_err(|error| BackendError::WorkUnitFocus(error.to_string()))?;
        let revision = values.get(&session).map_or(1, |focus| focus.revision + 1);
        let focus = WorkUnitFocus {
            revision,
            context,
            source,
        };
        values.insert(session.clone(), focus.clone());
        if let Some(sender) = self
            .sender
            .lock()
            .map_err(|error| BackendError::WorkUnitFocus(error.to_string()))?
            .as_ref()
        {
            let _ = sender.send((session, focus.clone()));
        }
        Ok(focus)
    }

    pub(super) fn subscribe(&self) -> broadcast::Receiver<(SessionId, WorkUnitFocus)> {
        if let Ok(sender) = self.sender.lock() {
            if let Some(sender) = sender.as_ref() {
                return sender.subscribe();
            }
        }
        let (sender, receiver) = broadcast::channel(1);
        drop(sender);
        receiver
    }

    pub(super) fn disconnect(&self) {
        if let Ok(mut sender) = self.sender.lock() {
            *sender = None;
        }
    }

    pub(super) fn reconnect(&self) {
        if let Ok(mut sender) = self.sender.lock() {
            *sender = Some(broadcast::channel(1).0);
        }
    }
}
