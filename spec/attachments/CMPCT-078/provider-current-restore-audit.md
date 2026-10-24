# Current provider restore audit

Provider owner e22f772a preserves existing precedence, repeated-installation and
failure-atomic replay behavior and tests. Current physical/fresh-process Codex
acceptance remains blocked outside providers by finite-positive restored context
limits; the gate must continue checking complete fresh physical suffixes.

One additional direct-provider gap was found while auditing repeated restoration:
`checkpoint_restore::select` compares prior generation but not prior model when
the replay omits a synthetic boundary envelope. A retained same-generation
checkpoint can therefore replace the already installed model. The owned
`codex-idempotent-boundary-installation` feature is extended first, and its actual
integration target now checks typed lineage refusal, checkpoint preservation,
blocked physical transport and successful authentic retry. Behavioral RED is
queued in `/tmp/cmpct-provider-replay-model-red.log`; no production repair has
been applied before that execution.

The existing direct 081 rejected-upgrade regression is strengthened to walk the
actual error source chain and downcast `WebSocketUpgradeRejected`, rather than
only matching status text. It retains every 404/405/426/501 case and requires
exactly one error, no subsequent completion, no POST fallback and unchanged
checkpoint. This does not substitute for CLI's separate real-runtime typed-error
repair. CLI source-loss locations are recorded in the 071 current media audit.

No board changes, staging, commits or foreign-owner edits.

## Current model-identity RED and additional freshness finding

The first replay-model command ended at a concurrent core compilation error,
not behavioral RED. `/tmp/cmpct-provider-replay-model-red2.log` then executed
the new test and failed its typed lineage assertion (0 passed, 1 failed,
2 filtered, Cargo exit 101). Only after that execution, selection now refuses
a changed bound model at the installed generation before replacing history.

A focused audit additionally found that the lazy physical stream captures its
generation on first poll rather than when its request is prepared. A prepared
stream can therefore send after failed restore or adopt a newer generation for
its old body. Likewise an old response can publish its checkpoint after a
newer generation is installed. New feature
`codex-physical-request-freshness.feature` precedes its actual integration target
`cmpct078_physical_freshness`. The command is queued in
`/tmp/cmpct-provider-physical-freshness-red.log`; no freshness production repair
is applied before behavioral execution. Retain the fresh-process gate and
all existing suffix, precedence and failure-atomicity assertions.

## Freshness and preflight execution checkpoint

`/tmp/cmpct-provider-physical-freshness-red.log` completed behavioral RED:
0/2 passed, Cargo101. A prepared stream sent after failed restore, and an old
physical response published completion after generation replacement. The stream
now retains generation at construction, checks blocked/generation state before
send and after terminal response, and commits under a generation-checked state
lock. The first focused rerun passed freshness2, idempotent3 and preflight2.

The concurrent gate `/tmp/cmpct083-owner-lineage-repaired-matrix.log` exposed
an intermediate editing regression: the publication guard was attached to
`transition_generation` rather than `commit_for_generation`, rejecting every
advancing private candidate. That placement is corrected: explicit transitions
retain their original advancing behavior; only physical response commits require
the captured generation under the state lock. The earlier seven-test GREEN is
not final evidence for this correction; a full owned rerun follows. Gate owner
should rerun the actual Codex cold and transport negatives without weakening
any candidate admission or readiness assertions.

Nonmutating restore APIs are implemented after behavioral RED2/2; signatures
and integration instructions are in `provider-restore-preflight-api.md`.
CLI and sessions must still validate real malformed cold restoration before
any boundary/token mutation; provider-only success is not that runtime evidence.
