# BUG-249: Preserve in-flight transcript and tool lifecycle on abrupt termination

## Priority and evidence

**P2 — confirmed crash-window completeness gap, not the cause established for the screenshot.**
Visible text, thinking, and tool invocations accumulate in memory until result/Error/
Interrupted/Done boundaries. Abrupt process termination during a tool can lose the visible
prefix and invocation despite external side effects. Graceful interruption already flushes content.

## Code to inspect

- `rust/agent-loop/src/background_output.rs:359`: accumulates text/thinking without a durable incremental record.
- `rust/agent-loop/src/background_output.rs:385`: accumulates invocations before their result arrives.
- `rust/agent-loop/src/background_output.rs:410`: first assistant persistence at tool-result handling.
- `rust/agent-loop/src/background_output.rs:517`: graceful error/interrupt/done persistence paths.
- Review response-attempt snapshots/abort handling and provider checkpoint publication alongside these paths.

## Required repair

1. Specify the durability boundary for displayed output and tool dispatch. If full displayed-prefix
   survival is required, journal before publishing that prefix; otherwise document an explicit bounded
   loss window and obtain approval rather than silently promising complete conversation persistence.
2. Use versioned append-only deltas or bounded checkpoints with stable session/request/attempt/message/
   call identity. Do not serialize the entire history for every token.
3. Durably record invocation identity/lifecycle before executing side-effecting work under the chosen
   guarantee. Distinguish requested, started, completed, cancelled, not-started, and outcome-unknown.
4. On restart reconstruct the authentic partial transcript and a truthful incomplete/interrupted state.
   Never infer tool success from a requested call or automatically replay an unknown side effect.
5. Preserve response-attempt rollback semantics: content from aborted/retried provider attempts must
   not accidentally become committed assistant content or duplicate a successful attempt.
6. Reconcile journal promotion/compaction with final message commits, provider checkpoints and cleanup.
   Share retry/durability/framing mechanisms from BUG-246/247/248; approve major schema changes first.

## Regression scenarios

- Spawn an isolated real agent process, observe a known text prefix, terminate it before Done, and
  recover the prefix according to the specified acknowledgement boundary in a fresh process.
- Persist/start a tool that writes a sentinel file, terminate before its result is archived, then reopen;
  show an authentic unknown/interrupted lifecycle and assert the sentinel action is not executed twice.
- Terminate before dispatch, during dispatch, after result capture and during final message promotion.
- Abort a provider response attempt, retry successfully, terminate/reopen, and verify correct attempt visibility.
- Graceful interruption, normal completion and repeated recovery produce no duplicate content.
- Large/rapid streams remain bounded and avoid quadratic full-history write amplification.

Use subprocess synchronization and shared fixtures rather than killing the live fspec daemon.
Keep process-crash tests distinct from BUG-247 power/system-crash durability verification.

## Non-goals and prohibited shortcuts

- Do not synthesize successful outcomes, rerun tools to recreate missing results, or persist opaque
  provider reasoning in diagnostic logs. Journal sensitive content only in the established private store.
- This card does not justify disabling native provider lineage checks or retaining aborted attempts as facts.

## Completion evidence

- [ ] Acknowledged-visible-prefix and pre-dispatch guarantees are explicit in scenarios/documentation.
- [ ] Real subprocess termination/restart tests cover content, side effects, and attempt rollback.
- [ ] Recovery, promotion, retention and size behavior are versioned and idempotent.
- [ ] Feature-first tests and scoped quality/coverage checks pass.

Related: BUG-246, BUG-247, BUG-248, BUG-250. Investigation:
`docs/investigations/2026-10-23-compaction-and-resume.md`.
