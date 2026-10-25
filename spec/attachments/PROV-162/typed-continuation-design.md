# PROV-162 bounded typed protocol continuation proposal

## Existing policy being reconciled

PROV-148 deliberately permits only text output in at most three internal
follow-ups. PROV-149 and later lineage contracts retain typed reasoning and
hosted search. This is a policy reconciliation, not a claim that the original
restriction was an accidental regression. Keep explicit end_turn:false and
interrupted-incomplete eligibility, the three-follow-up cap, ordinary absent/
true fallbacks and immediate client-tool handoff. No new retry policy.

## Proposed explicit replay allowlist

For eligible tool-free protocol continuation only, accept validated known
CodexItem families: assistant messages containing output_text; reasoning with
typed summary_text and/or owned uninterpreted encrypted_content; and completed
web_search_call items only when the existing native capability was negotiated.
Recognized hosted actions are search, open_page and find, with their known
query/queries/sources or URL/pattern fields. Validate required IDs, strings,
arrays and completed hosted status. Preserve accepted native items and their
ordering exactly, including opaque bytes, message phases and call IDs. Validate
through the existing CodexItem representation plus narrow family/field checks;
do not forward arbitrary native families or unknown fields. Reject unsupported
or malformed continuation shapes before another physical send, with a generic
compatibility diagnostic that does not print encrypted content.

Empty terminal output remains an explicit failure, not a retry/empty-success
case. A refusal section anywhere suppresses protocol continuation, even with
end_turn:false; ordinary completed refusal remains visible and settles once by
the existing ordinary-turn path. Incomplete filtering/budget/safety/failure
outcomes never continue, and incomplete client calls are never executable.
This is not permission to retry toward a different answer after a refusal.
Completed native function/custom tool calls still return to the generic loop
with original paired call/result identity; no tool effect is replayed internally.

## Ownership and finite budgets

Append only validated output from this model's current admitted physical
response to its already validated full-current-DAG request. Initial request
construction keeps PROV-149 model/ancestry validation. Every internal physical
send retains generation/AttemptAuthority, full serialized admission, normal
stored OAuth account/lifecycle validation and normal same-account token refresh.
Final checkpoint publication remains only after successful chain settlement;
cancellation, reset/compaction, changed ancestry/model/account or failure cannot
publish stale typed output. Do not introduce another persistent account/session
registry, durable protocol cursor, arbitrary restored blob or delta mode.

Three protocol follow-ups are local to one CompletionModel request. Transport
reconnect retains its existing independent finite policy; task /continue nudges
and genuine-user accounting remain owned by the existing outer loop. Neither
transport recovery nor task recovery replenishes another budget. Clarify cap
diagnostics as protocol exhaustion, distinct from /continue and transport
recovery. No additional automatic continuation, refusal bypass or settings/UI
subsystem is proposed.

Report each validated physical terminal's existing usage once, preserve exact
aggregate/request usage and original request identities, re-admit the full typed
body on every physical follow-up, and never claim a final completion when cap,
admission, cancellation or unsafe terminal validation fails. Preserve existing
shared Responses/Copilot behavior; this change is Codex-scoped.

## Required causal and final-source proof

Real synthetic numeric-loopback transport tests precede production changes.
Cover reasoning plus text and negotiated hosted-search continuations, incomplete
interrupted typed continuation, empty/unknown/malformed output, refusal and all
unsafe terminals, exact cap and usage, serialized admission growth, cancellation
and generation reset before follow-up. Verify next genuine input retains exact
owned typed ancestry while changed model/history remains rejected. Preserve
existing stored-account transport controls and add an actual account-change
between-protocol-requests control without real credentials or issuer calls.
Use the real CLI stream loop to compose protocol follow-ups with native tool/
result IDs and an outer nudge budget; run existing genuine-user/recovery-budget
controls rather than replacing accounting with a parallel model. No live
provider or external issuer tests. Physical tests do not prove live support.
