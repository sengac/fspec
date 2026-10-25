# Replacement progress and controller handoff

This is a normal operational handoff document, not an injected/compaction
summary. No session/subagent or compaction tool was used. Preserve the original
session 64d78cce-6820-4adc-b7c5-dca958c23b6d and all unpublished work. Do not
resume it, retry compaction, repair checkpoint lineage or change models.
The requested runtime remains codex/gpt-6.1-sol.

## Exact current state

The fixed thirteen-card inventory remains authoritative. BUG-259 through BUG-267
are Done; BUG-268, BUG-269, PROV-162 and CONT-011 remain backlog. The latter four
and BUG-254 specifying were freshly verified through native show-work-unit JSON.
BUG-254 and earlier BUG-253/255/256/257/258 were not changed. No outside card,
commit, publication, daemon, SDK, security/network setting or real credential
was changed. Coordinator CONT-011 is still last, not completed.

The replacement finished BUG-265 final review/traceability/native Done, then
fully completed BUG-266 and BUG-267 sequentially with features/tests before
production, real causal red, final green, affected controls, quality and native
workflow. Each has an attached `validation-and-review.md` with exact approvals,
invalid attempts distinguished from causal failures, warnings and exclusions.
Do not repeat their decisive tests absent changed source/unresolved concern.

## BUG-265 closure additions

- Final nine safe shape tests compile root function-pointer witnesses and pass.
- BUG-228 shared dispatch guard now checks both genuine/recovery sync calls and
  rejects removing either; its final test and scoped lint pass.
- CMPCT-041 test reads extracted metrics correctly, then fails on its stale NAPI
  arm reader. `final-test-source-review.log` records it. The added focused
  `cmpct041-focused-layout-baseline.json` proves NAPI agent_loop.rs byte-identical
  to immutable b2475dcd and the same absent CompactionStarted predicate. This is
  focused static proof, not a full baseline Cargo run. CONT-011 owns retargeting
  this reader while preserving all accessor/writer assertions.
- Five nested Cargo shape scenarios and the RPC-043 NAPI nested default-check
  test were NOT run; do not imply witnesses execute those subprocess tests.
- Global validate-tags reports 522 unrelated feature violations; scoped native
  tag validation passes for new features. Do not claim global validation green
  or fix unrelated tags. All relevant coverage audits pass.

## BUG-266 completed behavior

Approved content stop is nonfinal; late usage survives to [DONE] or clean EOF
after stop. Post-stop transport errors remain typed errors; stop-only pending
transport does not invent successful settlement. No timeout or new marker.
stream.rs types were mechanically moved byte-identically to stream_types.rs,
re-exported through the old path before repair; pre/post-move BUG-227 controls
and unchanged causal reds are retained.

The behavior fix adds content_stopped filtering (only usage after stop), flushes
content without terminating parsing, and buffers executable calls in the real
rig adapter until successful FinalResponse is emitted. Native Done now bills
12 input / 3 output exactly once before acceptance. 37 distinct tests pass;
default check and final scoped Clippy pass (reused helper dead-code warnings).
Final test-only lock-across-await warning was fixed and rerun successfully.

There are two features to satisfy the native 1-feature/1-test gate:
custom-stream-settlement (six tests, providers) and custom-done-settlement (one
test, CLI). Native line relinking appends a second mapping instead of updating;
unlink by test file removes only the first. This caused two known-not-executed
validating denials. Supervisor explicitly approved exact scenario-only native
--all reset/restoration, audit and one retry; that retry succeeded. All evidence
is retained. Never repair by direct coverage JSON edits or relaxing gates.

## BUG-267 completed behavior

Only agent-loop/src/agent_loop/providers.rs changes production: after the
existing builder-error short circuit, match the inner manager's selected typed
Custom identity when no explicit facade exists, and call the existing custom
builder with the exact slug. Preserve test-support stub routing, native branches
and explicit facade routes. Native approval explicitly leaves incompatible
Custom/native explicit overrides fail closed; no expanded MODEL-004 policy.

Five real shared-loop tests cover six colliding native names, four alias names,
one noncolliding script, three native/escape/compatible-explicit cases and one
incompatible-explicit no-request rejection. Captures assert actual URI/dialect,
upstream model, attributed native Done schema, pre-request arming, accepted Done,
exact persisted physical usage and no nudge. New support modules are bounded and
temporary HOME/config/CWD/environment is restored. Two causal failures/three
controls before repair; final all five pass. Existing five shadowing + ten tool
facade + seven BUG-265 recovery + one exact test-support stub pass, 28 distinct
tests total. Default library check, final scoped Clippy/format/review pass.

## Next work and resource boundary

Start BUG-268 with native lifecycle/discovery, actual endpoint model criteria and
attachment. Only its card/attachment were read; no spec/test/production for it
was started. Its provider.rs is already 292 lines: request any needed major split
approval before changes. Use existing Responses adapter from BUG-264, not new
business logic. Synthetic numeric-loopback actual requests only, preserving
credentials, headers, tools/history/opaque reasoning and armed stop/nudge/Done.

Then BUG-269 real scripted OAuth request-path integration preserving script-owned
refresh; then PROV-162 after concrete native approval of typed replay policy from
codex-protocol-replay-policy.md; then CONT-011 final affected-source matrix and
stale-reader cleanup. Prior discovery remains in remaining-source-preparation.md.
No real provider tests, keychain changes, downloads, emulator/Windows/Valheim work.

Last measured free disk: 8.0 GiB, strict reserve 6 GiB; no active cargo/rustc
process was found at handoff. All Cargo invocations remained offline, ci-test,
explicit package and target, jobs two. No clean was run by this replacement.
If blocked by disk, report to supervisor for the specifically authorized scoped
project Rust clean only after no active build/agent/patch writes; do not delete
the separate Documents/Codex validation target or arbitrary caches.

All native coverage generation used isolated disposable projects containing only
the exact feature to avoid global sidecar changes, copying the native-generated
artifact unchanged before ordinary validated links. All later links/rebases are
native. Source/spec/report edits use apply_patch. No work-unit JSON was edited.
