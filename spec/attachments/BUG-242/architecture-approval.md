# Shared architecture approval: BUG-242 through BUG-252

On 2026-10-23 the user selected **Approve full repair** in response to this question:

> May I proceed with bounded architectural changes for separate versioned provider-state
> storage, cursor-based archival replay, and incremental recovery journaling, while
> preserving existing conversation data and model-context behavior?

This authorizes the bounded shared storage/replay changes needed for all eleven cards,
with migration and regression coverage. It is not approval to discard historical data,
change the hierarchical DAG model, weaken provider lineage/admission checks, fabricate
tool success/cancellation, rerun unknown side effects, or bypass any ACDD gate.

Owners must coordinate shared schemas and APIs. Preserve legacy original coordinates,
native provider identities, authoritative committed generations and reduced model context.
Keep code in focused modules; no unrelated global refactor or configuration rewrite.

For BUG-242, merely moving the existing failure to an earlier typed error does not repair
the normal timeout handoff. Test actual Codex zero-result and partial-batch boundaries,
establish truthful settlement/recovery, and reach coherent synthesis wherever safe.
Genuinely unrecoverable states must remain explicit and fail closed.

The coordinator independently audits the integrated behavior and completion evidence.
Workers leave cards in validating until that audit succeeds.
