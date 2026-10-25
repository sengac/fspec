# Exact native approval: superseded RPC-039 scenario reconciliation

## Known failed operation and partial state

Native CLI `link-coverage move-background-session-into-codelet-sessions` for
scenario `handle_output uses the new chunks_tx broadcast and no longer touches
GLOBAL_CHUNK_CALLBACK`, new test file
`rust/sessions/tests/background_session_shape/broadcast.rs`, was rejected by
mandatory step validation. The feature still required optional broadcasting and
the global callback remaining, whereas RPC-041 and the original tests require
mandatory broadcasting and the global callback's absence.

The first five independent RPC-039 scenario mappings were already successfully
rebased through native commands. The failed sixth new mapping was not created;
its old mapping was not removed. Remaining cases were not attempted. Exact
commands/status/stdout/stderr are in `mechanical-coverage-rebase.jsonl`.

The operation stopped. No skip flag, validation threshold change, fake step
comment, direct coverage-file edit or blind retry was used. The structural audit
retains all 14 original tests, all 67 assertion predicates and all original step
comments; runtime source-shape controls passed before and after extraction.

## Real request_user_input response

Question ID: `reconcile_superseded_rpc039_steps`.

Question: “Approve replacing RPC-039's superseded optional-broadcast/global-
callback-remains steps with RPC-041's existing mandatory-broadcast/global-
callback-absent contract, preserving all 67 assertions and changing no runtime
policy, so native coverage rebasing can proceed?”

Selected: **Approve exact rebase**.

Exact additional response:

> Supervising caller approves under the human user’s explicit grant to manage
> reasonable in-scope session requests. This is not fabricated new human speech.
> Reconcile only the two superseded RPC-039 scenario steps to the existing
> mandatory-broadcast/global-callback-absent RPC-041 contract; preserve all 67
> assertions, APIs and runtime/security semantics. Record the exact
> reconciliation evidence, then retry the known-not-executed blocked coverage
> mapping once through normal validation. Do not edit coverage files directly,
> relax step validation, repeat a denial without new relevant evidence, or claim
> excluded scenarios ran.

## Authorized material change

Only the two contradictory scenario steps in
`move-background-session-into-codelet-sessions.feature` are reconciled to the
already existing RPC-041 contract. Tests and runtime code are not changed by
this approval. This provides new relevant evidence for one normal-validation
retry of the failed mapping; remaining independent mappings then continue.
No excluded Cargo-subprocess scenario is considered executed or passed.
