use crate::work_units::WorkUnitsWatcher;
use codelet_rpc_types::WorkUnitInfo;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast::{self, error::RecvError};

pub struct WorkUnitSubscription {
    active: Arc<Mutex<bool>>,
    cancelled: Arc<tokio::sync::Notify>,
}

impl WorkUnitSubscription {
    pub fn new(watcher: &WorkUnitsWatcher, observe: impl Fn(&Path) + Send + 'static) -> Self {
        Self::from_receiver(watcher.workspace().to_owned(), watcher.subscribe(), observe)
    }

    pub fn from_receiver(
        project: PathBuf,
        mut receiver: broadcast::Receiver<Vec<WorkUnitInfo>>,
        observe: impl Fn(&Path) + Send + 'static,
    ) -> Self {
        let active = Arc::new(Mutex::new(true));
        let cancelled = Arc::new(tokio::sync::Notify::new());
        let stopped = cancelled.clone();
        observe(&project);
        let live = active.clone();
        let result = std::thread::Builder::new()
            .name("work-unit-focus".into())
            .spawn(move || {
                futures::executor::block_on(async move {
                    loop {
                        let received = tokio::select! {
                            biased;
                            _ = stopped.notified() => break,
                            received = receiver.recv() => received,
                        };
                        match received {
                            Ok(_) | Err(RecvError::Lagged(_)) => {
                                let guard = live
                                    .lock()
                                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                                if !*guard {
                                    break;
                                }
                                observe(&project);
                            }
                            Err(RecvError::Closed) => break,
                        }
                    }
                })
            });
        if let Err(error) = result {
            tracing::warn!(%error, "could not start work-unit reconciliation subscription");
        }
        Self { active, cancelled }
    }
}

impl Drop for WorkUnitSubscription {
    fn drop(&mut self) {
        *self
            .active
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = false;
        self.cancelled.notify_one();
    }
}
