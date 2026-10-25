# BUG-261: local-only transport blocker

Historical blocker recorded before the approved local-only seam and repair.
See `validation-and-review.md` for the final outcome; this original incident
record remains preserved and is not valid defect-red evidence.

The first new integration fixture attempted a full OAuth Codex model against a
local `/responses` MockServer. This was **not a valid red reproduction**:
`prepare_oauth_request` calls `rewrite_codex_url`, which rewrites every bare
`/responses` URL, including localhost, to the production Codex endpoint.
The local inference server received zero requests. A recovery-state assertion
observed an inference setup deadline rather than the required post-401 issuer
failure. The three failures in `red-ci-test.log` are fixture failures, not causal
evidence for the card's defect.

The test path was stopped after inspecting that result. The newly added unsafe
test file was removed so ordinary test discovery cannot repeat that unintended
external routing. The requests used synthetic access/refresh values and a 50 ms
setup deadline. The evidence does **not** establish whether the attempted
external connection reached the remote service or transmitted an HTTP request;
no successful inference, billing, or use of real credentials is claimed.

Existing `prov154_codex_recovery` tests safely use a direct client request to
`/recovery`, which is not rewritten. That can prove typed forced-refresh causes,
but cannot by itself prove the full Codex model's retry/cancellation composition.
The model hardcodes `/responses`, and its client type is not transport-generic.

## Approval needed

Approve a narrow, test-support-only injectable local transport/endpoint seam
that leaves shipping OAuth endpoint policy and credential binding unchanged,
then add a transport-isolation regression before retrying the full model test.
Alternatively approve a production transport abstraction after design review.
Do not relax endpoint or credential security policy, use actual providers, or
claim helper-only coverage as end-to-end recovery proof.

Current card remains incomplete. The feature and discovery data are preserved;
the original audit's forced-refresh error-erasure finding still needs a correct
local-only failing regression before implementation. Independent selected cards
can continue without this seam.
