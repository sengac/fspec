# Provider owner readiness — 2026-10-18

Owner `c0448260` owns providers only, not patched Rig. Initial source audit
preserves all inherited staged edits; no staging, commit, branch or board action.
Coordination messages to parent, CLI, sessions, Rig and gate owners were rejected
as busy/not queued; this attachment is the durable mailbox.

## Initial checkpoint findings

- CMPCT-083 cross-layer F1 remains a CLI/gate integration concern: authentic
  checkpoint-only assistant envelopes retain metadata but must not be decoded
  as conversational messages with nonempty Anthropic content.
- F2's last-checkpoint-wins source is superseded by current
  `codex/checkpoint_restore.rs`. Selection validates every checkpoint before live
  restore, recognizes the leading committed synthetic boundary, ignores valid
  older-generation suffix checkpoints without deleting their envelopes, and
  rejects incompatible retained identity. This is initial source evidence,
  not fresh-process acceptance evidence.
- `ProviderSession::validate_restore(&[String])` and
  `validate_committed_restore(generation, &HashMap<String, Value>, &[String])`
  are present. The latter allocates isolated Codex history; a shallow session
  clone is not a sandbox because its extensions share Arcs. CLI/sessions must
  preflight before changing live boundary, token counters or readiness.
- Existing generation freshness guards and idempotent boundary installation
  are retained. Their final inherited logs are not assumed current success.

## Bounded verification in progress

`/tmp/cmpct-provider-owner4-checkpoint-baseline.log` runs explicit provider
078 precedence/lineage/idempotence/freshness,080 replay/invalid/preflight and081
transport targets with `--profile ci-test -j2 --features test-support` against
the shared target. No duplicate broad workspace build, ignored gate or fallback.
Gate owner retains authentic bound-writer prefix/suffix and fresh-process tests;
passing direct provider targets cannot certify those cross-layer requirements.

No new production code is justified without a genuine failing regression first.
