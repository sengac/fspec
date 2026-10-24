use codelet_core::persistence::load_session;
use codelet_rpc_types::StreamChunk;
use codelet_sessions::background_session::BackgroundSession;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, Weak};
use std::task::{Context, Wake, Waker};

type Receive = Pin<Box<dyn Future<Output = ()> + Send>>;

pub struct CompletionObserver {
    session: Weak<BackgroundSession>,
    receives: Mutex<Vec<Receive>>,
    pub pending_at_completion: Mutex<Vec<bool>>,
}

impl CompletionObserver {
    pub fn new(session: &Arc<BackgroundSession>) -> Arc<Self> {
        let observer = Arc::new(Self {
            session: Arc::downgrade(session),
            receives: Mutex::new(Vec::new()),
            pending_at_completion: Mutex::new(Vec::new()),
        });
        observer.arm();
        observer
    }

    pub fn stop(&self) {
        self.receives.lock().expect("registered receivers").clear();
    }

    fn arm(self: &Arc<Self>) {
        let session = self.session.upgrade().expect("live observed session");
        let mut receiver = session.subscribe_to_stream();
        let mut receive: Receive = Box::pin(async move {
            let _ = receiver.recv().await;
        });
        let waker = Waker::from(self.clone());
        assert!(receive
            .as_mut()
            .poll(&mut Context::from_waker(&waker))
            .is_pending());
        self.receives
            .lock()
            .expect("registered receivers")
            .push(receive);
    }
}

impl Wake for CompletionObserver {
    fn wake(self: Arc<Self>) {
        let session = self.session.upgrade().expect("live observed session");
        if matches!(
            session.get_buffered_output(usize::MAX).last(),
            Some(StreamChunk::CompactionComplete { .. })
        ) {
            let pending = load_session(session.id)
                .expect("durable manifest during synchronous completion delivery")
                .compaction_commit
                .expect("committed generation during completion")
                .publication_pending;
            self.pending_at_completion
                .lock()
                .expect("completion observations")
                .push(pending);
        }
        self.arm();
    }
}
