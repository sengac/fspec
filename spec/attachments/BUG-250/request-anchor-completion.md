# Causal request anchor completion

The real BackgroundOutput regression in `transcript-request-anchors.feature`
failed before this repair: consecutive provider failures without assistant
originals made canonical export return `interior transcript request has no
original causal anchor`. Evidence: `/tmp/bug250-request-anchors-red.log`.

The approved durable request boundary is metadata on the first authentic delta:
`JournalRecord.request_anchor = { original_count, preceding_message_id }`.
It is captured from the authoritative committed manifest before admission,
publication or tool dispatch. It adds no extra record, admission slot, displayed
message, retry store or model original. A separately journaled control event was
discarded during development because it would consume bounded recovery capacity
before retaining the actual delta; no such event remains in the implementation.

Canonical archive projection verifies the prefix length and final original
identity, rejects messages predating their request boundary, and merges safe
terminal records at causal boundaries without timestamp sorting. Existing
historical records retain the established strict compatibility policy: malformed
explicit boundaries never become inferred coordinates. Boundaries cannot be
replaced by later deltas. No replay of tool effects or provider requests occurs.

Executable metadata RED is `/tmp/bug250-request-metadata-red.log` (two runtime
decode failures before the contract existed). The later immutability scenario
has its own assertion RED in `/tmp/bug250-request-metadata-immutability-red.log`
(changed boundary incorrectly admitted; two other scenarios already green).

`/tmp/bug250-request-metadata-final-green.log` passes all twelve focused core
scenarios: three new boundary scenarios, two causal export/pinned snapshot
regressions, two journal integrity scenarios, and five admission/lifecycle
scenarios. These compile the actual complete core source and actual integration
tests with matching existing dependencies. They supplement, not replace,
canonical Cargo validation.

The canonical source/caller command is
`/tmp/bug246-250-final-integrated-cargo.log`; final caller runtime evidence is
`/tmp/bug250-request-caller-final-green.log`. Queued and in-flight compile-blocked
commands are not claimed as passes. The unrelated in-flight archival replay
return-type error in earlier combined attempts belongs to BUG-245's ongoing
repair and is not behavioral evidence for this card.

Existing mixed-target features were split without changing their acceptance
criteria: `causal-transcript-archive.feature` owns core export scenarios and
`transcript-recovery-retirement.feature` owns agent-loop recovery scenarios.
The original cold-display and canonical lifecycle features now each map to one
test file. Coverage is transferred using fspec commands and unique mappings;
no temporal or workflow checks are bypassed. No commits or live-store mutations.
