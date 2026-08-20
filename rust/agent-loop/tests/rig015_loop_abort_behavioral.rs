//! Feature: spec/features/streaming-loop-abort-behavioral-test.feature
//!
//! RIG-015: Behavioral proof that the RIG-014 streaming loop detector
//! ABORT actually (1) stops the in-flight provider stream mid-stream and
//! (2) re-prompts the LLM with a corrective note on the next turn.
//!
//! Unlike RIG-014's `@integration` scenarios (structural source
//! assertions), these tests drive the FULL production `agent_loop`
//! end-to-end: a real `SessionManager` + `BackgroundSession` + the
//! test-support `"stub"` provider arm. The stub's `StubModel` is
//! configured (via the test-only `set_looping_stream_hook`) to stream
//! 30 normal words followed by an unbounded "the model thinks that"
//! loop, with a shared poll counter. The poll counter proves the stream
//! was actually cancelled mid-stream (polled far fewer times than its
//! full length), and the recorded completion-request chat history proves
//! the next turn's context carries the corrective note.
//!
//! Run with:
//!   cargo test -p codelet-agent-loop --features test-support \
//!       --test rig015_loop_abort_behavioral
//!
//! The `test-support` cfg-gate keeps default `cargo test --workspace`
//! green (the stub provider arm + hook only compile with the feature).
//!
//! RED PHASE: these tests reference
//! `codelet_providers::stub_model::{set_looping_stream_hook,
//! clear_looping_stream_hook, last_request_history}`, which do not exist
//! yet, so the file fails to compile until the stub-model hook is
//! implemented.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[cfg(feature = "test-support")]
mod rig015 {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use codelet_agent_loop::FspecAgentHooks;
    use codelet_rpc_types::StreamChunk;
    use codelet_sessions::session_manager::SessionManager;
    use serial_test::serial;
    use uuid::Uuid;

    /// RIG-015: the persistence layer (SESSION_STORE / MESSAGE_STORE) is a
    /// process-global lazy static that captures the data dir at FIRST init.
    /// All three tests therefore must share ONE long-lived data dir, or the
    /// store's path points at a dropped tempdir. This static keeps a single
    /// tempdir alive for the whole test process; the tests are `#[serial]`
    /// so they never race on the global stores.
    fn shared_data_dir() -> &'static std::path::Path {
        static DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
        DIR.get_or_init(|| {
            tempfile::tempdir()
                .expect("shared data dir tempdir")
                .keep()
        })
        .as_path()
    }

    /// The looping phrase the synthetic model degenerates into.
    const LOOP_PHRASE: [&str; 4] = ["the", "model", "thinks", "that"];

    /// 30 words of normal (non-looping) prose before the loop onset.
    fn normal_words() -> Vec<String> {
        let words = [
            "the", "architecture", "of", "the", "streaming", "loop", "detector", "relies", "on",
            "word", "level", "tokenization", "with", "a", "bounded", "sliding", "window", "that",
            "evaluates", "four", "distinct", "repetition", "signals", "in", "order", "of",
            "specificity", "before", "escalating",
        ];
        words.iter().map(|w| w.to_string()).collect()
    }

    /// Total stream length: 30 normal words + 300 loop words.
    const TOTAL_STREAM_LEN: usize = 30 + 300;

    /// Build the full looping stream: 30 normal words, then 300 words of
    /// the repeating phrase.
    fn looping_stream() -> Vec<String> {
        let mut stream = normal_words();
        for i in 0..300 {
            stream.push(LOOP_PHRASE[i % LOOP_PHRASE.len()].to_string());
        }
        stream
    }

    /// Drive the full production `agent_loop` with the looping stub
    /// stream. Sends ONLY turn 1 (the looping stream); the caller sends
    /// turn 2 explicitly so it can first observe turn 1's abort. (Sending
    /// both turns up front would let turn 2's `reset_interrupt()` clear
    /// `is_interrupted` before the test can observe turn 1's abort.)
    ///
    /// Returns the chunks-broadcast receiver subscribed BEFORE turn 1 is
    /// sent, so the test does not miss the `Done` chunk the abort emits.
    async fn drive_looping_turn() -> (
        Arc<SessionManager>,
        Arc<codelet_sessions::background_session::BackgroundSession>,
        Arc<AtomicUsize>,
        tokio::sync::broadcast::Receiver<(codelet_rpc_types::SessionId, StreamChunk)>,
    ) {
        // Hermetic data dir (RPC-025 pattern): the persistence layer +
        // provider manager resolve the global data dir. The SESSION_STORE
        // / MESSAGE_STORE singletons capture this path at first init, so
        // all tests share one long-lived dir (see shared_data_dir).
        let _ = codelet_common::set_data_directory(shared_data_dir().to_path_buf());

        let manager = Arc::new(SessionManager::new());
        manager.set_hooks(Arc::new(FspecAgentHooks::new()));

        codelet_providers::stub_provider::register_stub_provider();
        manager.set_default_model("stub/canned");

        let poll = Arc::new(AtomicUsize::new(0));
        codelet_providers::stub_model::set_looping_stream_hook(looping_stream(), Arc::clone(&poll));

        let tmp = tempfile::tempdir().expect("tempdir");
        let project = tmp
            .path()
            .to_str()
            .expect("tempdir path is utf8")
            .to_string();
        let session_uuid = Uuid::new_v4();
        let session_id_str = session_uuid.to_string();
        manager
            .create_session_with_id(&session_id_str, "stub/canned", &project, "rig015-session")
            .await
            .expect("create_session_with_id");

        let session = manager
            .get_session(&session_id_str)
            .expect("session must exist after create_session_with_id");

        // Subscribe BEFORE sending input so the abort's Done chunk is not
        // missed (broadcast channels are fire-and-forget).
        let chunks_rx = manager.chunks_tx().subscribe();

        // Turn 1: the looping stream (detector aborts mid-stream).
        session
            .send_input("hello".to_string(), None)
            .expect("send_input turn 1");

        (manager, session, poll, chunks_rx)
    }

    /// Wait for the loop-detector abort: the stream must have started
    /// polling (poll > 0) AND the session must report interrupted.
    /// Returns the number of words polled before the abort.
    async fn wait_for_abort(
        session: &codelet_sessions::background_session::BackgroundSession,
        poll: &Arc<AtomicUsize>,
    ) -> usize {
        let _ = wait_until(
            || {
                poll.load(Ordering::Acquire) > 0 && session.is_interrupted.load(Ordering::Acquire)
            },
            Duration::from_secs(20),
        )
        .await;
        poll.load(Ordering::Acquire)
    }

    /// Poll `pred` until true or `timeout` elapses.
    async fn wait_until(mut pred: impl FnMut() -> bool, timeout: Duration) -> bool {
        let deadline = tokio::time::Instant::now() + timeout;
        while tokio::time::Instant::now() < deadline {
            if pred() {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        pred()
    }

    // =========================================================================
    // Scenario: Looping stream is cancelled mid-stream and the turn ends
    // =========================================================================

    #[serial]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn scenario_looping_stream_cancelled_mid_stream_and_turn_ends() {
        // @step Given a stub-provider session whose model streams 40 normal words then an unbounded "the model thinks that" loop
        let (_manager, session, poll, mut chunks_rx) = drive_looping_turn().await;

        // @step When the agent loop processes the turn
        // (the agent_loop task, spawned by FspecAgentHooks on session
        // creation, streams the looping stub stream; the RIG-014 detector
        // fires and session.interrupt() cancels the in-flight stream.)
        let polled = wait_for_abort(&session, &poll).await;

        // @step Then the in-flight provider stream is cancelled before the full stream is consumed
        // @step And the model's stream is polled far fewer times than its full length
        //
        // The poll counter counts every RawStreamingChoice the stream loop
        // actually consumed. The RIG-014 auto-continue mechanism re-drives
        // the looping stream after turn 1's abort (up to 10 synthetic
        // "Continue" retry turns), so the CUMULATIVE counter spans up to 11
        // turns. If cancellation were broken, each turn would drain all 330
        // words (11 * 330 = 3630 total). A cancelled stream stops mid-way on
        // every turn, so the cumulative total stays far below that ceiling.
        assert!(
            polled > 0,
            "the stream must have started polling before the abort"
        );
        assert!(
            polled < TOTAL_STREAM_LEN * 11,
            "the in-flight stream must be cancelled mid-stream on every turn: only {polled} of {} cumulative words were polled (a non-cancelled stream would poll {} across all 11 turns)",
            TOTAL_STREAM_LEN * 11,
            TOTAL_STREAM_LEN * 11
        );

        // @step And the session reports interrupted
        assert!(
            session.is_interrupted.load(Ordering::Acquire),
            "session.is_interrupted must be true after the loop-detector abort"
        );

        // @step And the turn completes with an interrupted/done chunk on the chunks broadcast
        // (The stream loop's interrupt path emits StreamChunk::Interrupted
        // followed by StreamChunk::Done via the manager-owned broadcast.
        // `chunks_rx` was subscribed before turn 1 was sent, so the Done
        // chunk cannot be missed.)
        let session_id_str = session.id.to_string();
        let mut saw_done = false;
        let _ = wait_until(
            || {
                loop {
                    match chunks_rx.try_recv() {
                        Ok((sid, chunk)) => {
                            if sid.value == session_id_str && matches!(chunk, StreamChunk::Done) {
                                saw_done = true;
                                return true;
                            }
                        }
                        Err(_) => return saw_done,
                    }
                }
            },
            Duration::from_secs(5),
        )
        .await;
        assert!(
            saw_done,
            "a StreamChunk::Done must be broadcast after the loop-detector abort"
        );

        // Clean up the global hook so it does not leak into other tests.
        codelet_providers::stub_model::clear_looping_stream_hook();
    }

    // =========================================================================
    // Scenario: Persisted assistant message is truncated with the marker note
    // =========================================================================

    #[serial]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn scenario_persisted_assistant_message_truncated_with_marker_note() {
        // @step Given a stub-provider session whose model streams 40 normal words then an unbounded "the model thinks that" loop
        let (manager, session, poll, _chunks_rx) = drive_looping_turn().await;

        // @step When the agent loop processes the turn and the loop detector aborts
        let _ = wait_for_abort(&session, &poll).await;
        // Send turn 2 so the turn fully settles (and the corrective note is
        // staged for the next turn) — matching the production two-turn flow.
        session
            .send_input("continue".to_string(), None)
            .expect("send_input turn 2");
        // Allow the persistence write (on the interrupt/Done path) to land.
        let _ = wait_until(
            || {
                codelet_core::persistence::manifest::load_session(session.id)
                    .map(|m| !m.messages.is_empty())
                    .unwrap_or(false)
            },
            Duration::from_secs(5),
        )
        .await;

        // @step Then the persisted assistant message contains the normal prose up to the loop onset
        // @step And the persisted assistant message ends with the marker note stating the response was cut off due to repetitive output
        // @step And the persisted assistant message does NOT contain the degenerate looping tail
        let manifest = codelet_core::persistence::manifest::load_session(session.id)
            .expect("session manifest must be loadable after the abort");
        let stored = codelet_core::persistence::manifest::get_session_messages(&manifest)
            .expect("session messages must load");
        let assistant_text: String = stored
            .iter()
            .filter(|m| m.role == "assistant")
            .map(|m| m.content.clone())
            .collect::<Vec<_>>()
            .join("\n");

        let marker = "Response cut off: repetitive output detected";
        assert!(
            assistant_text.contains(marker),
            "the persisted assistant message must end with the RIG-014 marker note \
             ('{marker}'); got: {assistant_text}"
        );
        // The normal prose up to loop onset must be retained.
        assert!(
            assistant_text.contains("streaming loop detector"),
            "the persisted assistant message must retain the normal prose up to the \
             loop onset; got: {assistant_text}"
        );
        // The degenerate tail (many repetitions of the phrase) must be dropped.
        let degenerate = "the model thinks that the model thinks that the model thinks that";
        assert!(
            !assistant_text.contains(degenerate),
            "the persisted assistant message must NOT contain the degenerate looping \
             tail; got: {assistant_text}"
        );

        // Keep `manager` alive (the session is owned by it).
        drop(manager);
        codelet_providers::stub_model::clear_looping_stream_hook();
    }

    // =========================================================================
    // Scenario: Next turn's context carries the corrective note
    // =========================================================================

    #[serial]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn scenario_next_turn_context_carries_corrective_note() {
        // @step Given a session whose previous turn was aborted by the streaming loop detector
        let (_manager, session, poll, _chunks_rx) = drive_looping_turn().await;

        // Wait for turn 1's abort, then send turn 2 so the agent loop's
        // take_pending_loop_abort_note path injects the corrective note as
        // a User message into turn 2's context before the model is prompted.
        let _ = wait_for_abort(&session, &poll).await;
        session
            .send_input("continue".to_string(), None)
            .expect("send_input turn 2");

        // @step When the agent loop starts the next turn
        // (The StubModel records the request chat history.)
        let note_present = wait_until(
            || {
                codelet_providers::stub_model::last_request_history()
                    .iter()
                    .any(|m| m.contains("repetitive output"))
            },
            Duration::from_secs(10),
        )
        .await;

        // @step Then the next completion request's chat history contains the corrective note
        assert!(
            note_present,
            "the next turn's completion request chat history must contain the \
             corrective note (mentioning 'repetitive output')"
        );

        // @step And the corrective note states the previous response was cut off due to repetitive output
        // @step And the corrective note instructs the model to continue with a fresh approach without repeating its earlier reasoning
        let hist = codelet_providers::stub_model::last_request_history();
        let note = hist
            .iter()
            .find(|m| m.contains("repetitive output"))
            .expect("a corrective note must be present (checked above)");
        assert!(
            note.contains("fresh approach"),
            "the corrective note must instruct a fresh approach; got: {note}"
        );
        assert!(
            note.contains("do not repeat") || note.contains("without repeating"),
            "the corrective note must instruct not to repeat earlier reasoning; got: {note}"
        );

        codelet_providers::stub_model::clear_looping_stream_hook();
    }
}
