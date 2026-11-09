# Research: compaction commit bookkeeping (CMPCT-086)

## Where it lives
- `CommittedGeneration` (`rust/core/src/persistence/compaction_boundary.rs:53`) is stored as `SessionManifest.compaction_commit`. Its fields are `version`, `identity`, `source { session_id, generation, provider, model, protocol, prefix }`, `covered_prefix`, `verified_suffix`, `checkpoint { provider, model, generation, payload }`, `input_tokens`, `output_reservation`, `token_provenance` and `publication_pending`.
- It is written by `commit_compaction` (`compaction_commit.rs:151`), with `covered_prefix: candidate.source.prefix.clone()` and `publication_pending: true`. `reconcile_compaction` (`compaction_commit.rs:189`) later flips `publication_pending` to false.

## What is redundant
- `covered_prefix` is always a clone of `source.prefix`, and `validate_committed_boundary` rejects any manifest where `commit.covered_prefix != commit.source.prefix` (`compaction_boundary.rs:151`). The CMPCT-084 research measured this on 53 of 53 compacted sessions: the duplicate was 1.32 MB of 6.30 MB of manifest bytes, and up to 1,245 UUIDs listed twice in one manifest. It is rewritten on every manifest revision.

## What is still read after publication (must stay)
- **`checkpoint.payload`:** read on restore (`cli/src/session/restore_compaction.rs:21,55`, `session/compaction.rs:170`) and by replay (`core/src/persistence/session_replay.rs:52`).
- **`source.prefix` and `verified_suffix`:** the lineage check `lineage.starts_with(prefix ++ suffix)` and the turn counts (`turns_summarized` / `turns_kept` in `cli/src/compaction_publication.rs`, `agent-loop/src/turn_compaction.rs`, `sessions/src/committed_publication.rs`).
- **`identity`, `source.generation`, `output_reservation` and the rest:** all used by `validate_committed_boundary`.

## Design
- `covered_prefix` stops being a stored field. `CommittedGeneration::covered_prefix()` returns `&self.source.prefix`, and readers use the method.
- **Backward compatibility:** a `#[serde(default, rename = "covered_prefix", skip_serializing)] legacy_covered_prefix: Option<Vec<Uuid>>` field accepts old manifests. `validate_committed_boundary` still rejects one whose legacy value differs from `source.prefix`. The next manifest write drops the duplicate.
- **Out of scope:** clearing the whole commit after publication, because restore and lineage checks need it. Replacing the prefix with a count plus digest is a further option, but it would change those checks, so it is left for a later card.
