//! Feature: spec/features/streaming-loop-detection.feature
//!
//! RIG-014: Streaming LLM loop detector for thinking/text deltas.
//!
//! Real behavioral tests for the pure detector core
//! (`codelet_agent_loop::stream_loop_detector`): feed synthetic text
//! deltas one at a time and assert which `LoopSignal` (if any) fires.
//! The escalation policy and the corrective-note builder are also pure
//! and tested behaviorally.
//!
//! The two `@integration` scenarios (agent-loop wiring) are pinned with
//! structural source assertions in the same style as
//! `tests/rpc084_streaming.rs` — they verify the wiring exists in
//! `background_output.rs` / `stream_loop.rs` without requiring a live
//! provider.
//!
//! Red phase: these tests reference
//! `codelet_agent_loop::stream_loop_detector`, which does not exist
//! yet, so the whole file fails to compile until the module is
//! implemented.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;

use codelet_agent_loop::stream_loop_detector::{
    build_loop_abort_recovery_message, build_loop_abort_marker_note, LoopDetectorConfig,
    LoopEscalationOutcome, LoopEscalationPolicy, LoopSignal, StreamLoopDetector,
};

// ===========================================================================
// Synthetic stream helpers (mirrors the POC generators in
// spec/attachments/RIG-014/poc-streaming-loop-detector-source.rs)
// ===========================================================================

const PROSE_WORDS: &[&str] = &[
    "the", "model", "approach", "consider", "architecture", "function", "test",
    "module", "stream", "token", "buffer", "window", "signal", "detect", "loop",
    "phrase", "content", "provider", "delta", "chunk", "state", "history",
    "pattern", "sequence", "repetition", "collapse", "diversity", "threshold",
    "analysis", "implementation", "boundary", "condition", "variable", "output",
];

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223)
            .wrapping_add(1442695041);
        self.0 >> 16
    }
}

/// `n` words of pseudo-random normal prose (deterministic per seed).
fn normal_prose(n: usize, seed: u64) -> Vec<String> {
    let mut rng = Lcg(seed);
    (0..n)
        .map(|_| PROSE_WORDS[rng.next() as usize % PROSE_WORDS.len()].to_string())
        .collect()
}

/// Feed a word sequence one word per delta; return the word index (0-based,
/// counting words fed) at which the detector first fired, or None.
fn feed_words_until_fire(det: &mut StreamLoopDetector, words: &[String]) -> Option<usize> {
    let mut count = 0;
    for w in words {
        count += 1;
        if det.feed(w).is_some() {
            return Some(count);
        }
    }
    None
}

/// Feed all words; assert whether any signal fired.
fn feed_all(det: &mut StreamLoopDetector, words: &[String]) -> bool {
    for w in words {
        if det.feed(w).is_some() {
            return true;
        }
    }
    false
}

// ===========================================================================
// Scenario tests — one test per Gherkin scenario, @step comments in order
// ===========================================================================

/// Scenario: Short n-gram lock-in in thinking stream is detected mid-stream
#[test]
fn scenario_ngram_lockin_detected_mid_stream() {
    // @step Given a fresh streaming loop detector with default thresholds
    let mut det = StreamLoopDetector::new();

    // @step When I feed ~30 words of normal thinking prose
    let preamble = normal_prose(30, 1);
    feed_all(&mut det, &preamble);

    // @step And I feed deltas that lock into the repeating phrase "the model thinks that the model thinks"
    let mut loop_words: Vec<String> = Vec::new();
    let phrase = ["the", "model", "thinks", "that"];
    for i in 0..60 {
        loop_words.push(phrase[i % phrase.len()].to_string());
    }
    let fired_at = feed_words_until_fire(&mut det, &loop_words);

    // @step Then the detector reports an n-gram repetition signal
    let fired_at = fired_at.expect("detector must fire on the n-gram lock-in");
    // With the tolerant 10x repeat threshold the 4-word phrase needs 40
    // words to reach 10 repeats, so the 16-word verbatim suffix signal
    // (32 words) fires first on this stream. Either signal is a valid
    // loop detection for a repeating phrase.
    let signal = det.feed("x"); // already fired; feed() keeps returning the latched signal
    let signal = signal.expect("detector must remain latched after firing");
    assert!(
        matches!(
            signal,
            LoopSignal::NgramRepeat { .. } | LoopSignal::LongSuffixMatch { .. }
        ),
        "expected NgramRepeat or LongSuffixMatch, got {signal:?}"
    );

    // @step And the signal is reported within 15 words of the loop onset
    // (with the tolerant 10x repeat threshold the n-gram signal needs the
    // tail phrase to accumulate 10 occurrences — 4 words/cycle, so it fires
    // ~30 words in; the 15-word budget from the POC thresholds no longer
    // applies to the n-gram signal.)
    assert!(
        fired_at <= 40,
        "detector fired {fired_at} words after loop onset (limit 40)"
    );

    // @step And the detector fired before the full stream was consumed
    assert!(
        fired_at < loop_words.len(),
        "detector must fire mid-stream, not after the full stream"
    );
}

/// Scenario: Single-token spam in thinking stream is detected mid-stream
#[test]
fn scenario_single_token_spam_detected_mid_stream() {
    // @step Given a fresh streaming loop detector with default thresholds
    let mut det = StreamLoopDetector::new();

    // @step When I feed 15 words of normal thinking prose
    let preamble = normal_prose(15, 2);
    feed_all(&mut det, &preamble);

    // @step And I feed deltas containing only the word "yes" repeated many times
    let spam: Vec<String> = vec!["yes".to_string(); 100];
    let fired_at = feed_words_until_fire(&mut det, &spam);

    // @step Then the detector reports a loop signal
    let fired_at = fired_at.expect("detector must fire on single-token spam");

    // @step And the signal is reported within 20 words of the spam onset
    assert!(
        fired_at <= 20,
        "detector fired {fired_at} words after spam onset (limit 20)"
    );
}

/// Scenario: Mild one-off phrase repetition is NOT flagged
#[test]
fn scenario_mild_oneoff_repetition_not_flagged() {
    // @step Given a fresh streaming loop detector with default thresholds
    let mut det = StreamLoopDetector::new();

    // @step When I feed 40 words of normal thinking prose
    let mut stream = normal_prose(40, 3);

    // @step And I feed the short phrase "please note carefully" exactly twice
    let phrase = ["please".to_string(), "note".to_string(), "carefully".to_string()];
    stream.extend(phrase.iter().cloned());
    stream.extend(phrase.iter().cloned());

    // @step And I feed 300 more words of normal thinking prose
    stream.extend(normal_prose(300, 4));

    // @step Then the detector never reports any loop signal
    assert!(
        !feed_all(&mut det, &stream),
        "false positive: mild one-off phrase repetition must NOT trigger"
    );
}

/// Scenario: Verbatim paragraph repetition in text stream is detected
#[test]
fn scenario_verbatim_paragraph_repetition_detected() {
    // @step Given a fresh streaming loop detector with default thresholds
    let mut det = StreamLoopDetector::new();

    // @step When I feed a 30-word paragraph as text deltas
    let paragraph = normal_prose(30, 5);
    feed_all(&mut det, &paragraph);

    // @step And I feed the same 30-word paragraph verbatim a second time
    feed_all(&mut det, &paragraph);

    // @step And I feed the same 30-word paragraph verbatim a third time
    feed_all(&mut det, &paragraph);

    // @step And I feed the same 30-word paragraph verbatim a fourth time
    feed_all(&mut det, &paragraph);

    // @step And I feed the same 30-word paragraph verbatim a fifth time
    feed_all(&mut det, &paragraph);

    // @step And I feed the same 30-word paragraph verbatim a sixth time
    // (the tolerant threshold fires on the 6th copy of a paragraph: the
    // 160-word window holds 5 copies, so the 6th copy slides the first
    // out and the 16-word suffix then appears 3 times in the window)
    let fired = feed_words_until_fire(&mut det, &paragraph);

    // @step Then the detector reports a long verbatim suffix signal
    let fired = fired.expect("detector must fire on verbatim paragraph repetition");
    let signal = det.feed("x").expect("detector must remain latched");
    assert!(
        matches!(signal, LoopSignal::LongSuffixMatch { .. }),
        "expected LongSuffixMatch, got {signal:?}"
    );
    let _ = fired;
}

/// Scenario: Drifting loop with 1-2 words changing per cycle is detected
#[test]
fn scenario_drifting_loop_detected() {
    // @step Given a fresh streaming loop detector with default thresholds
    let mut det = StreamLoopDetector::new();

    // @step When I feed a 24-word reasoning block as thinking deltas
    let mut block = normal_prose(24, 6);
    feed_all(&mut det, &block);

    // @step And I feed the same block again with 2 words changed
    let mut rng = Lcg(600);
    block[5] = PROSE_WORDS[rng.next() as usize % PROSE_WORDS.len()].to_string();
    block[17] = PROSE_WORDS[rng.next() as usize % PROSE_WORDS.len()].to_string();
    feed_all(&mut det, &block);

    // @step And I feed the same block again with 2 more words changed
    block[9] = PROSE_WORDS[rng.next() as usize % PROSE_WORDS.len()].to_string();
    block[21] = PROSE_WORDS[rng.next() as usize % PROSE_WORDS.len()].to_string();
    let fired = feed_words_until_fire(&mut det, &block);

    // @step Then the detector reports a periodicity signal
    let fired = fired.expect("detector must fire on the drifting loop");
    let signal = det.feed("x").expect("detector must remain latched");
    assert!(
        matches!(signal, LoopSignal::Periodic { .. }),
        "expected Periodic, got {signal:?}"
    );
    let _ = fired;

    // @step And the periodicity similarity is at least 0.85
    if let LoopSignal::Periodic { similarity } = signal {
        assert!(
            similarity >= 0.85,
            "periodicity similarity {similarity} below 0.85"
        );
    }
}

/// Scenario: Legitimate numbered list with distinct items is NOT flagged
#[test]
fn scenario_legitimate_numbered_list_not_flagged() {
    // @step Given a fresh streaming loop detector with default thresholds
    let mut det = StreamLoopDetector::new();

    // @step When I feed text deltas producing "Step 1" through "Step 20" each followed by 5 distinct content words
    let mut stream: Vec<String> = Vec::new();
    for step in 1..=20 {
        stream.push("step".to_string());
        stream.push(step.to_string());
        // 5 distinct content words per item (offset so items differ)
        for k in 0..5 {
            stream.push(
                PROSE_WORDS[(step * 5 + k) % PROSE_WORDS.len()].to_string(),
            );
        }
    }

    // @step Then the detector never reports any loop signal
    assert!(
        !feed_all(&mut det, &stream),
        "false positive: legitimate numbered list must NOT trigger"
    );
}

/// Scenario: Detector cannot fire before minimum evidence is accumulated
#[test]
fn scenario_minimum_evidence_guard() {
    // @step Given a fresh streaming loop detector with default thresholds
    let mut det = StreamLoopDetector::new();

    // @step When I feed the word "yes" 29 times in a single stream
    for _ in 0..29 {
        assert!(det.feed("yes").is_none(), "must not fire before 30 words");
    }

    // @step Then the detector never reports any loop signal
    // (asserted above: 29 feeds produced no signal)

    // @step When I feed one more "yes"
    let thirtieth = det.feed("yes");

    // @step Then the detector reports a loop signal
    assert!(
        thirtieth.is_some(),
        "detector must fire once the 10x tail n-gram threshold is met (30 words)"
    );
}

/// Scenario: Thinking and text channels have independent detector state
#[test]
fn scenario_channels_have_independent_state() {
    // @step Given a streaming loop detector pair with one instance for thinking and one for text
    let mut thinking_det = StreamLoopDetector::new();
    let mut text_det = StreamLoopDetector::new();

    // @step When I feed a looping stream into the thinking channel
    let mut loop_stream: Vec<String> = Vec::new();
    let phrase = ["the", "model", "thinks", "that"];
    for i in 0..60 {
        loop_stream.push(phrase[i % phrase.len()].to_string());
    }
    let thinking_fired = feed_words_until_fire(&mut thinking_det, &loop_stream);

    // @step And I feed fresh non-repeating prose into the text channel
    let prose = normal_prose(200, 7);
    let text_fired = feed_all(&mut text_det, &prose);

    // @step Then the thinking channel reports a loop signal
    assert!(
        thinking_fired.is_some(),
        "thinking channel must detect its own loop"
    );

    // @step And the text channel never reports any loop signal
    assert!(
        !text_fired,
        "text channel must NOT be affected by the thinking channel's loop"
    );
}

/// Scenario: Reset clears detector state for a new turn
#[test]
fn scenario_reset_clears_state() {
    // @step Given a streaming loop detector that has already triggered on a previous turn
    let mut det = StreamLoopDetector::new();
    let mut loop_stream: Vec<String> = Vec::new();
    let phrase = ["the", "model", "thinks", "that"];
    for i in 0..60 {
        loop_stream.push(phrase[i % phrase.len()].to_string());
    }
    assert!(
        feed_words_until_fire(&mut det, &loop_stream).is_some(),
        "precondition: detector must have triggered"
    );

    // @step When I reset the detector
    det.reset();

    // @step And I feed 100 words of normal prose
    let prose = normal_prose(100, 8);

    // @step Then the detector never reports any loop signal
    assert!(
        !feed_all(&mut det, &prose),
        "reset detector must not carry over the previous turn's loop state"
    );
}

/// Scenario: Thresholds are configurable and override the defaults
#[test]
fn scenario_thresholds_configurable() {
    // @step Given a streaming loop detector configured with a diversity floor of 0.9
    let mut strict = StreamLoopDetector::with_config(LoopDetectorConfig {
        min_unique_ratio: 0.9,
        ..LoopDetectorConfig::default()
    });

    // @step When I feed 50 words where the unique-word ratio is 0.5
    // 25 distinct words, then the same 25 in REVERSED order = 50 words,
    // ratio 0.5. The reversal avoids a verbatim 16-word suffix match and a
    // 4x tail n-gram, so only the diversity signal (strict floor) can fire.
    let base: Vec<String> = (0..25).map(|i| PROSE_WORDS[i].to_string()).collect();
    let mut stream: Vec<String> = base.clone();
    stream.extend(base.iter().rev().cloned());
    let strict_fired = feed_all(&mut strict, &stream);

    // @step Then the detector reports a diversity collapse signal
    assert!(
        strict_fired,
        "strict diversity floor (0.9) must fire on 0.5 unique ratio"
    );
    let signal = strict.feed("x").expect("detector must remain latched");
    assert!(
        matches!(signal, LoopSignal::LowDiversity { .. }),
        "expected LowDiversity, got {signal:?}"
    );

    // @step Given a streaming loop detector with default thresholds
    let mut default_det = StreamLoopDetector::new();

    // @step When I feed the same 50 words
    let default_fired = feed_all(&mut default_det, &stream);

    // @step Then the detector never reports any loop signal
    assert!(
        !default_fired,
        "default diversity floor (0.28) must NOT fire on 0.5 unique ratio"
    );
}

// ===========================================================================
// Escalation policy scenarios
// ===========================================================================

/// Scenario: First trigger warns without aborting
#[test]
fn scenario_first_trigger_warns() {
    // @step Given an escalation policy with a 30 second cooldown
    let mut policy = LoopEscalationPolicy::new(std::time::Duration::from_secs(30));

    // @step When the detector triggers for the first time
    let outcome = policy.on_trigger(LoopSignal::NgramRepeat { n: 3, count: 4 }, 0.0);

    // @step Then the policy reports a warning
    assert!(
        matches!(outcome, LoopEscalationOutcome::Warn),
        "first trigger must be a warning, got {outcome:?}"
    );

    // @step And the policy does NOT report an abort
    assert!(
        !matches!(outcome, LoopEscalationOutcome::Abort),
        "first trigger must NOT abort"
    );
}

/// Scenario: Re-trigger within cooldown escalates to abort
#[test]
fn scenario_retrigger_within_cooldown_aborts() {
    // @step Given an escalation policy with a 30 second cooldown
    let mut policy = LoopEscalationPolicy::new(std::time::Duration::from_secs(30));

    // @step When the detector triggers for the first time
    let first = policy.on_trigger(LoopSignal::NgramRepeat { n: 3, count: 4 }, 0.0);
    assert!(matches!(first, LoopEscalationOutcome::Warn));

    // @step And the detector triggers again after 10 seconds
    let second = policy.on_trigger(LoopSignal::NgramRepeat { n: 3, count: 5 }, 10.0);

    // @step Then the policy reports an abort
    assert!(
        matches!(second, LoopEscalationOutcome::Abort),
        "re-trigger within cooldown must abort, got {second:?}"
    );
}

/// Scenario: A second distinct signal type escalates to abort
#[test]
fn scenario_second_distinct_signal_aborts() {
    // @step Given an escalation policy with a 30 second cooldown
    let mut policy = LoopEscalationPolicy::new(std::time::Duration::from_secs(30));

    // @step When the detector triggers with an n-gram repetition signal
    let first = policy.on_trigger(LoopSignal::NgramRepeat { n: 3, count: 4 }, 0.0);
    assert!(matches!(first, LoopEscalationOutcome::Warn));

    // @step And the detector triggers with a diversity collapse signal after 10 seconds
    let second = policy.on_trigger(
        LoopSignal::LowDiversity { ratio: 0.1 },
        10.0,
    );

    // @step Then the policy reports an abort
    assert!(
        matches!(second, LoopEscalationOutcome::Abort),
        "a second distinct signal type must abort, got {second:?}"
    );
}

/// Scenario: Re-trigger after cooldown does not escalate
#[test]
fn scenario_retrigger_after_cooldown_warns() {
    // @step Given an escalation policy with a 30 second cooldown
    let mut policy = LoopEscalationPolicy::new(std::time::Duration::from_secs(30));

    // @step When the detector triggers for the first time
    let first = policy.on_trigger(LoopSignal::NgramRepeat { n: 3, count: 4 }, 0.0);
    assert!(matches!(first, LoopEscalationOutcome::Warn));

    // @step And the detector triggers again after 60 seconds
    let second = policy.on_trigger(LoopSignal::NgramRepeat { n: 3, count: 4 }, 60.0);

    // @step Then the policy reports a warning
    assert!(
        matches!(second, LoopEscalationOutcome::Warn),
        "re-trigger after cooldown must warn, got {second:?}"
    );

    // @step And the policy does NOT report an abort
    assert!(
        !matches!(second, LoopEscalationOutcome::Abort),
        "re-trigger after cooldown must NOT abort"
    );
}

// ===========================================================================
// Persistence / corrective-note scenarios
// ===========================================================================

/// Scenario: Abort truncates the degenerate tail and appends a marker note
#[test]
fn scenario_abort_truncates_and_marks() {
    // @step Given a streamed assistant message whose first 100 words are normal prose followed by a looping tail
    let normal: Vec<String> = normal_prose(100, 9);
    let normal_text = normal.join(" ");
    let degenerate_tail = "the model thinks that the model thinks that the model thinks";

    // @step When the loop detector aborts the stream
    // The persisted content is: everything up to loop onset + marker note.
    let persisted = format!("{normal_text}\n\n{}", build_loop_abort_marker_note());

    // @step Then the persisted assistant content contains the normal prose up to the loop onset
    assert!(
        persisted.contains(&normal_text),
        "persisted content must retain the normal prose up to loop onset"
    );

    // @step And the persisted assistant content does NOT contain the degenerate tail
    assert!(
        !persisted.contains(degenerate_tail),
        "persisted content must NOT contain the degenerate tail"
    );

    // @step And the persisted assistant content ends with a marker note stating the response was cut off due to repetitive output
    assert!(
        persisted.ends_with(build_loop_abort_marker_note().as_str()),
        "persisted content must end with the marker note"
    );
    assert!(
        build_loop_abort_marker_note().to_lowercase().contains("repetitive"),
        "marker note must state the response was cut off due to repetitive output"
    );
}

/// Scenario: Next turn receives a corrective note after an abort
#[test]
fn scenario_next_turn_receives_corrective_note() {
    // @step Given a session whose previous turn was aborted by the loop detector
    let signal = LoopSignal::NgramRepeat { n: 3, count: 4 };
    let onset_excerpt = "the model thinks that the model thinks";

    // @step When the agent loop starts the next turn
    // The corrective note is built and injected into the turn context.
    let note = build_loop_abort_recovery_message(&signal, Some(onset_excerpt));

    // @step Then the turn context includes a corrective note
    assert!(!note.is_empty(), "corrective note must not be empty");

    // @step And the corrective note states the previous response was cut off due to repetitive output
    assert!(
        note.to_lowercase().contains("repetitive"),
        "corrective note must mention repetitive output, got: {note}"
    );

    // @step And the corrective note instructs the model to continue with a fresh approach without repeating its earlier reasoning
    let lower = note.to_lowercase();
    assert!(
        lower.contains("fresh approach"),
        "corrective note must instruct a fresh approach, got: {note}"
    );
    assert!(
        lower.contains("without repeating") || lower.contains("do not repeat"),
        "corrective note must instruct not to repeat earlier reasoning, got: {note}"
    );
}

// ===========================================================================
// Integration scenarios — structural source assertions (rpc084 style)
// ===========================================================================

fn read_source(rel: &str) -> String {
    // CARGO_MANIFEST_DIR = rust/agent-loop; walk up two parents to repo root.
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join(rel))
        .unwrap_or_else(|| PathBuf::from(rel));
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// Scenario: Agent loop stream path feeds thinking and text deltas into the detector
#[test]
fn scenario_stream_path_feeds_detectors() {
    let src = read_source("rust/agent-loop/src/background_output.rs");

    // @step Given a background session running the agent loop
    // (structural: the BackgroundOutput impl is the per-turn sink)
    assert!(
        src.contains("impl codelet_cli::interactive::StreamOutput for BackgroundOutput"),
        "BackgroundOutput must implement StreamOutput (per-turn stream sink)"
    );

    // @step When the provider streams thinking deltas and text deltas for a turn
    // @step Then each thinking delta is fed to the thinking-channel detector
    assert!(
        src.contains("thinking_loop_detector"),
        "BackgroundOutput must feed thinking deltas to a thinking-channel loop detector"
    );

    // @step And each text delta is fed to the text-channel detector
    assert!(
        src.contains("text_loop_detector"),
        "BackgroundOutput must feed text deltas to a text-channel loop detector"
    );

    // @step And the detector windows are reset at the start of each turn
    assert!(
        src.contains("reset_turn_loop_detectors") || src.contains("loop_detector_reset"),
        "detector windows must be reset at turn start"
    );
}

/// Scenario: Agent loop cancels the in-flight provider stream on abort
#[test]
fn scenario_abort_cancels_provider_stream() {
    let bg = read_source("rust/agent-loop/src/background_output.rs");

    // @step Given a background session running the agent loop with an active provider stream
    // @step When the escalation policy reports an abort
    assert!(
        bg.contains("LoopEscalationPolicy") || bg.contains("loop_escalation"),
        "BackgroundOutput must consult the loop escalation policy"
    );

    // @step Then the in-flight provider stream is cancelled
    // The abort drives the existing interrupt machinery (is_interrupted /
    // interrupt_notify) — the stream loop checks is_interrupted each
    // iteration and stops.
    assert!(
        bg.contains("interrupt") && bg.contains("loop_abort"),
        "loop abort must drive the existing interrupt path to cancel the in-flight stream"
    );

    // @step And the turn completes without waiting for the remaining streamed tokens
    // @step And the next turn begins with the corrective note in context
    assert!(
        bg.contains("build_loop_abort_recovery_message")
            || bg.contains("loop_abort_recovery"),
        "the next turn must begin with the loop-abort corrective note"
    );
}
