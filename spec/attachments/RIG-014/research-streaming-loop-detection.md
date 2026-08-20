# Research: LLM Streaming Loop Detection

**Date:** 2026-08-19
**Purpose:** Investigate algorithms and prior art for detecting LLM output loops in real time (during streaming), with particular focus on thinking/reasoning token loops.

---

## 1. Problem Definition

LLMs can enter **repetition collapse**: once a phrase is repeated, the context makes the next copy *more* likely (self-reinforcement under greedy/constrained decoding). The stream then degenerates into an n-gram loop, single-token spam, or verbatim paragraph repetition until `max_tokens` is exhausted.

Key constraint for this project: **detection must be streaming (online)** — the loop happens *while* tokens are being generated. A post-hoc check on completed text is too late: the model has already burned the token budget, and the user has already watched minutes of garbage. The detector must consume deltas one at a time and fire early.

### Failure modes observed (from literature + field reports)

| Pattern | Example | Detection signal |
|---|---|---|
| Short n-gram lock-in | `the model thinks that the model thinks that...` | tail n-gram repetition |
| Single-token spam | `yes yes yes yes yes...` | diversity collapse |
| Verbatim block/paragraph loop | same 20–100 word block repeated | long suffix match |
| Drifting loop | block repeated with 1–2 words changing per cycle | drift-tolerant periodicity |
| Tool-call loop (agent level) | same tool + same args repeated | normalized-args hash (out of scope for streaming text, but see §4) |

---

## 2. Academic Sources

### 2.1 "Solving LLM Repetition Problem in Production: A Comprehensive Study of Multiple Solutions" (arXiv 2512.04419)

**Authors:** Weiwei Wang, Weijie Zou, Jiyong Min (Shenzhen Sunline Tech). PDF attached: `llm-repetition-production.pdf`.

Key findings:

- **Three distinct repetition patterns** in production batch code-interpretation: business-rule generation repetition, method-call-relationship analysis repetition, and PlantUML syntax generation repetition.
- **Root cause (theoretical):** Markov-model analysis shows greedy decoding cannot escape repetitive loops; the self-reinforcement effect means a repeated phrase increases the probability of another copy.
- **Production impact:** 75–80% reproducibility rate across deployment modes; processing time degraded 43%–471% (28 min → 40–160 min in their batch scenario).
- **Their solutions are model-side** (not applicable to us as an API client, but confirm the failure mode is fundamental):
  1. Beam search with `early_stopping=True` — universal post-hoc fix, but addresses symptoms.
  2. `presence_penalty` — effective for one of their three bad cases.
  3. DPO fine-tuning — universal model-level fix.
- **Implication for us:** since we consume third-party APIs (Anthropic, OpenAI, Gemini, etc.), we cannot fix decoding. The only viable layer is **client-side streaming detection + early abort**. This paper validates that client-side detection is the correct architectural choice for API consumers.

### 2.2 "LoopLLM: Transferable Energy-Latency Attacks in LLMs via Repetitive Prompts" (arXiv 2511.07876)

PDF attached: `loopllm-energy-latency.pdf`.

Relevant angle: repetition loops are not just a quality problem — they are an **availability/DoS vector**. Repetitive outputs make inference cost scale with output length (sequential generation), so a looping model wastes compute and latency. This reinforces the value of *early* detection: every token generated inside a loop is wasted cost.

### 2.3 Related work cited by 2512.04419

- "Learning to break the Loop: Analyzing and Mitigating Repetitions for Natural Text Generation" — establishes repetition as fundamentally tied to decoding strategy.
- Repetition penalty / length normalization / diversity-promoting methods — all model-side.

### 2.4 Sebastian Raschka, "Why LLMs get stuck in repetition loops" (sebastianraschka.com)

Practical explanation: every generated token becomes context for the next prediction; once a phrase repeats, the new context assigns even more probability to another copy. Greedy or highly constrained decoding then keeps selecting the same local continuation. Confirms the loop is **self-sustaining once started** — i.e., the window at loop-onset is short, and a streaming detector with a modest window catches it quickly.

---

## 3. Prior Art: Open-Source Streaming Detectors

### 3.1 loop-guard (github.com/Joshuaakaspace/loop-gaurd)

~255 lines of Python. The closest prior art to what we need. **Online, token-at-a-time** detector with three signals:

1. **Tail n-gram repetition** — the last n tokens (n ∈ {3, 5, 8}) already appear ≥ 4 times inside a 96-token window.
2. **Window diversity collapse** — fraction of distinct tokens in the window drops below 0.28 (only checked once window ≥ 40 tokens).
3. **Long verbatim suffix match** — the last ≥ 16 tokens appear verbatim earlier in the window.

Plus a **minimum-evidence guard** (no checking before 12 tokens) to avoid tripping on stream start. Their test corpus defines the canonical failure modes: normal prose, mild one-off phrase repetition (false-positive resistance), severe n-gram lock-in, single-token spam, long verbatim block repetition.

**Limitation:** works on raw tokens; we receive text deltas, so we adapt to word-level (see §5).

### 3.2 agent-loop-detector (github.com/KorahStone/agent-loop-detector)

Turn-level (not streaming): compares whole outputs using Jaccard / cosine / Levenshtein; 3 consecutive outputs ≥ 0.85 similar → loop. Useful only as a coarse second layer between turns; too slow to catch mid-stream collapse.

---

## 4. Prior Art: Agent Harnesses

### 4.1 vtcode (github.com/vinhnx/vtcode)

Has a sophisticated **tool-call** `LoopDetector` (`crates/codegen/vtcode-core/src/core/loop_detector/mod.rs`, ~1,700 lines) but **no streaming text loop detection** (its `stream_sanitization.rs` only strips provider noise tokens).

Stealable architecture:

- **Multi-signal detector** with ordered precedence: identical-call hard stop → repetitive read-target → navigation streak (warn → hard stop) → oscillation pattern (A→B→A→B) → sliding-window dedup.
- **Soft-warn → hard-stop escalation** with a 30-second warning cooldown.
- **Normalized-args hashing** before comparison: nulls write-tool content, canonicalizes shell commands, extracts read targets. Their documented bug: paginated reads (same path, different `offset`/`limit`) collapsed into one family key and deadlocked the agent — fixed by adding `off=N::lim=N::raw=bool` suffixes to the family key.
- **Recovery policy:** on hard stop, inject a "synthesize a final answer NOW using data already in your conversation history" instruction rather than silently aborting.

### 4.2 opencode (github.com/sst/opencode)

**No streaming loop detection at all.** Defenses are prompt-level ("avoid unnecessary repetition and verbosity") and agent step limits. Confirms this is a gap in the current ecosystem.

---

## 5. Proposed Detection System (Design)

### 5.1 Word-level online detector

We consume text deltas (not raw tokens), so we tokenize on whitespace and lowercase. A `StreamLoopDetector` is fed one delta at a time and maintains a bounded word window (default 96 words ≈ a few hundred tokens of thinking text).

**Four signals, evaluated in order of specificity:**

| # | Signal | Catches | Default thresholds |
|---|---|---|---|
| 1 | **Tail n-gram repetition** — last n words (n ∈ {3, 5, 8}) appear ≥ 4× in window | short lock-in, token spam | window 96, max_repeats 4 |
| 2 | **Diversity collapse** — unique-word ratio < 0.28 (window ≥ 40 words) | degenerate spam | min_unique_ratio 0.28 |
| 3 | **Long verbatim suffix** — last 16+ words appear verbatim earlier in window | paragraph loops | min_long_match 16 |
| 4 | **Drift-tolerant periodicity** — last P words match ≥ 85% of the P words immediately before them (word-pair match ratio) | drifting loops (1–2 words change per cycle) | P = 24, min_match 0.85 |

Signal 4 is our addition over loop-guard: real thinking loops often drift slightly each cycle (the model "retries" the same reasoning with a changed word), which escapes exact-match checks.

**Minimum-evidence guard:** no checking before 12 words in the window.

**Per-channel state:** one detector instance per (turn, channel) — thinking and text are independent streams and must not share windows (a thinking loop must not be masked by fresh text, and vice versa).

### 5.2 Escalation policy (borrowed from vtcode)

1. **First trigger → warn:** emit a status event, keep streaming. Record trigger time.
2. **Re-trigger within cooldown (30s) OR a second signal type fires → abort:** cancel the provider stream, truncate the looping tail from the persisted assistant content, and on the next turn inject a corrective system note ("Your previous response was cut off due to repetitive output. Continue with a fresh approach; do not repeat your earlier reasoning.").
3. **Persistent (2nd abort in same session) → surface to user** via a notification with the offending tail excerpt.

### 5.3 Complexity

All four signals are O(window) per word in the worst case; window is bounded (96), so worst case is ~400 word comparisons per delta word — negligible next to network/tokenization cost. Fast path: signals 1/3/4 only run when the window is large enough; signal 2 only when window ≥ 40.

### 5.4 POC validation (2026-08-19)

A standalone Rust POC (`/tmp/loop-investigation/poc`) implementing the 4-signal detector was run against 7 synthetic stream generators:

```
normal prose                 expect_trigger=false got=false  PASS
mild one-off repeat          expect_trigger=false got=false  PASS
n-gram lock-in               expect_trigger=true  got=true   at_word=43  NgramRepeat{n:3, count:4}   PASS
single-token spam            expect_trigger=true  got=true   at_word=20  NgramRepeat{n:3, count:4}   PASS
verbatim block loop          expect_trigger=true  got=true   at_word=39  LongSuffixMatch{m:16}       PASS
drifting block loop          expect_trigger=true  got=true   at_word=47  Periodic{sim:0.917}         PASS
structured list (legit)      expect_trigger=false got=false  PASS
```

Key results:

- **Detection latency:** loops caught within ~15–35 words of loop onset (well before a 4096-token thinking budget is burned).
- **Zero false positives** on normal prose, mild one-off repetition, and legitimate repeated structure (numbered step lists).
- **Drifting loops** (2 words/cycle changed) — the case loop-guard's exact-match design would miss — caught by the periodicity signal at 91.7% similarity.

---

## 6. Open Questions

1. **Threshold tuning per model:** adaptive-thinking models (Opus 4.7) may legitimately repeat phrasing inside extended thinking. May need per-provider threshold profiles.
2. **Should detection run on tool-call argument text too?** (e.g., a model streaming a giant repeated string into a Write tool's content arg). Deferred — tool-call loop detection is a separate story (vtcode-style).
3. **Abort granularity:** does the rig-core stream support cancellation mid-delta? (Likely yes via tokio task abort; verify in implementation.)
