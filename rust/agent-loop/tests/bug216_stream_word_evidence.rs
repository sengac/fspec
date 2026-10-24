//! Feature: spec/features/streaming-loop-false-positive-regressions.feature

use codelet_agent_loop::stream_loop_detector::{
    LoopDetectorConfig, LoopSignal, StreamLoopDetector,
};
use proptest::prelude::*;

const PATH: &str = "rust/fspec-tui/src/app/dispatch_esc_cascade.rs:";

fn technical_prose() -> String {
    format!(
        "Inspect {PATH} for keyboard focus handling. \
         Later compare {PATH} with navigation tests and verify routing behavior. "
    )
}

fn suffix_detector(repeats: usize) -> StreamLoopDetector {
    StreamLoopDetector::with_config(LoopDetectorConfig {
        ngram_sizes: Vec::new(),
        min_unique_ratio: 0.0,
        period_min_matches: 2.0,
        min_long_match_repeats: repeats,
        ..LoopDetectorConfig::default()
    })
}

proptest! {
    #[test]
    fn scenario_detection_is_invariant_under_arbitrary_subword_chunk_boundaries(
        sizes in prop::collection::vec(1usize..32, 1..64),
        looping in any::<bool>(),
    ) {
        // @step Given equivalent technical prose or repeating text containing paths punctuation and Unicode
        let mut text = technical_prose();
        text.push_str("Unicode Straße café Δοκιμή punctuation (::), file_name.rs:42! ");
        if looping {
            text.push_str(&"the model thinks that ".repeat(100));
        }
        let mut whole = StreamLoopDetector::new();
        let expected = whole.feed(&text);
        prop_assert_eq!(expected.is_some(), looping);
        let characters: Vec<char> = text.chars().collect();
        let mut chunked = StreamLoopDetector::new();
        let mut first = None;
        let mut offset = 0;
        let mut chunk_index = 0;

        // @step When the text is streamed whole or split at arbitrary character boundaries
        while offset < characters.len() {
            let end = (offset + sizes[chunk_index % sizes.len()]).min(characters.len());
            let chunk: String = characters[offset..end].iter().collect();
            let signal = chunked.feed(&chunk);
            first = first.or(signal);
            offset = end;
            chunk_index += 1;
        }

        // @step Then the first loop signal is identical for every chunking
        prop_assert_eq!(first, expected);
    }

    #[test]
    fn scenario_long_suffix_detection_honors_the_configured_earlier_copy_threshold(repeats in 1usize..6) {
        // @step Given a long suffix detector requiring between one and five earlier copies
        let mut detector = suffix_detector(repeats);
        let passage = (0..16).map(|index| format!("distinct{index} ")).collect::<String>();

        // @step When a distinct sixteen word passage is repeated up to the configured threshold
        for _ in 0..repeats {
            // @step Then no long suffix signal is reported before the trailing copy
            prop_assert!(detector.feed(&passage).is_none());
        }

        // @step When the trailing copy completes
        let signal = detector.feed(&passage);

        // @step Then a long suffix signal is reported
        prop_assert_eq!(signal, Some(LoopSignal::LongSuffixMatch { m: 16 }));
    }
}

#[test]
fn scenario_two_legitimate_pathname_references_are_not_repetition_collapse() {
    // @step Given technical prose referencing the same long pathname twice with distinct surrounding words
    let text = technical_prose();
    let mut detector = StreamLoopDetector::new();

    // @step When every character is streamed as a separate delta
    let signals: Vec<_> = text
        .chars()
        .filter_map(|character| detector.feed(&character.to_string()))
        .collect();

    // @step Then no loop signal is reported
    assert!(signals.is_empty(), "false positive: {signals:?}");
}

#[test]
fn scenario_overlapping_matches_do_not_inflate_long_suffix_evidence() {
    // @step Given a long suffix detector requiring three earlier sixteen word copies
    let mut detector = suffix_detector(3);

    // @step When only thirty two identical complete words are streamed
    let premature = detector.feed(&"yes ".repeat(32));

    // @step Then no long suffix signal is reported
    assert!(premature.is_none());

    // @step When thirty two more identical complete words are streamed
    let signal = detector.feed(&"yes ".repeat(32));

    // @step Then a long suffix signal is reported
    assert_eq!(signal, Some(LoopSignal::LongSuffixMatch { m: 16 }));
}

#[test]
fn scenario_reset_clears_partial_words_and_overlong_words_do_not_create_evidence() {
    // @step Given a detector receiving an unfinished word longer than its carry limit
    let mut detector = StreamLoopDetector::new();
    for _ in 0..10_000 {
        assert!(detector.feed("yes").is_none());
    }

    // @step When the word ends and fresh distinct prose follows
    let signal = detector.feed(&format!(" {}", technical_prose()));

    // @step Then no loop signal is reported
    assert!(signal.is_none());

    // @step When the detector is reset with an unfinished word pending
    assert!(detector.feed("unfinished").is_none());
    detector.reset();

    // @step And a genuine complete word loop is streamed
    let early = detector.feed(&"yes ".repeat(29));
    let signal = detector.feed("yes ");

    // @step Then the loop is detected at the normal threshold
    assert!(early.is_none());
    assert!(matches!(signal, Some(LoopSignal::NgramRepeat { .. })));
}
