use futures::{Stream, StreamExt};
use rig::completion::CompletionError;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RecoveryStatus {
    WaitingForNetwork { attempt: usize },
    RetryingStream { attempt: usize },
    Recovered,
    Failed,
    Cancelled,
}

impl RecoveryStatus {
    fn notification(self) -> String {
        match self {
            Self::WaitingForNetwork { attempt } => {
                format!("⟳ Reconnecting... waiting for network (attempt {attempt})")
            }
            Self::RetryingStream { attempt } => format!("⟳ Reconnecting... {attempt}/5"),
            Self::Recovered => "✓ Reconnected".into(),
            Self::Failed => "✗ Reconnection failed".into(),
            Self::Cancelled => "Reconnection cancelled".into(),
        }
    }
}

#[derive(Clone, Copy, Debug, thiserror::Error)]
pub enum NetworkFailure {
    #[error("Codex network connection failed")]
    Connection,
    #[error("Codex network setup deadline exceeded")]
    SetupDeadline,
    #[error("Codex network stream interrupted before terminal completion")]
    Interrupted,
    #[error("Codex network idle deadline exceeded")]
    IdleDeadline,
}

impl NetworkFailure {
    pub(super) fn into_http_error(self) -> rig::http_client::Error {
        rig::http_client::Error::Instance(Box::new(self))
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Codex reconnection failed after five stream retries: {source}")]
pub struct CodexNetworkFailure {
    #[source]
    pub source: NetworkFailure,
}

type Observer = Arc<dyn Fn(String) + Send + Sync>;

#[derive(Default)]
struct State {
    status: Option<RecoveryStatus>,
    observer: Option<Observer>,
}

#[derive(Clone, Default)]
pub struct CodexRecoveryHandle(Arc<Mutex<State>>);

impl std::fmt::Debug for CodexRecoveryHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_tuple("CodexRecoveryHandle")
            .field(&self.status())
            .finish()
    }
}

impl CodexRecoveryHandle {
    pub fn status(&self) -> Option<RecoveryStatus> {
        self.0.lock().ok().and_then(|state| state.status)
    }

    pub fn is_reconnecting(&self) -> bool {
        matches!(
            self.status(),
            Some(RecoveryStatus::WaitingForNetwork { .. } | RecoveryStatus::RetryingStream { .. })
        )
    }

    pub fn set_observer(&self, observer: Observer) {
        if let Ok(mut state) = self.0.lock() {
            state.observer = Some(observer);
        }
    }

    fn update(&self, status: RecoveryStatus) {
        let observer = self.0.lock().ok().and_then(|mut state| {
            state.status = Some(status);
            state.observer.clone()
        });
        if let Some(observer) = observer {
            observer(status.notification());
        }
    }

    pub(super) fn track<T: Clone + Send + 'static>(
        self,
        stream: impl Stream<Item = Result<rig::streaming::RawStreamingChoice<T>, CompletionError>>
            + Send
            + 'static,
    ) -> impl Stream<Item = Result<rig::streaming::RawStreamingChoice<T>, CompletionError>> + Send
    {
        async_stream::stream! {
            if let Ok(mut state) = self.0.lock() {
                state.status = None;
            }
            let mut guard = RecoveryGuard { handle: self, finished: false };
            futures::pin_mut!(stream);
            while let Some(event) = stream.next().await {
                match &event {
                    Ok(rig::streaming::RawStreamingChoice::ToolCall(_)
                        | rig::streaming::RawStreamingChoice::FinalResponse(_)) => guard.finish(true),
                    Err(_) => guard.finish(false),
                    _ => {}
                }
                yield event;
            }
            guard.finish(false);
        }
    }
}

struct RecoveryGuard {
    handle: CodexRecoveryHandle,
    finished: bool,
}

impl RecoveryGuard {
    fn finish(&mut self, success: bool) {
        if !self.finished {
            self.finished = true;
            if self.handle.is_reconnecting() {
                self.handle.update(if success {
                    RecoveryStatus::Recovered
                } else {
                    RecoveryStatus::Failed
                });
            }
        }
    }
}

impl Drop for RecoveryGuard {
    fn drop(&mut self) {
        if !self.finished && self.handle.is_reconnecting() {
            self.handle.update(RecoveryStatus::Cancelled);
        }
    }
}

pub(super) struct RecoveryPolicy {
    handle: CodexRecoveryHandle,
    streams: usize,
    connections: usize,
}

impl RecoveryPolicy {
    pub fn new(handle: CodexRecoveryHandle) -> Self {
        Self {
            handle,
            streams: 0,
            connections: 0,
        }
    }

    pub fn connected(&mut self) {
        self.connections = 0;
    }

    pub async fn retry(&mut self, error: CompletionError) -> Result<(), CompletionError> {
        let mut source: Option<&(dyn std::error::Error + 'static)> = Some(&error);
        let mut failure = None;
        while let Some(error) = source {
            if let Some(kind) = error.downcast_ref::<NetworkFailure>() {
                failure = Some(*kind);
                break;
            }
            source = error.source();
        }
        let Some(failure) = failure else {
            return Err(error);
        };
        let delay = if matches!(failure, NetworkFailure::Connection) {
            self.connections = self.connections.saturating_add(1);
            self.handle.update(RecoveryStatus::WaitingForNetwork {
                attempt: self.connections,
            });
            Duration::from_secs([5, 10, 20, 40, 60][self.connections.saturating_sub(1).min(4)])
        } else {
            self.connections = 0;
            if self.streams == 5 {
                return Err(CompletionError::HttpError(
                    rig::http_client::Error::Instance(Box::new(CodexNetworkFailure {
                        source: failure,
                    })),
                ));
            }
            self.streams += 1;
            self.handle.update(RecoveryStatus::RetryingStream {
                attempt: self.streams,
            });
            Duration::from_millis(250 * (1 << (self.streams - 1)))
        };
        tokio::time::sleep(delay).await;
        Ok(())
    }
}
