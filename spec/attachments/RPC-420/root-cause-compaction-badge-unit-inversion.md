# RPC-420 — Compaction reduction badge shows ~9800%: root-cause dossier

**Date:** 2026-07-08 (restored after add-attachment self-copy truncation)
**Author:** Supervisor session (first-principles DeepSearch, 3 independent traces)
**Symptom:** After a compaction, the Rust TUI SessionHeader briefly (10 s, RPC-417 auto-hide) shows
`[X%: COMPACTED ~9800%]` when the real reduction is on the order of 60–99%. The scrollback
`[compaction] ...% reduction` notice is equally garbled (e.g. `-9700.0% reduction`).

---

## 1. The canonical wire contract (established from first principles)

`rpc_types::CompactionResult.compression_ratio` is a **percent of tokens removed, in [0, 100]**.

Evidence — every real producer multiplies the fraction helper by 100 before shipping:

| Producer | Code | Reaches |
|---|---|---|
| `codelet/sessions/src/handle_impl.rs:335` | `compression_ratio(original, compacted) * 100.0` | `/compact` RPC return (embedded + WS) |
| `codelet/agent-loop/src/inject_summary_handler.rs:92` | `compression_ratio(...) * 100.0` | **the only `CompactionComplete` StreamChunk the TUI receives** |
| `codelet/napi/src/session_bindings.rs:3129` | `compression_ratio(...) * 100.0` | TS `sessionCompact` result |
| `codelet/napi/src/inject_summary_handler.rs:104` | `compression_ratio(...) * 100.0` | TS `CompactionComplete` chunk |
| `codelet/napi/src/agent_loop.rs:1715` | `info.compression_ratio * 100.0 // Convert to percentage` | dead fallback arm (fraction→percent, correct) |
| `codelet/agent-loop/src/background_output.rs:306` | `info.compression_ratio * 100.0` | dead fallback arm (fraction→percent, correct) |

The underlying helper (`codelet/cli/src/interactive_helpers.rs:179-185`) returns a **fraction of
tokens removed** in [0,1]. The internal `CompactionCompleteInfo` StreamEvent is documented as
fraction (`codelet/cli/src/interactive/output.rs:132-133`) and the `* 100.0` conversions happen
exactly once at each wire boundary. **The wire is percent. Consistently.**

## 2. The reference implementation (TS Ink TUI) displays it directly

- `src/tui/components/AgentView.tsx:978` — `setCompactionReductionRef.current?.(Math.round(result.compressionRatio));`
  (also raw pass-through at lines 2763 and 5605)
- `src/tui/components/SessionHeader.tsx:129-132` —
  `[${fp}%: COMPACTED ${formatPercentage(Math.abs(compactionReduction))}%]` — no arithmetic.
- TS fixtures sanity-check perfectly as percent-removed:
  - `sessionFixture.ts:136`: 8500→3200, `compressionRatio: 62.4` — (8500−3200)/8500 = 62.35% ✓
  - `PERF-002...test.tsx:284-290`: 150000→40000, `compressionRatio: 73.3` — 73.33% ✓
  - `SessionHeader.rendering.test.tsx:161-167`: input `35.567` renders `COMPACTED 35.57%` — 1:1 ✓
- No TypeScript code anywhere computes `(1 - ratio) * 100` (exhaustive search).

**Reference display formula: `display = round(compressionRatio)` (percent, used as-is).**

## 3. The defect — fspec-tui alone invented a third convention

Two compute sites treated the wire value as **fraction remaining** (compacted/original):

1. `codelet/fspec-tui/src/app/dispatch_stream_chunks.rs:141-142` (badge):
   `((1.0 - compaction_result.compression_ratio) * 100.0).round() as i32`
2. `codelet/fspec-tui/src/app/dispatch_slash_commands.rs:287-296` (`format_compaction_notice`):
   `(1.0 - result.compression_ratio) * 100.0`

| wire value (percent removed) | `(1 − r) × 100` | after `.abs()` (header_build.rs:161) |
|---|---|---|
| 60.0 | −5900 | `COMPACTED 5900%` |
| 99.0 | −9800 | `COMPACTED 9800%` ← observed symptom |

The `.abs()` masks the sign flip (TS's `Math.abs` is a defensive guard — keeping it for parity is
fine; it is not the bug, it merely hid it).

### 3.1 The wrong convention was encoded in specs and tests too

The RPC-100 feature's architecture note contained the sentence that introduced the defect:
> "(TS bug: should be (1-ratio)*100; Rust will use the correct formula)" —
> `spec/features/agentview-session-header-compaction-percentage.feature:13`

That note was **wrong**: TS was correct; the spec inverted it. Downstream artifacts inherited it:
RPC-100/RPC-047/RPC-417 feature rules/examples/scenarios; fixtures in `slash_compact_rpc047.rs`,
`agentview_session_header_compaction_percentage_rpc100.rs`,
`agentview_compaction_badge_auto_hide_rpc417.rs`, `tests/common/mod.rs:749` (MockBackend `1.0`).
The fixture `10000→4000, compression_ratio: 0.4` matched **neither** the helper's fraction-removed
(0.6) **nor** the wire percent (60.0) — it was "fraction remaining", a three-way inconsistency.

### 3.2 Why no test ever caught it

`rpc037_cross_transport_parity.rs` was served by `StubSessionManagerHandle`'s hand-written `0.5`
(`codelet/core/src/session_manager_handle.rs:1669-1682`). The real producer returns `50.0` for
1000→500. **No test exercised a real producer.**

### 3.3 Root enabler

`codelet/rpc-types/src/lib.rs:894-900`: `pub compression_ratio: f64` had **no documented unit**.

## 4. Required fix (display side — the backend is CORRECT and must NOT change)

> ⚠️ An earlier analysis recommended removing the `* 100.0` in the backends. That is **wrong**: it
> would break the still-alive TS Ink frontend. The wire contract is percent; only fspec-tui misread it.

1. `dispatch_stream_chunks.rs`: `let reduction = compaction_result.compression_ratio.round() as i32;`
2. `format_compaction_notice`: `let reduction_pct = result.compression_ratio;` (keep `{:.1}`).
3. `header_build.rs`: keep `r.abs()` + explanatory comment (TS parity, defensive only).
4. `chrome_state.rs:86-91` doc comment: pass-through contract.
5. `rpc-types` `CompactionResult`: doc-comment percent unit `[0,100]`, NOT a fraction.
6. `StubSessionManagerHandle`: `0.5` → `50.0`; update rpc037 assertions.
7. `tests/common/mod.rs:749` MockBackend default: `1.0` → `0.0`.
8. Test fixtures → percent-removed (0.4→60.0, 0.3→70.0, 0.25→75.0, 0.5→50.0); asserted display
   strings stay THE SAME.
9. Regression scenario: wire `99.0` → `COMPACTED 99%` (never ~9800, never negative).
10. Producer-truth test: `compression_ratio(10000, 4000) * 100.0 == 60.0` feeding the pipeline.
11. Spec corrections via fspec CLI for RPC-100/RPC-047/RPC-417 + strike the "TS bug" claim.

## 5. Resolution status

**IMPLEMENTED** by worker session 06d5514c (2026-07-08). New capability spec:
`spec/features/compaction-reduction-display-contract.feature` (@RPC-420) + new test
`codelet/fspec-tui/tests/compaction_reduction_display_contract_rpc420.rs` (5 scenarios, 100%
coverage). All fixture/spec conversions applied; red 15 failures → green across 241 test binaries;
`cargo check --workspace` clean.

## 6. Out of scope (tracked separately)

- **CMPCT-038**: chunk's `compacted_tokens` counts the DAG summary only (~99% vs ~60% real).
- **RPC-421**: premature `/compact` RPC-result notice (fake ~99%, measured pre-injection).
