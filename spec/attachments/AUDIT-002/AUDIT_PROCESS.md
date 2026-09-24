# Laya-Powered Coverage-Mapping Audit — Process Document

Audit of the fspec codebase (2026-09-22) that classifies every
`spec/features/*.feature.coverage` scenario→test→impl mapping as
**aligned / divergent / unverifiable**, combining a deterministic structural
pass with Laya (laya-rs) as the semantic oracle.

This document records the exact process so it can be turned into a first-class
`fspec` capability (see AUDIT-001) and re-run at any time.

---

## 1. Prerequisites — laya setup

```bash
git clone https://github.com/apiplant/laya-rs.git /tmp/laya-rs
cd /tmp/laya-rs && cargo build --release
cp target/release/laya ~/.local/bin/laya
# First run downloads the checkpoint (~843 MB) into ~/.cache/laya-rs/laya-typed-decisions
laya "We were billed twice for March."
```

### How laya works (facts that shaped this audit)

- **Process model**: every `laya` invocation is a fresh process. The model
  (843 MB weights, mmap'd) is reloaded per invocation. `laya answer` loads it
  **once** and answers a whole `{"state", "questions"}` file in **one batched
  forward pass** — so the cost model is: minimize invocation count, maximize
  questions per invocation.
- **Inference speed (CPU, aarch64, no CUDA)**: ~12 s for 10 questions,
  ~90 s for 60 questions (≈1.5 s/question amortized; model load dominates for
  small batches). RAM per process ≈ 2.5 GB.
- **Sequence layout** (`src/schema.rs::build_sequence`, `max_len=1024`,
  `head_max_len=256`): `[CLS] <type> question: <instructions> [SEP] [MASK]opt0
  [MASK]opt1 ... [SEP] <state> [SEP]`. Budget is allocated options-first;
  **the state is placed last and its tail is truncated** on overflow.
- **State strings are passed through verbatim** (`Value::String` → raw text),
  while JSON objects are serialized with keys sorted alphabetically. ⇒ to
  control ordering, the state must be a **plain string**, with the most
  important content (scenario name) first and the least important (long code)
  later, so any truncation eats the tail.
- **Option text** is truncated to 48 tokens each and eats the
  `head_max_len=256` budget together with the instructions — keep option
  criteria short (≤ ~8 words) so the instructions survive.
- **Confidence semantics**: laya reports low absolute confidence even on
  correct calls (observed mean ≈ 0.025 aligned / 0.006 divergent). Treat the
  *choice* as a prioritization signal, not a verdict; the probability spread
  (argmax − runner-up) is a useful tiebreaker.

---

## 2. Pass 1 — Deterministic structural audit (`structural_audit.py`)

Walks every `spec/features/*.feature.coverage` and validates each mapping:

| Check | Kind emitted | Meaning |
|---|---|---|
| impl file exists | `impl_file_missing` | path is dead (e.g. TS→Rust migration removed `src/`) |
| test file exists | `test_file_missing` | same, for test mappings |
| impl line ≤ file length | `impl_lines_beyond_eof` | line numbers point past EOF (file shrank) |
| test line ≤ file length | `test_lines_beyond_eof` | same, for test ranges |
| impl lines non-empty | `impl_lines_empty` | mapping exists but carries no lines |
| scenario has a testMapping | `scenarios_no_test_mapping` | spec-only, never linked |
| testMapping has implMappings | `no_impl_mapping` | test linked, impl not |
| impl block > 300 lines | `impl_suspiciously_large` | "whole-file" blob mapping |
| impl lines scattered (<30% density across span) | `impl_scattered` | suspicious multi-block mapping |

Handles both impl-line formats (explicit line arrays and `"a-b,c-d"` range
strings, including comma-separated ranges).

**Output**: `problems.json` = `{summary: {counts, top files}, problems: [8999 records]}`.

Cost: ~1 s over 1,758 coverage files / 11,877 scenarios.

---

## 3. Pass 2 — Laya semantic audit

### 3a. Dataset builder (`build_dataset.py`)

For every scenario whose **first impl mapping still points at an existing
file**, emit one record (JSONL):

```json
{"qid": "<feature>::<scenario>", "feature": "...", "scenario": "...",
 "state": "SCENARIO \"...\" (feature ...): TEST t.rs:66-101; IMPL f.rs (73 mapped lines) among ...\nCODE UNDER REVIEW:\n// f.rs:996-1074\n<code>",
 "code": "..."}
```

Key design decisions (each was discovered empirically):

1. **Code excerpt goes into the STATE string, not the question
   instructions** — the instructions/head budget is only ~250 tokens; the
   state gets the remaining ~750–950.
2. **Metadata first, code second** — laya truncates the *tail* of the
   sequence, so the scenario name (which is what disambiguates scenarios that
   map the same file) survives; long excerpts get clipped.
3. **Excerpt capped at 2,300 chars** for the first impl mapping (collapsed
   ranges with ±3-line padding, `// file:from-to` headers, tail truncation
   marker). This keeps metadata + code inside the 1024-token window for most
   records.
4. **Only the first impl mapping gets code** — extra mappings are listed in
   the metadata line so the model knows more files exist.
5. **Excluded from the dataset** (already decided by Pass 1): missing impl
   files, empty mappings, no test mapping.

Dataset size observed: **7,543 records** (4,843 scenarios eliminated by the
structural pass first).

### 3b. Question template (`run_laya_audit.py`)

Each record becomes one `choice` question with three options:

- `aligned` — the mapped code visibly implements the scenario's behavior
- `divergent` — related code exists but does NOT do what the scenario says
- `unverifiable` — can't decide from this excerpt alone

The question template (short, ~90 tokens) primes the model to look for the
scenario's core action inside the mapped lines and to call wrong/missing
behavior `divergent`.

### 3c. Batched parallel driver (`run_laya_audit.py`)

- **Batching**: 60 questions per `laya answer` invocation (one model load,
  one forward pass per batch). 7,543 records → 126 batches.
- **Parallelism**: 3 concurrent processes under a **10 GB RAM cap**
  (each process ≈ 2.5 GB; `LAYA_PARALLEL` env var).
- **Resumable**: each batch's `out.json` is cached in
  `batch_NNNN/out.json`; re-running the driver skips completed batches
  (crash/restart friendly — the driver was restarted mid-run once with zero
  data loss).
- **Invocation**:
  `laya answer batch/in.json batch/out.json --model ~/.cache/laya-rs/laya-typed-decisions`
  (note: `--model` is the current flag; older scripts use `--model-dir`.)
- **Result**: 126/126 batches, 7,542 answers, 0 failed batches, ~75 min
  wall-clock on CPU.

### 3d. Report (`report.py`)

- Rebuilds the qid→(feature, scenario, impl file) join **from each batch's
  `in.json`** (source of truth) — do not join by scanning records.jsonl,
  since identical `feature::scenario` names can appear in two different
  coverage files.
- Aggregates: totals, top features/files by divergent count (min 3 judged),
  confidence calibration.
- Outputs: `report.json` (aggregates) and `per_qid.json` (all verdicts with
  feature/scenario/impl-file + probabilities).

---

## 4. Validation performed

- **Format verification**: 10- and 12-question smoke runs before the full
  run. The first smoke run exposed a critical bug in *our* prompt layout:
  with scenario names at the tail, they were truncated off in the 1024-token
  window, making same-file scenarios produce **identical** probability
  vectors. Reordered metadata-first (see 3a.2) and re-verified: distinct
  scenarios now produce distinct distributions.
- **Hand verification of verdicts**: `delete-scenario-rust-port` (5/5
  divergent) — confirmed every scenario maps lines 1–282 of a 325-line file
  (whole-file blob); `modellimitsresolver-trait-provider-veto-authority`
  (8/8 divergent) — maps lines 1–119 of a 1,140-line file. Both correct.
- **Signal check**: 42% of divergent verdicts map ≥100 lines vs 16% of
  aligned (median 78 vs 32 lines) — the model reliably flags whole-file blob
  mappings.

## 5. Known limitations

1. Laya confidence is low in absolute terms → use as a **prioritized
   review queue**, not ground truth. The structural pass is the hard fact
   layer.
2. ~1 in 5,700 scenarios is excluded by qid collision (same
   `feature::scenario` name in two coverage files) — negligible.
3. Excerpt truncation can produce false `unverifiable`/`divergent` when the
   decisive logic is outside the 2,300-char window.
4. Laya judges one impl file at a time (the first mapping); multi-file
   scenarios may be under-evaluated.
5. Laya is CPU here; on a CUDA machine the whole semantic pass drops to
   minutes.

## 6. Re-running end to end

```bash
# Pass 1 (fast, always re-run)
python3 structural_audit.py          # -> problems.json

# Pass 2 (rebuild dataset, then run; resumable)
python3 build_dataset.py             # -> records.jsonl
LAYA_BATCH=60 LAYA_PARALLEL=3 python3 run_laya_audit.py   # -> laya_results.json
python3 report.py                    # -> report.json, per_qid.json
```

## 7. Artifacts (this audit, 2026-09-22)

| File | Size | Contents |
|---|---|---|
| `report.json` | 36 K | aggregates: totals, top features/files, structural counts |
| `problems.json` | 2.1 M | all 8,999 structural defect records |
| `per_qid.json` | 3.5 M | all 7,542 laya verdicts (choice, confidence, probs, feature, scenario, impl file) |
| `laya_results.json` | 2.4 M | raw laya output keyed by qid |
| `qmap.json` | 2.1 M | qid → feature/scenario/impl (rebuilt from batch inputs) |
| `records.jsonl` | 22 M | full dataset incl. code excerpts (regenerable) |
| `structural_audit.py`, `build_dataset.py`, `run_laya_audit.py`, `report.py` | 28 K | the four pipeline scripts |

(Large files are stored gzip-compressed in the card attachments; decompress
with `gunzip -c file.json.gz > file.json`.)
