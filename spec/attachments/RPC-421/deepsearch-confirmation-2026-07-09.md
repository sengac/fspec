# RPC-421 — DeepSearch Confirmation & Scope Additions (2026-07-09)

VERDICT: bug STILL LIVE. Zero RPC-421 markers in codelet/ — the code paths in the
original root-cause attachment are unchanged. This supplement adds newly confirmed
evidence and scope refinements from the post-CMPCT-038 audit.

## 1. Premature measurement — confirmed in BOTH twins + a THIRD instance

RPC handle — codelet/sessions/src/handle_impl.rs:
- :295 — original_tokens = inner.token_tracker.input_tokens
- :302 — execute_compaction(...) → reset_session_to_reminders
  (cli/src/interactive_helpers.rs:576) + recalculate_token_tracker (:613) —
  tracker now measures ONLY reminders + the compaction instruction
- :310 — compacted_tokens read IMMEDIATELY, before the DAG exists
- :327 — send_input("Continue") kicks the loop AFTER measurement
- :332-340 — fabricated CompactionResult (compression_ratio at :335,
  turns_summarized: 0)

NAPI twin — codelet/napi/src/session_bindings.rs (session_compact): identical
sequence — original :3051, execute_compaction :3072, premature measurement :3094,
send_input("Continue") :3118, fabricated result :3126-3132. Debug capture at :3110
records the fabricated value too.

NEW — THIRD instance (scope addition): codelet/cli/src/interactive/repl_loop.rs:94-112
— the CLI REPL reads token_tracker.input_tokens right after execute_compaction and
prints "[Context compacted: X→Y tokens, Z% compression]" on the same fabricated
basis (:111). It self-admits at :113: "[In-view DAG flow — agent will build summary
via SessionSearch]". CLI REPL users ONLY ever see the fabricated number (no
CompactionComplete print on this path).

## 2. The user-visible double-notice contradiction — confirmed

dispatch_slash_commands.rs:88-93 formats the fabricated RPC result via
format_compaction_notice (:290-299) → instant scrollback notice #1 (e.g.
"[compaction] 99.2% reduction (10000 → 80 tokens, 0 turns summarised)").
dispatch_stream_chunks.rs:130-133 EXPLICITLY acknowledges the double emission:
"the slash handler emits its own notice so double-emission for /compact is
acceptable parity with the TS Ink original" — that comment (and the RPC-047 spec
language it reflects) must be amended by this card. Notice #2 (honest, CMPCT-038
apply-site emission) arrives seconds later and CONTRADICTS #1. The header badge
ends up honest (overwritten by the CompactionComplete handler) — only the fake
scrollback line persists.

## 3. Test coverage — one test LOCKS THE BUG IN

- codelet/sessions/tests/rpc418_compact_session.rs:269-273 asserts only
  compacted_tokens < original_tokens — trivially satisfied by the fabricated
  reminders-only value; it encodes the premature measurement as passing behavior.
  THIS ASSERTION MUST BE REWORKED as part of this card.
- slash_compact_rpc047.rs uses canned MockBackend values (formatting/routing only).
- compaction_reduction_display_contract_rpc420.rs covers percent-unit display only.
- No test asserts the RPC result matches the post-DAG context; no test covers the
  double-notice contradiction.

## 4. Fix direction (refined)

1. Single-source the success notice from the CompactionComplete chunk handler
   (fires for both /compact and auto-compaction, carries the honest post-injection
   numbers). The slash handler keeps only the Err branch ("[error] /compact failed")
   and silent no-op behaviors.
2. compact_session (both twins) must stop shipping fabricated reduction numbers —
   either return an acknowledgement-shaped success (no token/ratio claims consumed
   for display) or populate honestly-labelled fields; the TUI must NOT render a
   reduction notice from the RPC result either way. Choose the design during
   specifying; acceptance criterion: EXACTLY ONE compaction notice per /compact,
   carrying post-injection numbers.
3. repl_loop.rs:94-112: print only a "compaction started / summary in progress"
   style message at that point; honest numbers are reported when the DAG lands
   (CompactionComplete path), or not at all for the plain CLI if that path has no
   completion print — do NOT print fabricated numbers.
4. Amend the RPC-047 spec text that blesses double emission as "acceptable parity",
   and the dispatch_stream_chunks.rs:130-133 comment.
5. Strengthen rpc418_compact_session.rs so the RPC no longer ships a fabricated
   ratio presented as a real reduction; add a TUI test asserting exactly one
   compaction notice per /compact.

## 5. Interaction with sibling cards

- CMPCT-039 (helper clamp — DONE) guarantees non-negative values but does NOT fix
  the fabrication — both cards are needed.
- CMPCT-041 (basis unification) changes what pre_compaction_tokens /
  original_tokens mean; RPC-421's honest notice consumes the CMPCT-038 apply-site
  emission, downstream of that basis. No hard dependency, but coordinate notice
  test wording.
