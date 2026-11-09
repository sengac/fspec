# BUG-284: recurring Codex compaction failures

## Purpose and scope

Investigate recurring live failures after BUG-278 and BUG-282 without assuming
they share one cause. Establish what actually failed, whether the runtime has
the prior fixes, and where checkpoint/request lineage first diverges. Production
repair is not authorized by this investigation; no history reset, lineage bypass,
daemon restart/install, credential change, commit or push is permitted.

## Required evidence before completion

1. Preserve failure receipts, relevant manifests/checkpoints/originals and hashes.
2. Identify running executable path, hash, build identity and source provenance;
   distinguish source fixes from code proven present in that executable.
3. Count distinct fixes/reviews separately from distinct observed live failures.
4. Compare checkpoint and reconstructed selected request at the first mismatch;
   trace relevant streaming, persistence, restore, concurrency and compaction paths.
5. Replay locally preserved real failure evidence in isolated diagnostic fixtures.
6. Audit prior tests and state their limits without retroactively changing receipts.
7. Attach causal findings, reproduction steps, unresolved hypotheses and explicit
   repeated live compaction/restore/continuation verification criteria.

## Current execution

Direct native MCP does not expose card creation; the supported Fspec CLI created
this separate card. A new native session was capacity-rejected, so the parent
withdrew that orchestration requirement. No existing native session is being
reused or closed. Read-only investigation and isolated local diagnostics continue.

The card must remain open while required evidence is missing. Unit-test success
cannot establish that the user's live recurrence is fixed.
