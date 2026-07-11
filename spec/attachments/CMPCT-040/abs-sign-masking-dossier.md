# CMPCT-040 — COMPACTED Badge Sign-Masking via `.abs()` / `Math.abs()`

**Date:** 2026-07-09
**Source:** Supervisor DeepSearch audit (post-CMPCT-038), confirmed against current HEAD.
**Type:** Bug — display integrity
**Depends on:** CMPCT-039 (source clamp — now DONE). Safe to implement.

---

## 1. The Problem

If a negative compaction reduction reaches the session header, BOTH stacks flip it
into a fake POSITIVE badge instead of showing the truth:

- Rust twin: codelet/fspec-tui/src/views/agent/header_build.rs:160-166
    Some(r) => format!("[{}%: COMPACTED {}%]", tokens.context_fill_pct, r.abs()),
  Comment claims ".abs() is TS parity only — purely defensive".
- TS twin: src/tui/components/SessionHeader.tsx:131
    `COMPACTED ${formatPercentage(Math.abs(compactionReduction))}%`

A negative means the context GREW during compaction. abs() renders growth as
reduction — actively lying to the user.

## 2. Writer Inventory (verified)

### Rust header value (compaction_reduction_by_session)
Exactly ONE writer:
- StreamChunk::CompactionComplete → dispatch_stream_chunks.rs:145-147 —
  compression_ratio.round() as i32 → set_compaction_reduction. No local clamp
  (relies entirely on upstream CMPCT-038/039 clamps).
Confirmed non-writers: dispatch_slash_commands.rs:89-91 (notice only, never badge);
store chrome_state.rs:92-95 is a dumb setter; clear paths only remove
(dispatch_stream_chunks.rs:76, dispatch_compaction_hide.rs:67); no restore/
persistence writer (in-memory HashMap, store/agent_view.rs:107).

### TS header value (compactionReduction)
- AgentView.tsx:978 — Math.round(result.compressionRatio) from CompactionComplete
  chunk (upstream-clamped).
- AgentView.tsx:2763 — setCompactionReduction(result.compressionRatio) RAW from
  NAPI sessionCompact RPC result. Was actively masking pre-CMPCT-039.
- AgentView.tsx:5605 — same raw assignment (retry dialog).

## 3. The Fix (in order)

1. Precondition: CMPCT-039 merged (DONE) — compression_ratio() clamps, all wire
   producers ship [0,100].
2. Rust: clamp at the single writer (dispatch_stream_chunks.rs:145 — e.g.
   .round().max(0.0) as i32 or .max(0) on the i32) so the display layer needs no
   defense. Then REMOVE r.abs() in header_build.rs and replace the misleading
   comment with one stating the writer-clamp invariant.
3. TS: REMOVE Math.abs in SessionHeader.tsx:131. Defensively clamp the two raw
   writers (AgentView.tsx:2763, :5605) with Math.max(0, result.compressionRatio)
   so the component invariant (compactionReduction >= 0 when non-null) holds even
   against a stale/unclamped backend.
4. Do NOT change store setter semantics, auto-hide behavior (RPC-417), or the badge
   format string.

## 4. Required Test Coverage (currently NONE for negatives in either stack)

1. Rust writer-clamp test: dispatch ChunkReceived with CompactionComplete
   { compression_ratio: -150.0 }; assert stored reduction is 0 and rendered header
   shows COMPACTED 0% (never 150%, never -150%). Model on
   agentview_session_header_compaction_percentage_rpc100.rs /
   compaction_reduction_display_contract_rpc420.rs patterns.
2. Rust renderer honesty test: with a negative forced into the store via the direct
   setter, the renderer must NOT sign-flip (pins the .abs() removal — renders the
   stored value verbatim).
3. TS SessionHeader rendering test: compactionReduction={-35} must NOT render
   COMPACTED 35%. Writer tests: negative compressionRatio RPC result stores 0.
   Extend SessionHeader.rendering.test.tsx (currently only tests 35.567).
4. Regression: RPC-100 / RPC-417 / RPC-420 header and badge tests stay green.

## 5. Files Expected to Change

- codelet/fspec-tui/src/app/dispatch_stream_chunks.rs (writer clamp)
- codelet/fspec-tui/src/views/agent/header_build.rs (remove .abs(), fix comment)
- src/tui/components/SessionHeader.tsx (remove Math.abs)
- src/tui/components/AgentView.tsx (clamp at :2763 and :5605)
- New/extended tests in codelet/fspec-tui/tests/ and src TS test suites

## 6. Out of Scope
- The unclamped producers themselves → CMPCT-039 (done).
- The premature /compact RPC measurement → RPC-421.
- original_tokens basis / cache double-count → CMPCT-041.
