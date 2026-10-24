# Pending recovery must not resurrect a deleted transcript

The independent finish audit found an additional cross-card lifecycle gap.
Canonical core deletion now removes the journal before removing its manifest,
but the agent-loop recovery spool can still retain transcript records on disk
and in `persistence_retry::OWNERS`. The transcript branch of
`persistence_retry_commit::commit` appends directly to `JournalStore` without
checking or acquiring the owning session's lifecycle boundary. An explicit retry
after deletion can therefore recreate the deleted private transcript directory.

This is a source-confirmed integration finding, not yet executable RED. It is
not counted as completion evidence. BUG-246 owns spool/caller retention; core
owns the canonical deletion boundary. Coordinate the repair rather than adding
an unrelated retry store or silently discarding live-session content.

Required acceptance: deletion and recovery have one coherent session lifecycle;
deleted sessions cannot regain private journal records from retained recovery;
failure leaves a visible retryable state; existing unrelated sessions and
conversation originals remain unaffected. Add feature-first executable tests
covering durable pending records and caller-retained ownership before production
changes. In particular, a check-then-append existence test alone does not prove
race safety with concurrent deletion.
