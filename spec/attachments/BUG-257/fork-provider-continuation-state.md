# BUG-257: Forked Codex sessions lose provider continuation state

## Review context

- Reviewed commit: `144653b0dc12ca096e835e86b620b109b29019bf` (`more fixes`).
- Review priority: P2 — forks of new-format Codex conversations cannot cold restore.
- Evidence: static inspection of persistence, fork construction, and checkpoint selection; no end-to-end fork test was executed during review.

## Problem and impact

Provider checkpoints now live in a separate provider-state store referenced by
the session manifest, rather than synthetic assistant message references.
`SessionStore::fork` still creates a new manifest and copies only conversation
message references. Its `provider_state` remains empty.

Forking a completed new-format Codex conversation therefore produces a child
with dialogue but no continuation checkpoint. Model replay cannot recover that
checkpoint from the copied originals, and Codex checkpoint selection rejects
the nonempty replay with `CheckpointError::Missing`.

## Source evidence

Locations refer to the reviewed commit:

- `rust/cli/src/session/provider.rs:93` persists extension state through the new provider-state writer instead of appending a synthetic message.
- `rust/core/src/persistence/provider_state.rs:51` publishes the separate state root into the manifest.
- `rust/core/src/persistence/manifest.rs:158` initializes new manifests with no provider state.
- `rust/core/src/persistence/manifest.rs:417` constructs the child manifest during a fork.
- `rust/core/src/persistence/manifest.rs:429` copies message references but does not install child provider state.
- `rust/core/src/persistence/provider_state/replay.rs:5` reads continuation metadata from the manifest state root or legacy checkpoint messages.
- `rust/providers/src/codex/checkpoint_restore.rs:107` rejects nonempty replay without a selected checkpoint or explicit reset.

## Reproduction to automate

1. Complete an uncompacted Codex turn using the new provider-state writer.
2. Reload its manifest and confirm the continuation exists outside message references.
3. Fork at the final conversation message using `fork_session`.
4. Cold restore the child through normal model replay.
5. Observe missing continuation state despite copied conversation history.

Expected: a supported fork can restore a valid continuation for exactly its
selected conversation prefix without mutating the parent.

## Suggested repair direction

Make fork construction explicitly handle provider state. A state root is bound
to a session and source prefix, so simply copying the parent's reference is not
safe. Re-anchor valid child state or define a supported provider-specific fork
transition. Earlier fork boundaries must never import later native history.
Keep provider checkpoints outside conversation originals and preserve lineage
validation rather than bypassing the missing-checkpoint error.

## Acceptance and validation guidance

- A fork at the final message of a completed new-format Codex turn cold restores.
- The child's next physical request has the intended prefix and no later parent content.
- Parent state and conversation coordinates remain unchanged.
- Cover older fork boundaries and reject unsupported boundaries explicitly.
- Retain legacy-checkpoint, provider-state integrity, and compaction-boundary coverage.

Related specifications: `spec/features/provider-state-original-separation.feature`,
`spec/features/provider-state-recovery.feature`, and
`spec/features/archival-model-replay-separation.feature`.
