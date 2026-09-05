//! PROV-145: per-profile loop-detection runtime wiring in the agent loop.
//!
//! Feature: spec/features/per-profile-loop-detection-session-wiring.feature
//! (PER-TURN DETECTOR CONSTRUCTION + RETRY CAP scenarios)
//!
//! RED PHASE: `codelet_agent_loop::background_output::LoopDetectionWiring`
//! does not exist yet, so this target fails to compile until the
//! implementation lands.
//!
//! Design (research §5.1): the detector is rebuilt per turn from the
//! session's current profile selection. These tests drive
//! `BackgroundOutput::emit` directly (cont008 pattern — no live provider)
//! with a wiring built from the stored profile values:
//!
//! * **Deterministic loop stream**: 10 unique normal words followed by 40
//!   words of "alpha beta gamma" (the phrase × 10, then 10 more — 50 total).
//!   The tail-3 n-gram "alpha beta gamma" reaches a window count of 10 at
//!   word 40 (160-word default window, threshold 10), so the DEFAULT wiring
//!   fires + aborts (trigger 1 at word 40 warns; the latched detector
//!   re-triggers at word 41 within the 30s cooldown → abort).
//! * **Loose wiring** (window 30, maxRepeats 12): the 30-word window holds
//!   at most 10 tail copies, below the threshold; the small window also
//!   disables the long-suffix / periodicity / diversity signals (research
//!   §7 item 5). The detector NEVER fires — proving the stored values
//!   loosen the detector.
//! * **Tight wiring** (maxRepeats 5): the detector-level fire index is
//!   25 (the 5th repeat) via the public `StreamLoopDetector::with_config`,
//!   and the wiring-level abort stages the corrective note — proving the
//!   stored threshold aborts earlier than the default 10.
//! * **Disabled wiring** (loopDetectionEnabled false): the detector path
//!   is a no-op — the same stream never stages a note.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use codelet_agent_loop::background_output::LoopDetectionWiring;
use codelet_agent_loop::{BackgroundOutput, stream_loop_detector::StreamLoopDetector};
use codelet_cli::interactive::StreamOutput;
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_sessions::background_session::BackgroundSession;
use codelet_sessions::session_manager::SessionManager;
use serial_test::serial;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// PROV-132/cont009 precedent: serialize tests that swap the process-global
/// data directory.
static DATA_DIR_GUARD: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Trimmed offline models.dev catalog (openai only). The same fixture as
/// rust/fspec-tui/tests/fixtures/prov101_models.json — inlined here so the
/// agent-loop crate stays self-contained (no cross-crate fixture paths).
const MODELS_FIXTURE: &str = r#"{
  "openai": {
    "id": "openai",
    "name": "OpenAI",
    "env": ["OPENAI_API_KEY"],
    "models": {
      "o3": {
        "id": "o3",
        "name": "o3",
        "reasoning": true,
        "tool_call": true,
        "attachment": true,
        "temperature": false,
        "limit": { "context": 200000, "output": 100000 }
      }
    }
  }
}
"#;

/// Create a fresh BackgroundSession (Noop hooks — no agent loop spawned).
/// Mirrors rust/fspec-tui/tests/cont008_goal_back_sync_test.rs.
async fn fresh_background_session() -> (
    tempfile::TempDir,
    Arc<SessionManager>,
    Arc<BackgroundSession>,
) {
    let data_dir = tempfile::tempdir().expect("tempdir");
    let cache_dir = data_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).expect("create cache dir");
    std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).expect("write models.json");
    // RPC-423 precedent (cont009): reset stores BEFORE setting data
    // directory so init_session_store() points at the fresh temp dir.
    codelet_core::persistence::reset_stores_for_tests();
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    std::env::set_var("OPENAI_API_KEY", "prov145-fake-key");
    let manager = Arc::new(SessionManager::new());
    manager.set_default_model("openai/o3");
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let sid = handle.create_session(None);
    let session = manager
        .get_session(&sid.value)
        .expect("session must exist after create_session");
    (data_dir, manager, session)
}

/// The 10 normal (distinct) words of the loop stream's prefix.
const NORMAL_WORDS: [&str; 10] = [
    "the", "architecture", "of", "the", "streaming", "loop",
    "detector", "relies", "on", "word",
];

/// The looping phrase (3 words).
const LOOP_PHRASE: [&str; 3] = ["alpha", "beta", "gamma"];

/// Build the full deterministic loop stream: 10 normal words + 50 loop
/// words. The extra 10 loop words (beyond the phrase × 10) keep feeding
/// the LATCHED default detector after it fires at word 40, so the
/// escalation policy's cooldown re-trigger escalates to abort.
fn loop_stream_words() -> Vec<String> {
    let mut words: Vec<String> = NORMAL_WORDS.iter().map(|w| w.to_string()).collect();
    for i in 0..50 {
        words.push(LOOP_PHRASE[i % LOOP_PHRASE.len()].to_string());
    }
    words
}

/// Feed the loop stream one word per `StreamEvent::Text` delta through the
/// given BackgroundOutput.
fn feed_loop_stream(output: &BackgroundOutput, words: &[String]) {
    for word in words {
        output.emit(codelet_cli::interactive::StreamEvent::Text(
            word.clone(),
        ));
    }
}

// ---------------------------------------------------------------------------
// PER-TURN DETECTOR CONSTRUCTION
// ---------------------------------------------------------------------------

/// Scenario: A stored loopDetectionEnabled false disables the detector for the session's turns
#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stored_loop_detection_enabled_false_disables_the_detector() {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given a per-turn loop-detection wiring built from the stored value loopDetectionEnabled false
    let (_dir, _manager, session) = fresh_background_session().await;
    let output = BackgroundOutput::with_provider(
        session.clone(),
        "openai".to_string(),
        Some(LoopDetectionWiring {
            enabled: false,
            ..LoopDetectionWiring::default()
        }),
    );

    // @step When the session's streaming loop detector path feeds 100 words of a degenerating repeating loop
    let words = loop_stream_words();
    feed_loop_stream(&output, &words);

    // @step Then no loop-detector abort fires for that turn
    assert!(
        !session.has_pending_loop_abort_note(),
        "a disabled wiring must never stage a corrective note"
    );

    // @step And the stream is never cancelled by the loop detector and no corrective note is staged
    assert!(
        !session.is_interrupted.load(Ordering::Acquire),
        "a disabled wiring must never interrupt the session"
    );
}

/// Scenario: A wiring built from absent values keeps the detector enabled
#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_wiring_built_from_absent_values_keeps_the_detector_enabled() {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given a per-turn loop-detection wiring built from all absent values
    let (_dir, _manager, session) = fresh_background_session().await;
    let output = BackgroundOutput::with_provider(
        session.clone(),
        "openai".to_string(),
        Some(LoopDetectionWiring::default()),
    );

    // @step When the session's streaming loop detector path feeds a 10-word normal prefix followed by "alpha beta gamma" repeated 10 times
    feed_loop_stream(&output, &loop_stream_words());

    // @step Then the loop detector fires and a corrective note is staged on the session (today's behavior preserved)
    assert!(
        session.has_pending_loop_abort_note(),
        "the default wiring (absent values) must fire + abort + stage the corrective note"
    );
}

/// Scenario: A stored window and repeat threshold loosen the detector
#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stored_window_and_repeat_threshold_loosen_the_detector() {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given a per-turn loop-detection wiring built from the stored values loopDetectionWindow 30 and loopDetectionMaxRepeats 12
    let (_dir, _manager, session) = fresh_background_session().await;
    let output = BackgroundOutput::with_provider(
        session.clone(),
        "openai".to_string(),
        Some(LoopDetectionWiring {
            window: 30,
            max_repeats: 12,
            ..LoopDetectionWiring::default()
        }),
    );

    // @step When the session's streaming loop detector path feeds a 10-word normal prefix followed by "alpha beta gamma" repeated 10 times
    feed_loop_stream(&output, &loop_stream_words());

    // @step Then no loop signal fires (the 30-word window holds fewer repeats than the stored threshold 12, and the small window disables the long-suffix, periodicity, and diversity signals)
    assert!(
        !session.has_pending_loop_abort_note(),
        "a loose wiring (window 30, threshold 12) must NOT fire: the window holds at most 10 tail copies"
    );

    // @step And the same stream fed through the default wiring DOES fire the n-gram repeat signal
    // (detector-level proof: the default config fires on the same stream)
    let mut default_det = StreamLoopDetector::new();
    let fired = loop_stream_words()
        .iter()
        .any(|w| default_det.feed(w).is_some());
    assert!(
        fired,
        "the default detector MUST fire on the same stream (proving the loose wiring is what suppressed it)"
    );
}

/// Scenario: A stored lower repeat threshold aborts earlier
#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stored_lower_repeat_threshold_aborts_earlier() {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given a per-turn loop-detection wiring built from the stored value loopDetectionMaxRepeats 5
    let (_dir, _manager, session) = fresh_background_session().await;
    let output = BackgroundOutput::with_provider(
        session.clone(),
        "openai".to_string(),
        Some(LoopDetectionWiring {
            max_repeats: 5,
            ..LoopDetectionWiring::default()
        }),
    );

    // @step When the session's streaming loop detector path feeds a 10-word normal prefix followed by "alpha beta gamma" repeated 10 times
    feed_loop_stream(&output, &loop_stream_words());

    // @step Then the loop detector fires after the 5th repetition of the tail
    // (detector-level: the fire index is word 25 — 10 normal + 15 loop words)
    let mut tight_det = StreamLoopDetector::with_config(
        codelet_agent_loop::stream_loop_detector::LoopDetectorConfig {
            max_repeats: 5,
            ..codelet_agent_loop::stream_loop_detector::LoopDetectorConfig::default()
        },
    );
    let fire_index = loop_stream_words()
        .iter()
        .enumerate()
        .find(|(_, w)| tight_det.feed(w).is_some())
        .map(|(i, _)| i)
        .expect("the tight detector must fire on the stream");
    assert_eq!(
        fire_index, 24, // 0-based: word 25
        "a threshold of 5 must fire at the 5th repeat (word index 24), not later"
    );

    // @step And the default wiring feeds 15 more words before it fires (repeat threshold 10)
    // (wiring-level: the abort DID stage the corrective note)
    assert!(
        session.has_pending_loop_abort_note(),
        "the tight wiring must abort + stage the corrective note"
    );
    let mut default_det = StreamLoopDetector::new();
    let default_fire_index = loop_stream_words()
        .iter()
        .enumerate()
        .find(|(_, w)| default_det.feed(w).is_some())
        .map(|(i, _)| i)
        .expect("the default detector must fire on the stream");
    assert!(
        default_fire_index > fire_index,
        "the default threshold (10) must fire LATER than the stored threshold (5): {default_fire_index} > {fire_index}"
    );
}

// ---------------------------------------------------------------------------
// RETRY CAP
// ---------------------------------------------------------------------------

/// Scenario: The per-turn retry cap resolves from the stored loopDetectionMaxRetries
#[test]
fn the_per_turn_retry_cap_resolves_from_the_stored_max_retries() {
    // @step Given a per-turn loop-detection wiring built from the stored value loopDetectionMaxRetries 2
    let wiring = LoopDetectionWiring {
        max_retries: 2,
        ..LoopDetectionWiring::default()
    };

    // @step When the agent loop reads the loop-abort auto-continue retry cap for the turn
    let cap = wiring.max_retries;

    // @step Then the cap is 2
    assert_eq!(cap, 2, "the wiring's stored cap must be 2");

    // @step And a wiring built from an absent value reads the cap 10 (the RIG-014 default)
    let default_cap = LoopDetectionWiring::default().max_retries;
    assert_eq!(
        default_cap, 10,
        "an absent value must read the RIG-014 default cap of 10"
    );
}

/// Scenario: The agent loop reads the per-turn retry cap from the profile resolution
#[test]
fn the_agent_loop_reads_the_per_turn_retry_cap_from_the_profile_resolution() {
    // Structural (source-shape) assertion, mirroring
    // rig014_streaming_loop_detection.rs — the retry cap must come from the
    // per-turn profile resolution (not the hard-coded const), and the
    // counter must keep its per-user-turn reset.
    let agent_loop_src = read_source("rust/agent-loop/src/agent_loop.rs");

    // @step Given the agent loop constructs the per-turn BackgroundOutput from the session's provider/model
    assert!(
        agent_loop_src.contains("BackgroundOutput::with_provider"),
        "the agent loop must construct the per-turn BackgroundOutput"
    );

    // @step When the turn's loop-detection wiring is resolved
    assert!(
        agent_loop_src.contains("resolve_profile_loop_detection"),
        "the agent loop must resolve the per-turn profile loop-detection wiring"
    );

    // @step Then the loop-abort retry cap comes from that resolution (the hard-coded const RIG014_MAX_LOOP_ABORT_RETRIES is gone)
    assert!(
        !agent_loop_src.contains("RIG014_MAX_LOOP_ABORT_RETRIES"),
        "the hard-coded const RIG014_MAX_LOOP_ABORT_RETRIES must be gone (replaced by the per-turn resolution)"
    );

    // @step And the retry counter still resets on genuine user input (per-user-turn semantics preserved)
    assert!(
        agent_loop_src.contains("loop_abort_retry_count = 0"),
        "the retry counter must still reset on genuine user input"
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn read_source(rel: &str) -> String {
    // CARGO_MANIFEST_DIR = rust/agent-loop; walk up two parents to repo root.
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join(rel))
        .unwrap_or_else(|| PathBuf::from(rel));
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}
