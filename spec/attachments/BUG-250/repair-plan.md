# BUG-250: Persist durable transcript errors and interruption outcomes

## Priority and evidence

**P2 — confirmed event-archive gap.**
`rust/agent-loop/src/background_output.rs:517` persists accumulated assistant content
before an error but emits the error itself only as a stream chunk. Cold resume therefore
cannot faithfully reconstruct the original failure sequence. Some interruption information
already exists on assistant metadata; preserve it instead of introducing competing meanings.

## Required repair and code boundaries

1. Inventory user-visible events and classify durable transcript outcomes versus transient chrome:
   errors, cancellation/interruption, failed compaction and settled terminal outcomes need explicit
   policy; spinners, provider-waiting state and token-rate animation must not revive as active state.
2. Define versioned transcript-only event envelopes with stable session/request/attempt identity,
   original timestamp, sequence, severity and sanitized message. Do not disguise these as user dialogue.
3. Persist the settled event in causal order relative to assistant content and tool outcomes.
   Establish acknowledgement/retry through the shared persistence mechanism, not a log-only side path.
4. Decode events into archival display without inserting error prose into model request history,
   duplicating interruption metadata, changing token accounting or blocking legacy text-only archives.
5. Apply BUG-244's exact-once replay identities and BUG-245's separate display/model projections.
   Reconnect and repeated resume must not duplicate notices or reactivate old pending operations.
6. Keep payload privacy equivalent to BUG-239: no raw provider bodies, credentials, prompts, tool payloads,
   or hidden reasoning in diagnostic/event messages without an explicit supported archival requirement.

Inspect `rust/agent-loop/src/background_output.rs:517`, interruption handling at line 522,
`rust/agent-loop/src/persist.rs`, `rust/core/src/persistence/message_envelope.rs`,
`rust/sessions/src/replay_content.rs`, and TUI chunk/error rendering.

## Regression scenarios

- Provider error follows partial assistant output: cold resume shows the authentic partial content
  and one original error with its original timestamp and correct position.
- User cancellation and loop interruption retain distinct genuine reasons; legacy metadata remains readable.
- Compaction fails before promotion: transcript records failure without a false completion or installed DAG.
- Repeated resume, reconnect, and live/snapshot overlap retain one settled notice per identity.
- The next physical model request excludes display-only events unless a separately specified reminder is needed.
- Sanitized errors omit credential-like strings and sensitive provider/tool payloads.
- Transient waiting/spinner events do not reappear as current activity after cold resume.
- Event persistence failure follows BUG-246's visible recovery contract rather than disappearing silently.

## Scope boundaries and questions to settle

- Graceful, known errors/interruption outcomes are in scope. Recovering an abrupt crash's unknown
  outcome is BUG-249; both must share lifecycle definitions.
- Decide retention and export behavior for durable notices before adding new event classes.
- Decide how multiple layers reporting the same causal failure map to one user-visible event.
- Existing private application logs remain useful diagnostics, not the authoritative transcript.

## Completion evidence

- [ ] Event scope/schema, legacy handling and privacy policy are specified before implementation.
- [ ] Real persisted error/cancellation replay tests assert ordering, identity and timestamp.
- [ ] Display-only events do not contaminate provider input or reactivate transient state.
- [ ] Red/green evidence, scoped checks and scenario/implementation coverage are attached.

Related: BUG-244, BUG-245, BUG-246, BUG-249, BUG-239. Investigation:
`docs/investigations/2026-10-23-compaction-and-resume.md`.
