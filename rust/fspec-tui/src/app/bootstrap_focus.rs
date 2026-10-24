use std::sync::Arc;

use tokio::sync::{broadcast, mpsc::UnboundedSender};

use crate::{components::Action, transport::FspecBackend};

use super::state::App;

async fn refresh(backend: &Arc<dyn FspecBackend>, sender: &UnboundedSender<Action>) {
    if let Ok(sessions) = backend.list_live_sessions().await {
        for session in sessions {
            if let Ok(focus) = backend.get_work_unit_focus(session.clone()).await {
                let _ = sender.send(Action::WorkUnitFocusChanged(session, focus));
            }
        }
    }
}

impl App {
    pub(crate) fn spawn_work_unit_focus_subscriber(&mut self) {
        let backend = self.backend.clone();
        let sender = self.action_tx.clone();
        let mut receiver = backend.work_unit_focus_rx();
        self.subscriber_tasks.push(tokio::spawn(async move {
            refresh(&backend, &sender).await;
            loop {
                match receiver.recv().await {
                    Ok((session, focus)) => {
                        let _ = sender.send(Action::WorkUnitFocusChanged(session, focus));
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => refresh(&backend, &sender).await,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }));
    }
}
