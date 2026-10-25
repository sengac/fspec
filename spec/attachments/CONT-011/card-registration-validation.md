# Card and attachment registration validation

Created from the auto-continue audit on 2026-10-25:

- CONT-011 coordinating bug, ten child bugs BUG-260 through BUG-269, and
  PROV-162 discovery story: twelve new cards, all in backlog.
- CONT-010 reused for existing lifecycle discovery; related to CONT-011 and
  annotated with PROV-161's newer inherited-settings contract.
- Full audit, follow-up index and audit test evidence registered on CONT-011.
- Every fix/discovery card has focused Markdown research with source evidence,
  confidence, history, constraints and test direction.
- Attachment copies compared byte-for-byte with their prepared sources.
- Scoped JSON/filesystem checks confirm each new card is listed once in backlog,
  each registered attachment exists and is nonempty, ten parent/child edges are
  reciprocal, and discovery relationships are present.
- `git diff --check` passed.

## Global-validator limitation

`fspec validate-work-units` was executed and returned 19,646 findings across
older cards, beginning with HOOK-001 child/parent mismatches and including
rules/examples shape complaints on existing work units. None of its findings
name CONT-011, PROV-162 or BUG-260 through BUG-269. These unrelated existing
issues were not repaired as part of creating the requested cards.

No production implementation changes or new passing-fix claims are implied by
this registration step. The earlier audit's executed tests and reproduction
are documented separately in [validation-evidence.md](validation-evidence.md).
