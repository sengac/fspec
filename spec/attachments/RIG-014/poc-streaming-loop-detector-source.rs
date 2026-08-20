//! POC: Streaming LLM loop detector (word-level, online).
//!
//! Signals:
//!  1. Tail n-gram repetition: the last n words already appear >= max_repeats
//!     times in the window.
//!  2. Diversity collapse: unique-word ratio in window < threshold.
//!  3. Long verbatim suffix match: last m words appear verbatim earlier.
//!  4. Periodicity (drift-tolerant): the window's recent half is a near-repeat
//!     of the half before it (catches loops that drift slightly per cycle).

use std::collections::VecDeque;

#[derive(Clone)]
struct Config {
    window: usize,
    ngram_sizes: Vec<usize>,
    max_repeats: usize,
    min_unique_ratio: f64,
    diversity_min_window: usize,
    min_long_match: usize,
    min_words_before_check: usize,
    // periodicity: compare last P words against the P words before them,
    // requiring >= sim threshold of word-pair matches
    period_len: usize,
    period_min_matches: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            window: 96,
            ngram_sizes: vec![3, 5, 8],
            max_repeats: 4,
            min_unique_ratio: 0.28,
            diversity_min_window: 40,
            min_long_match: 16,
            min_words_before_check: 12,
            period_len: 24,
            period_min_matches: 0.85,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
enum Signal {
    None,
    NgramRepeat { n: usize, count: usize },
    LowDiversity { ratio: f64 },
    LongSuffixMatch { m: usize },
    Periodic { sim: f64 },
}

struct Detector {
    cfg: Config,
    words: VecDeque<String>,
    triggered: bool,
    reason: Option<Signal>,
}

fn tokenize(delta: &str) -> Vec<String> {
    delta.split_whitespace().map(|w| w.to_lowercase()).collect()
}

impl Detector {
    fn new(cfg: Config) -> Self {
        Self { cfg, words: VecDeque::new(), triggered: false, reason: None }
    }

    fn feed(&mut self, delta: &str) -> Signal {
        if self.triggered {
            return self.reason.clone().unwrap_or(Signal::None);
        }
        for w in tokenize(delta) {
            self.words.push_back(w);
            if self.words.len() > self.cfg.window {
                self.words.pop_front();
            }
            if self.words.len() < self.cfg.min_words_before_check {
                continue;
            }
            let window: Vec<&str> = self.words.iter().map(|s| s.as_str()).collect();
            if let Some(sig) = self.check(&window) {
                self.triggered = true;
                self.reason = Some(sig.clone());
                return sig;
            }
        }
        self.reason.clone().unwrap_or(Signal::None)
    }

    fn check(&self, window: &[&str]) -> Option<Signal> {
        // 1. tail n-gram
        for &n in &self.cfg.ngram_sizes {
            if window.len() < n * self.cfg.max_repeats {
                continue;
            }
            let tail: Vec<&str> = window[window.len() - n..].to_vec();
            let mut count = 0;
            for i in 0..=window.len() - n {
                if window[i..i + n] == tail {
                    count += 1;
                }
            }
            if count >= self.cfg.max_repeats {
                return Some(Signal::NgramRepeat { n, count });
            }
        }
        // 2. diversity
        if window.len() >= self.cfg.diversity_min_window {
            let unique = window.iter().collect::<std::collections::HashSet<_>>().len();
            let ratio = unique as f64 / window.len() as f64;
            if ratio < self.cfg.min_unique_ratio {
                return Some(Signal::LowDiversity { ratio });
            }
        }
        // 3. long suffix
        let m = self.cfg.min_long_match;
        if window.len() >= 2 * m {
            let suffix = &window[window.len() - m..];
            for i in 0..=window.len() - 2 * m {
                if &window[i..i + m] == suffix {
                    return Some(Signal::LongSuffixMatch { m });
                }
            }
        }
        // 4. periodicity (drift-tolerant)
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
                return Some(Signal::Periodic { sim });
            }
        }
        None
    }
}

// ---------- synthetic stream generators ----------

fn rand_word(rng: &mut impl Rng) -> String {
    const V: &[&str] = &[
        "the", "model", "approach", "consider", "architecture", "function", "test",
        "module", "stream", "token", "buffer", "window", "signal", "detect", "loop",
        "phrase", "content", "provider", "delta", "chunk", "state", "history",
        "pattern", "sequence", "repetition", "collapse", "diversity", "threshold",
        "analysis", "implementation", "boundary", "condition", "variable", "output",
    ];
    V[rng.next() % V.len()].to_string()
}

trait Rng {
    fn next(&mut self) -> usize;
}
struct Lcg(usize);
impl Rng for Lcg {
    fn next(&mut self) -> usize {
        self.0 = self.0.wrapping_mul(6364136223).wrapping_add(1442695041);
        (self.0 >> 16) as usize
    }
}

fn normal_stream(n: usize, seed: usize) -> Vec<String> {
    let mut rng = Lcg(seed);
    (0..n).map(|_| rand_word(&mut rng)).collect()
}

fn mild_repeat_stream(n: usize, seed: usize) -> Vec<String> {
    let mut v = normal_stream(n, seed);
    // insert one short phrase repeated twice mid-stream
    let mut rng = Lcg(seed + 1);
    let phrase: Vec<String> = (0..3).map(|_| rand_word(&mut rng)).collect();
    let mut out = v[..40].to_vec();
    out.extend(phrase.iter().cloned());
    out.extend(phrase.iter().cloned());
    out.extend(v[40..].iter().cloned());
    out
}

fn ngram_loop_stream(n: usize, seed: usize) -> Vec<String> {
    let mut v = normal_stream(30, seed);
    let loop_words = vec!["the", "model", "thinks", "that", "the", "model", "thinks"];
    let mut i = 0;
    while v.len() < n {
        v.push(loop_words[i % loop_words.len()].to_string());
        i += 1;
    }
    v
}

fn token_spam_stream(n: usize, seed: usize) -> Vec<String> {
    let mut v = normal_stream(15, seed);
    v.extend(std::iter::repeat("yes".to_string()).take(n - 15));
    v
}

fn verbatim_block_stream(n: usize, seed: usize) -> Vec<String> {
    let mut rng = Lcg(seed);
    let block: Vec<String> = (0..24).map(|_| rand_word(&mut rng)).collect();
    let mut v = Vec::new();
    while v.len() < n {
        v.extend(block.iter().cloned());
    }
    v.truncate(n);
    v
}

fn drifting_loop_stream(n: usize, seed: usize) -> Vec<String> {
    // 24-word block, but 2 words drift (change) each cycle
    let mut rng = Lcg(seed);
    let mut block: Vec<String> = (0..24).map(|_| rand_word(&mut rng)).collect();
    let mut v = Vec::new();
    while v.len() < n {
        v.extend(block.iter().cloned());
        // drift: replace 2 positions with fresh words
        for pos in [5, 17] {
            block[pos] = rand_word(&mut rng);
        }
    }
    v.truncate(n);
    v
}

fn structured_list_stream(n: usize, seed: usize) -> Vec<String> {
    // "Step 1: <words> Step 2: <words> ..." — legitimate repeated structure
    let mut rng = Lcg(seed);
    let mut v = Vec::new();
    let mut step = 1;
    while v.len() < n {
        v.push(format!("step"));
        v.push(format!("{}", step));
        for _ in 0..5 {
            v.push(rand_word(&mut rng));
        }
        step += 1;
    }
    v
}

fn run(name: &str, words: Vec<String>, expect_trigger: bool) {
    let mut det = Detector::new(Config::default());
    let mut trigger_idx: Option<usize> = None;
    for (i, w) in words.iter().enumerate() {
        let sig = det.feed(w);
        if det.triggered {
            trigger_idx = Some(i);
            break;
        }
        let _ = sig;
    }
    let ok = det.triggered == expect_trigger;
    println!(
        "{:<28} expect_trigger={:<5} got={:<5} at_word={:<5} {:?}  {}",
        name,
        expect_trigger,
        det.triggered,
        trigger_idx.map(|i| i.to_string()).unwrap_or_else(|| "-".into()),
        det.reason,
        if ok { "PASS" } else { "FAIL" }
    );
    if !ok {
        std::process::exit(1);
    }
}

fn main() {
    println!("=== POC streaming loop detector ===\n");
    run("normal prose", normal_stream(400, 1), false);
    run("mild one-off repeat", mild_repeat_stream(400, 2), false);
    run("n-gram lock-in", ngram_loop_stream(400, 3), true);
    run("single-token spam", token_spam_stream(400, 4), true);
    run("verbatim block loop", verbatim_block_stream(400, 5), true);
    run("drifting block loop", drifting_loop_stream(400, 6), true);
    run("structured list (legit)", structured_list_stream(400, 7), false);
    println!("\nAll scenarios behaved as expected.");
}
