# Replacement runtime resource checkpoint

BUG-268 remains native **implementing**, not complete. No later card was started.
No session, model, compaction, publication, credential, network-policy or cleanup
operation was performed. All earlier unpublished work and evidence remain.

## Verified results

- The preceding canonical-root correction was already present in test_support.rs.
  The replacement reran it: three original issuer-isolation cases pass in
  `issuer-isolation-canonical-root.log`.
- Added specification-first tests for changed credential roots (direct and Unix
  symlink) and external inference origins returned by the synthetic issuer.
  All five isolation cases pass in `issuer-isolation-final.log`, before any
  full expired-token fixture was executed.
- Wired the already-approved nondefault issuer seam into the existing exchange
  entry point. Active invalid fixture roots return an error rather than falling
  through to the external issuer. Shipping builds exclude this seam.
- Actual two-genuine-turn refresh fixture has a causal Authorization RED:
  `live-refresh-causal-red.log` observes `Bearer synthetic-short-token`, not
  `Bearer synthetic-refreshed-copilot`. All auth/config is synthetic/temp;
  issuer and inference captures are numeric-loopback.
- Generated only the two missing BUG-268 coverage sidecars using the native
  generator in a temporary two-feature projection, then installed its unchanged
  generated output. The global dry run would also create two unrelated MCP
  sidecars, so it was not applied globally. Native link-coverage now maps all
  four existing live-session tests and all five isolation tests, without bypass.
- The first implementing transition was refused for missing coverage and did
  not execute. After that specific issue was resolved, one native retry passed.

## Unfinished production work

The request middleware now shares the provider's existing refresh coordinator,
updates Authorization at send/send_streaming/send_multipart, and follows the
coordinator's effective base URL. No new inference retry was added.

New bounded CopilotSession extension, private capture, binding and typed-model
integration are partially implemented. They use existing ProviderSession Arc
ownership, typed request/preamble/history/tool comparison, long-lived GitHub
credential/deployment/model binding, attempt epochs and sequence checks, drop
invalidation, reset/generation hooks, and no writer/checkpoint/restore adoption.
Live session binding detaches Copilot only for a different persisted session ID;
generic Clone and Codex persistence behavior are not modified. Opaque injection
is at the HTTP boundary after generic tracing; response capture strips the
known field before generic response processing and persistence.

These additions are NOT verified complete. Do not infer success from compiling
types. Preliminary integration attempts and errors are preserved separately.
`live-session-integration-third-attempt.log` compiles and executes four tests,
but all four fail because native Done is not observed. Source inspection found
the cause: existing shared Chat/Responses adapters emit Final BEFORE tools,
whereas the initial Copilot wrapper released buffered tools only upon Final.
The latest wrapper patch now drains the finite already-settled adapter stream,
commits actual settled assistant output, then emits Final before tools. This
patch has NOT been compiled or executed yet. The earlier outbound three-test
green log predates this new integration and is not final-source evidence.

Need final-source genuine two-turn/refresh green; real cancellation/stale/
failed/provenance/history/cleared/compaction/new/fork/restore/provider-switch
runtime guards; nonpersistence/debug/writer proof; every public front door and
actual nested Responses vision/header classification; old public consumer
tests; scoped quality/format/tags/coverage/direct review and native closure.
The Responses classifier currently checks only flat input_image entries; actual
nested input message image coverage must be addressed with a causal runtime
test, not silently accepted. Existing shared adapter reasoning/history must
remain unchanged.

## Resource stop

Before the last build: 7,265,564 KiB available. After compilation/test execution:
6,119,216 KiB, below the strict 6,291,456 KiB (6 GiB) reserve. A follow-up measured
6,201,360 KiB, still below reserve. No Cargo/rustc/BUG-268 test process remained.
No more build/test/lint invocation was started after observing the breach.

Supervisor-controlled cleanup is required: interrupt/pause this agent, verify no
active build or patch, confirm `/Users/rquast/projects/fspec/rust/target` scope,
then use the user's existing Cargo-clean authorization from the Rust workspace
and measure before resuming. The installed live fspec binary and separate
Documents validation target must remain untouched. Do not repeat immutable
historical baselines after cleanup without a changed-source concern.
