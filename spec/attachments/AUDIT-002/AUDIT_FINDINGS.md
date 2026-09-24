# Coverage-Mapping Audit — Findings (2026-09-22)

Full audit of fspec's `spec/features/*.feature.coverage` mappings using
deterministic structural checks + Laya (laya-rs) semantic classification.
Process: see `AUDIT_PROCESS.md`. Machine-readable data: `report.json`,
`problems.json`, `per_qid.json` (attachments to this card).

## Headline numbers

**Universe**: 1,758 coverage files · 11,877 scenarios · 1,620 impl files ·
1,107 test files referenced.

### A. Structural defects (deterministic, 100% reliable)

| Defect | Count | Root cause |
|---|---|---|
| impl file no longer exists | **3,730** | TS→Rust migration deleted `src/` and other TS trees |
| test file no longer exists | **3,495** | same migration (`src/`, `extension/`, `e2e/`) |
| impl lines beyond EOF | 761 | files shrank; line refs point past end of file |
| scenarios with no test mapping | 552 | spec-only, never linked |
| impl block > 300 lines | 345 | whole-file blob mappings |
| test mapping with no impl mapping | 297 | test linked, impl not |
| test lines beyond EOF | 293 | files shrank |
| scattered impl line sets | 78 | suspicious multi-block mappings |
| unparseable coverage files | 0 | — |

Missing-file records by top-level dir: `src` 5,865 · `rust` 953 ·
`extension` 327 · `spec` 45 · other 76.

### B. Laya semantic verdicts (7,542 mappings whose impl file still exists)

| Verdict | Count | % |
|---|---|---|
| aligned | 6,934 | 91.9% |
| **divergent** | **553** | **7.3%** |
| unverifiable | 55 | 0.7% |

Calibration signal: 42% of divergent verdicts map ≥100 lines (whole-file
blobs) vs 16% of aligned; median mapped lines 78 (divergent) vs 32
(aligned). Laya reliably flags blob mappings; hand-verified samples were
correct.

## Prioritized work list

### P0 — migrate-or-delete dead mappings (~7,225 defect records)

The TS→Rust migration orphaned most of the old coverage: 3,730 impl refs
and 3,495 test refs point at files that no longer exist. Two sub-tasks:

1. **Map the survivors**: for features where a Rust port exists
   (e.g. `show-coverage-rust-port` features map `rust/fspec-core/src/commands/*.rs`),
   re-link test + impl mappings to the new paths/lines.
2. **Retire the dead**: coverage files that are 100% dead (no live mapping
   at all) should either get fresh mappings or be archived. (See
   `problems.json` grouped by feature to decide per-file.)

### P1 — fix the divergent mappings (553; top offenders attached in report.json)

Features with the worst divergent ratios (min 3 judged):

| Feature | judged | div | ali |
|---|---|---|---|
| isolated-session-file-operations | 23 | 16 | 7 |
| show-coverage-rust-port | 27 | 14 | 13 |
| show-epic-rust-port | 16 | 13 | 3 |
| tag-stats-rust-port | 13 | 11 | 2 |
| query-orphans-rust-port | 10 | 9 | 1 |
| reverse-rust-port | 17 | 9 | 8 |
| answer-question-rust-port | 11 | 8 | 3 |
| modellimitsresolver-trait-provider-veto-authority | 8 | 8 | 0 |
| legacy-compaction-cleanup | 11 | 7 | 4 |
| query-metrics-rust-port | 8 | 7 | 1 |
| add-capability-rust-port | 8 | 6 | 2 |
| show-foundation-event-storm-rust-port | 8 | 6 | 2 |
| show-foundation-rust-port | 13 | 6 | 7 |
| delete-features-rust-port | 5 | 5 | 0 |
| delete-scenario-rust-port | 5 | 5 | 0 |

Files with the most divergent mappings:

| File | judged | div | ali |
|---|---|---|---|
| rust/tools/src/facade/wrapper.rs | 63 | 21 | 42 |
| rust/fspec-core/src/commands/show_coverage.rs | 26 | 14 | 12 |
| rust/fspec-core/src/commands/show_epic.rs | 16 | 13 | 3 |
| rust/fspec-core/src/commands/tag_stats.rs | 13 | 11 | 2 |
| rust/fspec-core/src/commands/query_orphans.rs | 10 | 9 | 1 |
| rust/fspec-core/src/commands/reverse.rs | 16 | 9 | 7 |
| rust/fspec-core/src/commands/answer_question.rs | 11 | 8 | 3 |
| rust/providers/src/model_limits.rs | 16 | 8 | 8 |
| rust/core/src/compaction/mod.rs | 9 | 7 | 2 |
| rust/fspec-core/src/commands/query_metrics.rs | 8 | 7 | 1 |

**Pattern** (verified by hand): most divergent verdicts are *whole-file blob
mappings* — every scenario of a feature points at the same giant line range
(e.g. `delete-scenario-rust-port`: 5/5 scenarios → lines 1–282 of a 325-line
file; `modellimitsresolver…`: 8/8 → lines 1–119 of a 1,140-line file). Fix =
re-point each scenario at the specific function(s) implementing it
(`link-coverage` with precise line ranges).

### P2 — hygiene

- 761 impl-line refs past EOF: re-link or trim.
- 552 scenarios with no test mapping: decide spec-only vs. link tests.
- 297 test mappings missing impl links: fill or drop.
- 345 impl blocks >300 lines: split into per-function mappings.

## Suggested process (feeds AUDIT-001)

Turn this ad-hoc pipeline into a first-class fspec capability, e.g.
`fspec audit-coverage --semantic`:

1. **Structural phase** (always on, fast, CI-able): run the Pass-1 checks,
   fail the build on `*_file_missing` and `*_beyond_eof` regressions.
2. **Semantic phase** (on demand): build the laya dataset (metadata-first
   state string, 2,300-char excerpts), batch 60 questions per model load,
   run in parallel under a RAM cap, emit a triage report.
3. **Gate integration**: block `update-work-unit-status → done` when a
   feature's audit score drops below threshold (e.g. >10% divergent),
   mirroring the existing link-coverage enforcement.

Acceptance criteria sketch (Gherkin):

```gherkin
Scenario: structural audit detects a dead impl mapping
  Given a coverage file whose impl mapping points at a deleted file
  When fspec audit-coverage runs
  Then the report lists the scenario under impl_file_missing
  And the exit code is non-zero when --strict is set

Scenario: semantic audit classifies a whole-file blob mapping
  Given a feature whose scenarios all map the same >100-line range
  When fspec audit-coverage --semantic runs against it
  Then each such scenario is flagged divergent or unverifiable
  And the report ranks it in the top-divergent table
```

## Caveats

- Laya verdicts are a **prioritized review queue**, not ground truth
  (mean confidence ≈ 0.006–0.025 by design of the model's task).
- Structural defects are hard facts; act on those first.
- Excerpt truncation (1024-token window) can hide logic that lives outside
  the mapped lines → some divergent calls are "unprovable from excerpt".
