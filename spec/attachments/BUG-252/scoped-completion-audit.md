# BUG-251 / BUG-252 scoped completion audit

## Audit scope

Read all four BUG-249/250/251/252 cards, repair plans, earlier implementation and
finish evidence, associated features, media/discovery integration tests, accepted
input persistence, provider discovery, session restoration, transcript-only
decoder, transcript archive and paging implementation. Preserve the shared dirty
tree and prior executable RED evidence. No production code changed in this audit;
no independent defect was established in BUG-251/252's owned implementation.

The BUG-251 converter retains ordered accepted images and supported stored
PDF/URL representations. URL sources are not fetched during replay. Invalid MIME,
base64, credentials in media URLs, unavailable blob sources and oversized image
metadata fail explicitly. Structured input uses the shared recoverable persistence
writer and existing content-addressed blob store, not a separate append path.

BUG-252 discovery keeps populated, empty, unsupported, authentication, HTTP,
protocol, transport/timeout and skipped-runtime states distinct. Only supported,
empty and unsupported discovery avoid the unavailable flag. Discovery never
calls inference, creates models or emits endpoint/profile/credential/payload
diagnostics. Existing explicit model IDs remain selectable under unsupported
discovery; other failures remain blocked.

## Completed quality pipeline

The previously pending pipeline actually completed successfully. Durable copies
of all three logs are attached to both cards:

| Command suffix, existing release-slim profile with two jobs | Result |
| --- | --- |
| `cargo check -p codelet-sessions -p codelet-providers --lib` | PASS |
| `cargo clippy -p codelet-sessions -p codelet-providers --lib --no-deps -- -D warnings` | PASS |
| `cargo clippy -p codelet-sessions --test bug251_multimodal_archives --test bug252_discovery_classification --no-deps -- -D warnings` | PASS |

Logs: `bug251-252-finish-check.log`, `bug251-252-finish-clippy.log`,
`bug251-252-finish-test-clippy.log`. These are executed outcomes, not inferred from
an empty log or a still-running command. The final test-target Clippy completed
after the owned helpers and tests were last modified.

Focused Rust formatting and `git diff --check` pass. Feature syntax and registered
tags pass for faithful-multimodal-archives, profile-discovery-classification,
transcript-only-replay and cold-transcript-terminal-replay. All fourteen scenarios
have exact single-test mappings, complete matching step annotations and in-bounds
implementation anchors. This is traceability, not measured runtime coverage.

Refreshed BUG-251 media/text decoder anchors after the transcript-only hook moved
their lines. Replaced BUG-250's broad whole-decoder mappings with the actual
display-only hook, outcome decoder and original-metadata archive entries. No
feature or test assertion was weakened.

## Current targeted execution

Run from `rust/`, without profile/config changes, artifact deletion or alternate
target directories:

```sh
cargo test -p codelet-sessions --test bug251_multimodal_archives \
  --test bug252_discovery_classification --test bug218_optional_model_discovery \
  --test prov131_local_profile_probe_resilience --test bug250_transcript_replay \
  --test bug250_cold_terminal_display --profile release-slim -j 2 \
  --no-fail-fast -- --nocapture
cargo test -p codelet-providers --test bug218_model_discovery_errors \
  --profile release-slim -j 2
```

Logs: `/tmp/bug251-252-scoped-final-green.log` and
`/tmp/bug252-scoped-final-provider.log`. Both are waiting on the shared artifact
lock at this checkpoint. Pending executions are not passing evidence. An earlier
queued sessions invocation was interrupted before compilation only to add
`--no-fail-fast`. One mistyped provider target failed target selection before
compilation; it was corrected to the existing target above. Neither event is
behavioral RED.

## BUG-249/250 coordination

No archival replay/media or core archival code is independently changed here.
`spec/attachments/BUG-250/sessions-finish-coordination.md` records the remaining
journal-source integration boundary for the parent and BUG-245 owner. The existing
display-only decoder is connected, but the current core archival enumeration is
manifest-only. BUG-249/250 status decisions remain with the parent. Both BUG-251
and BUG-252 remain validating until final behavioral results are inspected.

At handoff, the current sessions Cargo process is PID `41580`, queued for more
than 25 minutes; the provider regression process is PID `39710`, queued for more
than 29 minutes. Both logs still contain only the shared-artifact-lock wait.
The active preceding build is PID `21804`; its release test link remains active.
Do not duplicate these queued executions or claim they passed. The existing
`/tmp/bug249-250-clippy.log` has also completed the core/agent-loop/sessions strict
library Clippy pipeline successfully.

The earlier actual Cargo cold-display regression
`/tmp/bug250-cold-display-cargo-red.log` is now durably attached to BUG-250: zero
terminal failures were restored where exactly one was required. This is genuine
behavioral RED, not the pending run or a source-only inference. Parent action is
needed at the canonical archival/journal source boundary before that card can
truthfully finish. No commits or done transitions were made.

Subsequently parent accepted the original BUG-251/252 GREEN and strict-quality
evidence for the unchanged owned sources and owns their final status transitions. The
queued duplicate reruns (PIDs 41580 and 39710) were interrupted before compilation
to reduce the shared queue while implementing BUG-250. They did not fail a test.
BUG-250 now has a focused cold-display/replay run and actual canonical production
repair described in its `journal-archive-repair.md` attachment.
