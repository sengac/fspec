# BOARD-024 independent preparation audit response

Reviewed /tmp/board024-corrected-independent-audit.md against actual current
source. No production, daemon, tool activation, status, security, failed-history
or compaction changes were made. Earlier proposal-only boundaries remain.

## Already corrected before this review

- canonical.rs attaches the UI fixture temp root with with_cwd before either
  embedded or WebSocket construction; the client cwd remains distinct.
- browser.rs uses the approved feature-local injected launcher and deterministic
  viewer-enabled seam, with an Arc/Mutex recorder. There is no PATH, environment,
  shell or real browser interception. Its assert_served helper performs HTTP GET
  and compares the complete exact viewer rendering for the canonical file.
- Attachment activation no longer configures fake port 8123. That value remains
  only in the preexisting pure URL-encoding assertion, not a success fixture.

Those two findings were stale at the audit's cited source locations, so their
setup was not duplicated or rewritten.

## Remaining selection defect reproduced and fixed

The basename-only/global-first-highlight helper was still present. Extracted
that exact existing helper into board024/selection.rs, added an isolated
board024_selection_harness target and ran it before changing behavior:
**0 passed, 5 failed**. The failures prove duplicate-basename confusion,
unrelated chrome coordinates, separated highlight concatenation, wrapped-path
coordinates and internal-whitespace normalization bugs.

The corrected helper matches the complete original label rather than a basename,
retains internal spaces and literal percent text, joins only adjacent aligned
highlight runs for wrapping, handles wide Unicode cells, and returns coordinates
from the actual matched target glyph. It does not normalize action identifiers
or attachment paths. Empty or incomplete targets fail closed.

Added a sixth soft-wrap regression. The final exact shared helper is executed by
the isolated target and reused by the actual BOARD UI integration tests.

Actual final receipts:

- board024-selection-harness-red.log: 0/5, causal helper assertions fail.
- board024-selection-harness-green-verified.log: **6 passed, 0 failed**.
- board024-selection-harness-clippy-final.log: scoped Clippy with -D warnings,
  **exit 0**. Removed an expect call exposed by the strict test-target lint;
  no lint exemption was added.
- Scoped rustfmt checks and git diff --check pass. Changed maintained source
  files remain below 300 lines; maximum is support.rs at 253.

An intermediate compile error while placing the helper comparison block and the
initial expect-used Clippy rejection are not claimed as causal acceptance RED
or GREEN. The final receipts above are after those explicit corrections.

## Attachment acceptance proof strengthened further

The existing scenario now activates four exact stored paths across both real
transports and each supported/unavailable/launch-failure outcome:

- The original A/B duplicate Unicode, space, hash and percent basenames.
- literal%20name.md and literal name.md, with distinct canonical contents and
  distinct expected encoded URL tails; the literal percent sequence is not
  treated as an already-decoded filename.

Each fixture also starts a real legacy viewer rooted at a different temp
directory containing decoy files at the same paths. The App's legacy viewer port
points at that wrong root. Successful feature launches must use a different
real port and serve the exact canonical-root rendering through HTTP. This makes
silent legacy/client-root reuse fail even if its endpoint is available. Viewer
unavailability still must invoke no launcher despite the live decoy endpoint.
The decoy lease is stopped at the end of each case.

Existing exact OpenAttachment path, URL encoding, launch multiplicity, precise
visible failure, retained selection, layer/context and repeated-activation
assertions remain. No production viewer or path policy was modified.

## Native mapping readback

Refreshed only the three affected attachment scenario test ranges using three
native link-coverage/unlink-coverage replacement pairs with ordinary mandatory
step validation. Each scenario still has one mapping and its unchanged empty
implementation array:

- Exact paths/unavailable viewer: attachments.rs:35-154.
- Missing/directory/rejected targets: attachments.rs:156-189.
- Sanitization/original actions: attachments.rs:191-224.

All 17 original scenario identities remain. No skipped validation, global
generation, direct sidecar writes or artificial step comments were used.

## Remaining RED boundary

board024-independent-audit-causal-red.log is the actual post-correction UI
target run. It still fails compilation on the intentionally missing production
BoardCardDetailsLoaded action and the two approved feature-controller test
seams. The str inference diagnostic is downstream of that missing action.
There are no additional attachment setup/name-resolution errors in that run.

Only the selection harness is GREEN. The rooted-viewer/percent-path assertions
and the deeper BOARD canonical, generation, scrolling and Mux proofs must still
execute after authorized production implementation. No feature GREEN or
canonical Done is claimed; REFAC remains validating and BOARD remains testing.
