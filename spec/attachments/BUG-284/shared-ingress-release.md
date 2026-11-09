# BUG-284 shared ingress ownership release

The parent controller confirmed release of all 13 targets enumerated in
`approved-ingress-review.md`: no active REFAC-020 overlap, BOARD production not
started. Before-edit bytes, hashes and passing REFAC-019 evidence are frozen at
`/tmp/refac019-pre-bug284-shared-boundary/manifest.json`. All 13 current source
hashes matched that released baseline when independently checked.

Proceed with the scoped awaitable durable-submission bridge and ordinary-input
`Result<(), String>` error propagation. Preserve method order and request
payloads. Supervisor result shape already exists. No external idempotency-key
or broader payload redesign is required. Keep truthful missing-session/manager,
storage and queue refusal observable by the actual caller.

Keep the original frozen baseline and logs as prior-refactor evidence. Record
BUG-284's intentional semantic and return-type changes separately. Add relevant
caller compatibility/error tests; do not relabel these changes as mechanical
REFAC-019 preservation or refresh frozen evidence merely to make checks pass.

Return a stable shared-file handoff with exact current hashes, changed signatures
and passing caller checks so BOARD can integrate. Other concurrent files remain
protected. Existing approval restrictions, strict invariants, original-history
preservation and no install/restart/commit/push rules continue to apply.

This release removes the ownership hold on those 13 files; it does not claim
implementation or validation has occurred, grant permission for unrelated
changes, or override any future action-specific denial.
