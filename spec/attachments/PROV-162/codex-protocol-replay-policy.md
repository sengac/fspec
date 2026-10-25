# PROV-162: Codex protocol continuation and typed-history policy

## Why this is discovery, not a confirmed accidental bug

PROV-148 explicitly specifies a text-only bounded continuation policy. PROV-149
and later lineage work preserve richer typed response items. The interaction
can stop real reasoning-rich responses, but the approved contract must be
reconciled before changing policy or treating every restriction as regression.

## Source evidence

- `rust/providers/src/codex/lifecycle.rs:82`: selected completed end_turn:false
  and interrupted incomplete responses continue when no client tool was seen.
- Line 123 rejects empty output or anything other than text-only assistant items.
- `rust/providers/src/codex/streaming.rs:57`: three protocol follow-ups maximum.
- Line 204 replays before returning successful Final to the outer engine.
- `spec/features/codex-terminal-lifecycle.feature:10`: current policy.
- PROV-149 and CMPCT-078: typed output/opaque reasoning and full lineage contracts.

The outer /continue budget does not override this provider-level cap, and errors
here bypass ordinary clean-stop nudging. Provider behavior introduced in `2699693b`.

## Questions requiring explicit decisions

- Which typed reasoning and hosted-search items may be replayed safely?
- Should empty output be an explicit terminal failure or a bounded retry case?
- How should protocol follow-up limits relate to task-level zero-progress budgets?
- How should the UI explain protocol exhaustion versus /continue exhaustion?
- How are refusal and content-filter outcomes preserved without treating them as
  an instruction to bypass safety boundaries?

## Acceptance/test direction after decisions

Define the approved policy first, then real transport tests for all terminal
states and mixed item families. Assert exact opaque lineage, full physical
request admission, usage settlement, cancellation and finite recovery bounds.
Avoid relaxing native history validation or draining trailing executable tools.

This is distinct from reproduced refusal projection BUG-260 and profile-default
lifecycle discovery CONT-010. See
[full audit](../CONT-011/fspec-auto-continue-investigation.md).
