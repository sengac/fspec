//! PROV-145: per-profile loop-detection runtime wiring in the agent loop.
//!
//! Feature: spec/features/per-profile-loop-detection-detector-construction.feature
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

mod prov145_profile_config_applied_support;

use std::sync::atomic::Ordering;

use codelet_agent_loop::background_output::LoopDetectionWiring;
use codelet_agent_loop::{stream_loop_detector::StreamLoopDetector, BackgroundOutput};
use codelet_sessions::model_resolution::ProfileLoopDetection;
use prov145_profile_config_applied_support::{
    feed_loop_stream, fresh_background_session, loop_stream_words, read_source, DATA_DIR_GUARD,
};
use serial_test::serial;

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

    // @step Given a per-turn loop-detection wiring converted from a resolution with all values absent
    let (_dir, _manager, session) = fresh_background_session().await;
    let output = BackgroundOutput::with_provider(
        session.clone(),
        "openai".to_string(),
        Some(LoopDetectionWiring::from(ProfileLoopDetection::default())),
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
        fire_index,
        24, // 0-based: word 25
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

/// Scenario: The agent loop converts the profile resolution without literal fallbacks
#[test]
fn the_agent_loop_converts_the_profile_resolution_without_literal_fallbacks() {
    // @step Given a profile resolution that stores only loopDetectionWindow 30
    let resolved = ProfileLoopDetection {
        window: Some(30),
        ..ProfileLoopDetection::default()
    };
    // @step When it is converted into the per-turn loop-detection wiring
    let wiring = LoopDetectionWiring::from(resolved);
    // @step Then the wiring keeps window 30 and takes the named RIG-014 defaults for the absent values
    use codelet_agent_loop::loop_detection_defaults as defaults;
    assert_eq!(
        wiring,
        LoopDetectionWiring {
            enabled: defaults::DEFAULT_LOOP_DETECTION_ENABLED,
            window: 30,
            max_repeats: defaults::DEFAULT_LOOP_DETECTION_MAX_REPEATS,
            max_retries: defaults::DEFAULT_LOOP_DETECTION_MAX_RETRIES,
        }
    );
    assert_eq!(
        LoopDetectionWiring::from(ProfileLoopDetection::default()),
        LoopDetectionWiring::default()
    );
    // @step And the agent loop builds its wiring with LoopDetectionWiring::from and carries no literal fallback values
    let agent_loop = read_source("rust/agent-loop/src/agent_loop.rs");
    assert!(agent_loop.contains("LoopDetectionWiring::from("));
    for literal in ["unwrap_or(true)", "unwrap_or(160)", "unwrap_or(10)"] {
        assert!(!agent_loop.contains(literal), "literal fallback {literal}");
    }
}
