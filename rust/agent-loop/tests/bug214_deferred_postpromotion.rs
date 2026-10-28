//! Feature: spec/features/deferred-compaction-postpromotion-cancellation.feature

#[path = "support/automatic_compaction.rs"]
pub mod automatic;

use codelet_core::persistence::{get_session_messages_full, load_session};
use codelet_rpc_types::{SessionState, SessionStatus, StreamChunk};
use codelet_sessions::background_session::BackgroundSession;
use futures::Stream;
use std::pin::Pin;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex, Weak,
};
use std::task::{Context, Poll, Wake, Waker};
use wiremock::{matchers::method, Mock, ResponseTemplate};

type OutputStream = Pin<Box<dyn Stream<Item = StreamChunk> + Send>>;

struct PublicationInterrupt {
    session: Weak<BackgroundSession>,
    stream: Mutex<Option<OutputStream>>,
    fired: AtomicBool,
}

impl PublicationInterrupt {
    fn arm(session: &Arc<BackgroundSession>) -> Arc<Self> {
        let receiver = session.subscribe_to_stream();
        let stream = futures::stream::unfold(receiver, |mut receiver| async move {
            receiver.recv().await.ok().map(|chunk| (chunk, receiver))
        });
        let observer = Arc::new(Self {
            session: Arc::downgrade(session),
            stream: Mutex::new(Some(Box::pin(stream))),
            fired: AtomicBool::new(false),
        });
        observer.poll();
        observer
    }

    fn poll(self: &Arc<Self>) {
        let waker = Waker::from(self.clone());
        let mut context = Context::from_waker(&waker);
        let Ok(mut stream) = self.stream.try_lock() else {
            return;
        };
        while let Some(output) = stream.as_mut() {
            match output.as_mut().poll_next(&mut context) {
                Poll::Ready(Some(StreamChunk::CommittedCompaction { .. })) => {
                    self.fired.store(true, Ordering::Release);
                    if let Some(session) = self.session.upgrade() {
                        session.interrupt();
                    }
                    *stream = None;
                }
                Poll::Ready(Some(_)) => {}
                Poll::Ready(None) => *stream = None,
                Poll::Pending => return,
            }
        }
    }

    fn disarm(&self) {
        if let Ok(mut stream) = self.stream.lock() {
            *stream = None;
        }
    }
}

impl Wake for PublicationInterrupt {
    fn wake(self: Arc<Self>) {
        self.poll();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.poll();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_cancelling_publication_of_a_deferred_summary_keeps_the_session_usable(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given an agent task submits a valid summary through its deferred tool handoff
    let mut fixture = automatic::Harness::new(false, false, false).await;
    let original = load_session(fixture.session.id)?;
    let originals = get_session_messages_full(&original)?;
    fixture.server.reset().await;
    let requests = Arc::new(AtomicUsize::new(0));
    let request_count = requests.clone();
    Mock::given(method("POST"))
        .respond_with(move |_: &wiremock::Request| {
            let request = request_count.fetch_add(1, Ordering::SeqCst);
            let (delta, finish_reason) = if request == 0 {
                let arguments = serde_json::json!({
                    "content": "<dag-node depth=\"D1\" turns=\"0-2\" label=\"Original work\">The original question and completed evidence remain available.</dag-node>\n<dag-files>\n</dag-files>"
                });
                (
                    serde_json::json!({"role":"assistant","tool_calls":[{
                        "index":0,"id":"deferred-summary","type":"function",
                        "function":{"name":"inject_summary","arguments":arguments.to_string()}
                    }]}),
                    "tool_calls",
                )
            } else {
                (
                    serde_json::json!({"role":"assistant","content":"Unexpected continuation"}),
                    "stop",
                )
            };
            let chunk = serde_json::json!({
                "id":"deferred","object":"chat.completion.chunk","created":1,"model":"o3",
                "choices":[{"index":0,"delta":delta,"finish_reason":finish_reason}],
                "usage":{"prompt_tokens":100,"completion_tokens":10,"total_tokens":110}
            });
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(format!("data: {chunk}\n\ndata: [DONE]\n\n"))
        })
        .mount(&fixture.server)
        .await;
    // @step When the user interrupts during publication of the promoted summary
    let interrupt = PublicationInterrupt::arm(&fixture.session);
    fixture.send();
    let chunks = fixture.until_done().await;
    interrupt.disarm();
    assert!(interrupt.fired.load(Ordering::Acquire));
    assert!(fixture.session.is_interrupted.load(Ordering::Acquire));
    let inner = fixture.session.inner.lock().await;
    // @step Then the promoted generation remains authoritative and reconciled
    let manifest = load_session(fixture.session.id)?;
    let boundary = manifest
        .compaction_commit
        .as_ref()
        .ok_or("promoted generation")?;
    assert_eq!(boundary.identity.generation, 1);
    assert!(!boundary.publication_pending);
    codelet_core::persistence::compaction_boundary::validate_committed_boundary(&manifest)?;
    assert_eq!(inner.token_tracker.input_tokens, boundary.input_tokens);
    // @step And no compaction success or automatic continuation is emitted
    assert_eq!(requests.load(Ordering::SeqCst), 1);
    assert!(!chunks
        .iter()
        .any(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. })));
    let publication = chunks
        .iter()
        .position(|chunk| matches!(chunk, StreamChunk::CommittedCompaction { .. }))
        .ok_or("published deferred generation")?;
    assert!(!chunks[publication..].iter().any(|chunk| matches!(
        chunk,
        StreamChunk::SessionStateChange {
            state: SessionState::Running
        }
    )));
    // @step And compaction activity is cleared and the original conversation remains accessible
    assert_eq!(fixture.session.get_status(), SessionStatus::Idle);
    assert!(!fixture
        .session
        .compaction_in_progress
        .load(Ordering::Acquire));
    assert!(fixture.session.get_compaction_progress().is_none());
    let retained = get_session_messages_full(&manifest)?;
    assert_eq!(
        serde_json::to_value(&retained[..originals.len()])?,
        serde_json::to_value(&originals)?
    );
    // @step And the agent-loop caller leaves inference ready for a fresh user task
    let readiness = inner.ensure_compaction_inference_ready();
    assert!(
        readiness.is_ok(),
        "cancelled deferred publication must not re-block a reconciled generation: {readiness:?}"
    );
    Ok(())
}
