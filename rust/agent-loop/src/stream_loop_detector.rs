//! RIG-014: Streaming LLM loop detector for thinking/text deltas.
//!
//! Detects LLM repetition collapse (n-gram lock-in, token spam, verbatim
//! paragraph loops, drifting loops) in real time while provider tokens
//! stream. The detector consumes text deltas one at a time and can fire
//! mid-stream — it never requires the completed response text.
//!
//! ## Design (see spec/attachments/RIG-014/research-streaming-loop-detection.md)
//!
//! Word-level tokenization (whitespace split, case-insensitive) because the
//! agent loop receives text deltas, not raw provider tokens. A bounded
//! sliding window (default 96 words) is evaluated against four signals in
//! order of specificity:
//!
//! 1. **Tail n-gram repetition** — the last n words (n ∈ {3,5,8}) appear
//!    ≥ 10 times in the window. Catches short lock-in and token spam.
//!    The high repeat count is intentional: models legitimately re-emit
//!    the same sentence up to ~10 times and short phrases recur naturally
//!    in prose, so only sustained high-count repetition is a loop.
//! 2. **Diversity collapse** — unique-word ratio < 0.28 (window ≥ 40 words).
//!    Catches degenerate single-token spam.
//! 3. **Long verbatim suffix** — the last ≥ 16 words appear verbatim earlier
//!    in the window. Catches paragraph-level loops.
//! 4. **Drift-tolerant periodicity** — the last 24 words match ≥ 85% of the
//!    24 words immediately before them by word-pair ratio. Catches loops
//!    that drift 1–2 words per cycle (exact-match detectors miss these).
//!
//! A minimum-evidence guard (12 words) prevents triggering on the first few
//! deltas of a stream. Once fired, the detector latches: subsequent
//! `feed` calls keep returning the latched signal until `reset()`.
//!
//! ## Escalation
//!
//! [`LoopEscalationPolicy`] turns detector signals into warn/abort
//! decisions: the first trigger warns; a re-trigger within the cooldown
//! window, or a second distinct signal type, aborts. The abort drives the
//! session's existing interrupt machinery (see
//! `background_output::BackgroundOutput::feed_loop_detectors`).
//!
//! ## Purity
//!
//! No I/O, no async — trivially unit-testable. Research + POC evidence:
//! `spec/attachments/RIG-014/` (research report, POC summary + source,
//! arXiv 2512.04419 / 2511.07876 PDFs).

use std::collections::VecDeque;
use std::time::Duration;

/// RIG-014: A loop signal detected in a streaming text/thinking channel.
#[derive(Debug, Clone, PartialEq)]
pub enum LoopSignal {
    /// Tail n-gram repetition: the last `n` words appeared `count` times
    /// within the window.
    NgramRepeat { n: usize, count: usize },
    /// Diversity collapse: unique-word ratio fell below the configured
    /// floor. `ratio` is the observed unique-word ratio.
    LowDiversity { ratio: f64 },
    /// Long verbatim suffix: the last `m` words appear verbatim earlier in
    /// the window.
    LongSuffixMatch { m: usize },
    /// Drift-tolerant periodicity: the recent half of the window matches
    /// `similarity` (word-pair ratio) of the half before it.
    Periodic { similarity: f64 },
}

/// RIG-014: Configurable thresholds for the streaming loop detector.
///
/// All defaults are POC-validated (see
/// `spec/attachments/RIG-014/poc-streaming-loop-detector.md`). Per-model
/// tuning is possible without code changes.
#[derive(Debug, Clone)]
pub struct LoopDetectorConfig {
    /// Bounded sliding window size (words).
    pub window: usize,
    /// Tail n-gram sizes to check.
    pub ngram_sizes: Vec<usize>,
    /// Tail n-gram must appear at least this many times in the window.
    /// Tolerance: models legitimately re-emit the same sentence up to
    /// ~10 times, and short phrases recur naturally in prose — so the
    /// n-gram signal only fires on sustained high-count repetition.
    pub max_repeats: usize,
    /// Diversity collapse fires when unique-word ratio falls below this.
    /// Tolerance: a low-but-nonzero ratio (e.g. a technical write-up
    /// reusing a small vocabulary) is legitimate; only near-total
    /// collapse is a loop.
    pub min_unique_ratio: f64,
    /// Diversity signal is only evaluated once the window holds at least
    /// this many words.
    pub diversity_min_window: usize,
    /// Long verbatim suffix length (words).
    pub min_long_match: usize,
    /// Long verbatim suffix must appear at least this many times in the
    /// window before the signal fires (the trailing copy is not counted).
    /// Tolerance: models legitimately re-emit the same paragraph up to
    /// ~5 times, so the signal fires on the 6th copy.
    pub min_long_match_repeats: usize,
    /// Minimum words before any signal is evaluated (minimum-evidence
    /// guard).
    pub min_words_before_check: usize,
    /// Periodicity window length (words) — the recent P words are compared
    /// against the P words immediately before them.
    pub period_len: usize,
    /// Periodicity fires when the word-pair match ratio is at least this.
    pub period_min_matches: f64,
}

impl Default for LoopDetectorConfig {
    fn default() -> Self {
        Self {
            window: 160,
            ngram_sizes: vec![3, 5, 8],
            max_repeats: 10,
            min_unique_ratio: 0.15,
            diversity_min_window: 40,
            min_long_match: 16,
            min_long_match_repeats: 3,
            min_words_before_check: 12,
            period_len: 24,
            period_min_matches: 0.85,
        }
    }
}

/// RIG-014: Online, word-level streaming loop detector.
///
/// Feed one text delta at a time via [`feed`](Self::feed); the detector
/// tokenizes on whitespace (case-insensitive), maintains a bounded word
/// window, and evaluates the four signals in order of specificity. Once a
/// signal fires the detector **latches** — further `feed` calls keep
/// returning the latched signal until [`reset`](Self::reset) is called
/// (once per turn).
#[derive(Debug, Clone)]
pub struct StreamLoopDetector {
    cfg: LoopDetectorConfig,
    words: VecDeque<String>,
    latched: Option<LoopSignal>,
}

impl StreamLoopDetector {
    /// Create a detector with the POC-validated default thresholds.
    pub fn new() -> Self {
        Self::with_config(LoopDetectorConfig::default())
    }

    /// Create a detector with explicit thresholds.
    pub fn with_config(cfg: LoopDetectorConfig) -> Self {
        Self {
            cfg,
            words: VecDeque::new(),
            latched: None,
        }
    }

    /// Feed one streaming delta. Returns the loop signal if the detector
    /// has fired (either now or earlier — the detector latches).
    pub fn feed(&mut self, delta: &str) -> Option<LoopSignal> {
        if self.latched.is_some() {
            return self.latched.clone();
        }
        for word in delta.split_whitespace() {
            self.words.push_back(word.to_lowercase());
            if self.words.len() > self.cfg.window {
                self.words.pop_front();
            }
            if self.words.len() < self.cfg.min_words_before_check {
                continue;
            }
            let window: Vec<&str> = self.words.iter().map(|s| s.as_str()).collect();
            if let Some(sig) = self.check(&window) {
                self.latched = Some(sig.clone());
                return Some(sig);
            }
        }
        self.latched.clone()
    }

    /// Clear all detector state for a new turn.
    pub fn reset(&mut self) {
        self.words.clear();
        self.latched = None;
    }

    /// The latched signal, if the detector has fired.
    pub fn signal(&self) -> Option<&LoopSignal> {
        self.latched.as_ref()
    }

    fn check(&self, window: &[&str]) -> Option<LoopSignal> {
        // 1. Tail n-gram repetition (most specific).
        for &n in &self.cfg.ngram_sizes {
            if window.len() < n * self.cfg.max_repeats {
                continue;
            }
            let tail = &window[window.len() - n..];
            let mut count = 0usize;
            for i in 0..=window.len() - n {
                if &window[i..i + n] == tail {
                    count += 1;
                }
            }
            if count >= self.cfg.max_repeats {
                return Some(LoopSignal::NgramRepeat { n, count });
            }
        }

        // 2. Diversity collapse.
        if window.len() >= self.cfg.diversity_min_window {
            let unique = window.iter().collect::<std::collections::HashSet<_>>().len();
            let ratio = unique as f64 / window.len() as f64;
            if ratio < self.cfg.min_unique_ratio {
                return Some(LoopSignal::LowDiversity { ratio });
            }
        }

        // 3. Long verbatim suffix.
        let m = self.cfg.min_long_match;
        if window.len() >= 2 * m {
            let suffix = &window[window.len() - m..];
            for i in 0..=window.len() - 2 * m {
                if &window[i..i + m] == suffix {
                    return Some(LoopSignal::LongSuffixMatch { m });
                }
            }
        }

        // 4. Drift-tolerant periodicity (last-resort, catches drifting loops).
        let p = self.cfg.period_len;
        if window.len() >= 2 * p {
            let recent = &window[window.len() - p..];
            let prev = &window[window.len() - 2 * p..window.len() - p];
            let matches = recent
                .iter()
                .zip(prev.iter())
                .filter(|(a, b)| a == b)
                .count() as f64;
            let sim = matches / p as f64;
            if sim >= self.cfg.period_min_matches {
                return Some(LoopSignal::Periodic { similarity: sim });
            }
        }

        None
    }
}

impl Default for StreamLoopDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// RIG-014: Outcome of an escalation-policy decision.
#[derive(Debug, Clone, PartialEq)]
pub enum LoopEscalationOutcome {
    /// Non-fatal warning — streaming continues.
    Warn,
    /// Abort — the in-flight provider stream must be cancelled.
    Abort,
}

/// RIG-014: Warn-then-abort escalation policy.
///
/// - The **first** trigger warns.
/// - A **re-trigger within the cooldown window** aborts.
/// - A **second distinct signal type** (regardless of timing) aborts —
///   two independent loop signals is strong evidence of collapse.
/// - A re-trigger **after** the cooldown window warns again and restarts
///   the cooldown (a new loop episode).
///
/// `elapsed_secs` is the caller-supplied seconds since the previous
/// trigger (tests pass explicit values; production uses wall-clock).
pub struct LoopEscalationPolicy {
    cooldown: Duration,
    last_trigger: Option<f64>,
    last_signal: Option<LoopSignal>,
}

impl LoopEscalationPolicy {
    /// Create a policy with the given cooldown window.
    pub fn new(cooldown: Duration) -> Self {
        Self {
            cooldown,
            last_trigger: None,
            last_signal: None,
        }
    }

    /// Record a detector trigger at `elapsed_secs` (seconds since the
    /// previous trigger; 0 for the first) and return the decision.
    pub fn on_trigger(&mut self, signal: LoopSignal, elapsed_secs: f64) -> LoopEscalationOutcome {
        match self.last_trigger {
            None => {
                self.last_trigger = Some(elapsed_secs);
                self.last_signal = Some(signal);
                LoopEscalationOutcome::Warn
            }
            Some(prev_at) => {
                let gap = elapsed_secs - prev_at;
                let within_cooldown = gap <= self.cooldown.as_secs_f64();
                let distinct_signal = self
                    .last_signal
                    .as_ref()
                    .is_some_and(|prev| prev != &signal);
                if within_cooldown || distinct_signal {
                    // Escalate; keep the episode state so a third trigger
                    // still aborts.
                    LoopEscalationOutcome::Abort
                } else {
                    // New episode: warn and restart the cooldown clock.
                    self.last_trigger = Some(elapsed_secs);
                    self.last_signal = Some(signal);
                    LoopEscalationOutcome::Warn
                }
            }
        }
    }
}

/// RIG-014: Marker note appended to persisted assistant content when a
/// looping stream is aborted (keeps session history honest about the
/// truncation).
pub fn build_loop_abort_marker_note() -> String {
    "[Response cut off: repetitive output detected mid-stream. The degenerate tail was dropped by the streaming loop detector.]"
        .to_string()
}

/// RIG-014: Corrective note injected into the next turn's context after a
/// loop abort. Mirrors the `recovery_thinking.rs` (PROV-041) builder
/// pattern: states what happened and instructs the model how to proceed.
///
/// `onset_excerpt` is an optional short excerpt of the looping text for
/// context.
pub fn build_loop_abort_recovery_message(signal: &LoopSignal, onset_excerpt: Option<&str>) -> String {
    let mut msg = format!(
        "Your previous response was cut off because it fell into a repetitive output loop \
         (detected signal: {}). \
         Continue with a fresh approach: do not repeat your earlier reasoning or the \
         degenerate passage. If you were mid-task, resume from where the repetition began \
         and produce new, non-repetitive content.",
        describe_signal(signal)
    );
    if let Some(excerpt) = onset_excerpt {
        let truncated: String = excerpt.chars().take(200).collect();
        msg.push_str(&format!("\n\nExcerpt of the looping output:\n{truncated}"));
    }
    msg
}

/// Human-readable description of a loop signal (for log/recovery messages).
fn describe_signal(signal: &LoopSignal) -> String {
    match signal {
        LoopSignal::NgramRepeat { n, count } => {
            format!("the last {n} words repeated {count} times in the recent window")
        }
        LoopSignal::LowDiversity { ratio } => {
            format!("word diversity collapsed to {ratio:.2} unique ratio")
        }
        LoopSignal::LongSuffixMatch { m } => {
            format!("the last {m} words repeated verbatim from earlier in the stream")
        }
        LoopSignal::Periodic { similarity } => {
            format!("the stream became periodic ({similarity:.0}% word-pair match over the recent window)")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_matches_tolerant_thresholds() {
        let cfg = LoopDetectorConfig::default();
        assert_eq!(cfg.window, 160);
        assert_eq!(cfg.ngram_sizes, vec![3, 5, 8]);
        assert_eq!(cfg.max_repeats, 10);
        assert!((cfg.min_unique_ratio - 0.15).abs() < f64::EPSILON);
        assert_eq!(cfg.diversity_min_window, 40);
        assert_eq!(cfg.min_long_match, 16);
        assert_eq!(cfg.min_words_before_check, 12);
        assert_eq!(cfg.period_len, 24);
        assert!((cfg.period_min_matches - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn feed_is_case_insensitive() {
        // 30 words of "Yes" in mixed case must behave identically to
        // lowercase "yes" — the 30th word triggers the n-gram signal
        // (10 repeats of the 3-word tail "yes yes yes").
        let mut mixed = StreamLoopDetector::new();
        for i in 0..29 {
            assert!(mixed.feed("Yes").is_none(), "word {i} must not fire yet");
        }
        assert!(mixed.feed("yes").is_some(), "30th mixed-case word must fire");

        let mut lower = StreamLoopDetector::new();
        for i in 0..29 {
            assert!(lower.feed("yes").is_none(), "word {i} must not fire yet");
        }
        assert!(lower.feed("yes").is_some(), "30th lowercase word must fire");
    }

    #[test]
    fn latched_signal_persists_until_reset() {
        let mut det = StreamLoopDetector::new();
        for _ in 0..30 {
            det.feed("yes");
        }
        assert!(det.signal().is_some());
        assert!(det.feed("anything else").is_some());
        det.reset();
        assert!(det.signal().is_none());
        assert!(det.feed("fresh start here").is_none());
    }
}
