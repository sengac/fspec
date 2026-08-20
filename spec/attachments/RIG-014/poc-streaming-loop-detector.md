# POC: Streaming Loop Detector — Standalone Proof

**Date:** 2026-08-19
**Location:** `/tmp/loop-investigation/poc` (Cargo binary, single `src/main.rs`)

This is the standalone mock/POC requested before the ACDD work unit. It is NOT part of the fspec codebase — it exists only to prove the 4-signal detection design works against synthetic streaming loops before we spec + implement it for real.

## How to run

```bash
cd /tmp/loop-investigation/poc
cargo run --quiet
```

## Design (word-level, online)

- Feed one text delta at a time; tokenize on whitespace, lowercase.
- Bounded word window (default 96 words).
- Four signals, in order of specificity:
  1. **Tail n-gram repetition** — last n words (n ∈ {3,5,8}) appear ≥ 4× in window.
  2. **Diversity collapse** — unique-word ratio < 0.28 (window ≥ 40).
  3. **Long verbatim suffix** — last 16+ words appear verbatim earlier in window.
  4. **Drift-tolerant periodicity** — last P=24 words match ≥ 85% of the P words before them (word-pair ratio). *This catches drifting loops that exact-match detectors miss.*
- Minimum-evidence guard: no checking before 12 words.

## Test scenarios (7 synthetic stream generators)

| Scenario | Expect | Result | Triggered at (word) | Signal |
|---|---|---|---|---|
| normal prose | no | PASS | — | — |
| mild one-off repeat | no | PASS | — | — |
| n-gram lock-in | yes | PASS | 43 | NgramRepeat{n:3, count:4} |
| single-token spam | yes | PASS | 20 | NgramRepeat{n:3, count:4} |
| verbatim block loop | yes | PASS | 39 | LongSuffixMatch{m:16} |
| drifting block loop | yes | PASS | 47 | Periodic{sim:0.917} |
| structured list (legit) | no | PASS | — | — |

## Key findings

- **Detection latency:** loops caught within ~15–35 words of onset — far before a thinking budget is exhausted.
- **Zero false positives** on normal prose, mild one-off repetition, and legitimate repeated structure (numbered step lists).
- **Drifting loops** (2 words/cycle changed) — the case loop-guard's exact-match design misses — caught by the periodicity signal at 91.7% similarity.

## Full source

See `poc-source.rs` (verbatim copy of `/tmp/loop-investigation/poc/src/main.rs`).
