# Earlier Linux failures: two distinct data divergences

## Result

Preserved and rehydrated real state shows BUG-282's exact removed text transform
in **both earlier Linux failures**, while the first session also contains a
second discrepancy tied to cancellation:

| Session | Prior/current tool outcomes | Mismatching outcomes | Missing outcomes | Additional mismatch after canonical-output counterfactual |
| --- | --- | --- | --- | --- |
| `7c55a508-ff74-4a68-895f-c01e9bdbb03c` | 318 / 319 | 233 | 0 | Nine user originals absent from the checkpoint, persisted immediately after cancellation |
| `c09c6649-9724-42d6-8aa7-005a885b8d27` | 270 / 271 | 165 | 0 | None in this offline projection comparison |

In both unmodified reconstructions the first projected mismatch is index **3**,
with expected and actual `function_call_output` carrying the same call identity
but different output text. This is real preserved evidence, not a synthetic
payload chosen to exercise escaping.

## Exact transform relationship

For all **398** mismatching outcomes, the persisted current output equals the
checkpoint output after applying the exact legacy code sequence from
`/tmp/fspec-bug282-evidence/pre-fix-stream-handlers.rs:203–214`:

1. Strip the outer pair of double quotes if present.
2. Replace literal backslash-n with newline.
3. Replace literal backslash-t with tab.
4. Replace literal backslash-r with carriage return.
5. Replace backslash-double-quote with double quote.
6. Replace double-backslash with backslash.

There are no mismatches that require an additional guessed transform.

- For `7c55a508`, 112 cases also match without stripping the outer quotes; 121
  require the outer-quote stripping step.
- For `c09c6649`, 111 also match without stripping; 54 require it.

This establishes the data-level relationship described by BUG-282 on these real
historical failures. It does not establish source-to-binary provenance by itself.
The separate runtime provenance report owns that claim.

First outcome text fingerprints:

| Session | Call ID | Checkpoint text length/hash | Persisted text length/hash |
| --- | --- | --- | --- |
| `7c55a508` | `call_32853ca4f07448e399eb52c7b7fb8e52` | 30068 / `587baad365f1554b49fe518e58861fc7ec455c13c651b47f3cd5ac7d276e5798` | 29165 / `1807358d48d82a6cd157424bea0d0a3a17558ab7e38a7cb622af933b27db1331` |
| `c09c6649` | `call_b11571b86302444d94c91a6641335e17` | 2844 / `32e2ea38f9befdad4e348792cd400bb7b846af3c85547c0807ab8112bb48f7c1` | 2797 / `17c478a6ef6b52f1af94de495adc52ffde23994d417e59b46d72fff3434b170b` |

The full fingerprint list (no raw payloads) is `earlier-comparison.json`.

## Second divergence hidden behind the first in 7c55a508

Replacing only the differing outcome texts in a **cloned offline projection**
with their corresponding preserved checkpoint text lets the comparison advance
from index 3 to index **742**, where expected and current are different user
messages:

- Expected user text: 3387 characters; SHA256
  `74a3ef7f03b95b8d0deccec7621c9037882bb86c04af6813c1ee7952045c72e5`.
- First additional current user text: 411 characters; SHA256
  `9a4ddebf4992ce69de06548a8fcb5b4ce4ffffb4a7a645a6822ffc801b93530f`.
- The expected user message exists later in current input; the extra current
  message does not exist in the checkpoint's projected history.

There are **nine** such additional user originals. Their creation times run from
`2026-10-28T08:23:17.838661393Z` through
`2026-10-28T08:23:17.891607936Z`, immediately following the durable
`user_cancelled` terminal at `2026-10-28T08:23:17.832512369Z`.
Their stored `metadata.requestId` values are null.

Omitting exactly those nine messages **only from the cloned projection**, after
canonicalizing the 233 historical outcome texts, makes this Python comparison
pass. The exact original IDs, timestamps and fingerprints are in
`earlier-compound-counterfactual.json`.

These nine do not begin with `[SUPERVISOR:]`, so a check limited to that newer
prefix would not identify them. This report classifies them conservatively as
additional user originals associated in time with cancellation. It does not
infer their submission source from their text or claim timing alone proves the
entire upstream control-flow cause.

For `c09c6649`, substituting the 165 canonical outcome texts in the cloned
projection is sufficient for this comparison to pass. No additional user-message
omission is needed.

## Method and bounds

The two source manifests, all provider-state JSON files for their session IDs,
and exactly the manifest-referenced originals were copied under the private
`/tmp/compaction-investigation/earlier-private/` directory. Each required blob was
read and verified against its SHA256-addressed identity before rehydration. The
source-to-hash preservation manifest is
`earlier-private/preservation-manifest.json`.

The selected checkpoint references are:

- `7c55a508`: `d2e43e1a8eb3d4e70cd743448180f9df25238138d5f282774491eb193339ed02`
- `c09c6649`: `88e6708b1fa817c94a1c5d2a9a08591826b0c95707ab85a4a038680daa253fae`

The analysis imports the existing root diagnostic `replay_lineage.py` with
`ROOT` changed to this separate private directory. The diagnostic mirrors the
known projection and queue matching algorithm; **these additional earlier-session
counterfactuals have not yet been run through the actual Rust CodexHistory API**.
They are data comparisons, not a production repair or recommendation to delete
user messages or replace live historical outcomes. The parent has separately
used the production API for the two later sessions.

No original session, checkpoint, blob or message store was modified. No sessions
were resumed or retried. No builds, daemon operations, credentials, or denied
paths were used. The preserved raw evidence is private and should not be attached
as a user-facing payload without review; the report and fingerprint JSON contain
no conversation text.
