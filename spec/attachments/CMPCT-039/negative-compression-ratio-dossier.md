# CMPCT-039 — Negative `compression_ratio` Reachable on the Wire

**Date:** 2026-07-09
**Source:** Supervisor DeepSearch audit (post-CMPCT-038), confirmed against current HEAD.
**Type:** Bug — wire-contract violation
**Status note:** COMPLETED 2026-07-09 by worker agent (clamp in shared helper + belt-and-braces kept at emit sites). This dossier is retained as the design record.

---

## 1. Root Cause

The shared helper (codelet/cli/src/interactive_helpers.rs:179-185, pre-fix):

    pub fn compression_ratio(original_tokens: u64, compacted_tokens: u64) -> f64 {
        if original_tokens > 0 {
            1.0 - (compacted_tokens as f64 / original_tokens as f64)
        } else {
            0.0
        }
    }

goes NEGATIVE whenever compacted_tokens > original_tokens. Its doc comment claimed
"Returns a value between 0.0 and 1.0" — false for the tiny-session case.

### Why compacted > original is real

execute_compaction (interactive_helpers.rs:533-622) clears the session to surviving
system reminders + pushes the large COMPACTION_INSTRUCTION_FRESH/INCREMENTAL text
(INCREMENTAL embeds the entire prior DAG), then recalculate_token_tracker. Every
unclamped producer measures compacted_tokens at this point — before the agent builds
the summary. For small sessions, reminders + instruction exceed the original count
(fspec's reminders alone are ~35k tokens) → negative ratio.

## 2. Producer Audit (pre-fix state)

### Already clamped (CMPCT-038)
- codelet/agent-loop/src/inject_summary_handler.rs:110-111 — (ratio * 100.0).max(0.0)
- codelet/napi/src/inject_summary_handler.rs:119-120 — same

### UNCLAMPED — negative reachable on the wire
1. codelet/sessions/src/handle_impl.rs:335 (compact_session, RPC-418) — HIGH
   reachability. Surfaces: embedded transport → dispatch_slash_commands.rs:89-93 →
   format_compaction_notice (:291) → scrollback "[compaction] -150.0% reduction";
   WebSocket: rpc/src/lib.rs:1513 → tarpc JSON → transport/websocket.rs:746-751.
2. codelet/napi/src/session_bindings.rs:3129 (session_compact) — HIGH. Negative
   compressionRatio in the NAPI CompactionResult to JS/TS.
3. codelet/cli/src/interactive/repl_loop.rs:95 (CLI /compact) — HIGH. Printed
   verbatim to stdout (:110-112) + debug capture (:106).
4. codelet/cli/src/interactive/recovery_compaction.rs:450 — MEDIUM-LOW, debug
   capture JSON only.

### Latent (production-dead)
output.rs:488, background_output.rs:306, napi/agent_loop.rs:1721 forward
StreamEvent::CompactionComplete ratios unclamped; the only event producer
(emit_compaction_complete) has zero production callers.

## 3. Wire Contract

RPC-420 (rpc-types/src/lib.rs:899-902, dispatch_stream_chunks.rs:139-141,
header_build.rs:162-164, transport/mod.rs:468-470): compression_ratio is the
percent of tokens removed, in [0, 100]. Producers 1-4 violated this.

## 4. The Fix — Clamp in the Shared Helper

Clamp inside compression_ratio() itself (delivered as .clamp(0.0, 1.0)):
- One clamp fixes all producers at once; structurally prevents new unclamped ones.
- No caller legitimately needs the raw negative — context growth remains recoverable
  from originalTokens/compactedTokens shipped alongside.
- The inject_summary .max(0.0) clamps stay as belt-and-braces with comments.
- The helper doc comment now states the clamped contract truthfully.

## 5. Required Test Coverage (was ZERO for negatives)

1. Helper unit tests: growth → 0.0; normal ≈ 0.6; zero-original → 0.0; equal → 0.0.
2. Producer growth-case tests: compact_session (sessions) and NAPI session_compact
   assert ratio >= 0 and == 0.0 for growth.
3. CMPCT-038 tests remain green.

Delivered in codelet/napi/tests/cmpct039_ratio_clamp_test.rs (6 tests) +
feature spec/features/compression-ratio-clamping.feature (6 scenarios, 100% coverage).

## 6. Out of Scope
- abs()/Math.abs header masking → CMPCT-040 (depends on this card).
- Fabricated /compact RPC measurement → RPC-421.
- pre_compaction_tokens basis / cache double-count → CMPCT-041.
