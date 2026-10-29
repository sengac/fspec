# Compaction recurrence timeline and counting evidence

## Verified result

The retained evidence establishes **five distinct documented live compaction
failures across five sessions**, in two different symptom classes:

- One earlier macOS generation-advancement failure, known here through preserved
  repository receipts rather than a new read of its original machine.
- Four Linux failures containing the exact `assistant, tool result or DAG
  ancestry` diagnostic, verified against local durable terminal events and
  daemon diagnostic lines.

This is a lower bound for the available investigation history, not a total of all
Fspec failures ever experienced. It is **not five independently failed fixes**.
The two prior bug cards are BUG-278 and BUG-282; BUG-282 had an implementation pass
and a later corrective review, with two Done transitions. The review receipt
explicitly says it did not introduce another production fix.

## Chronological observations (all UTC, 2026-10-28)

| Time | Session | Evidence and classification |
| --- | --- | --- |
| 02:29:30.432108 | `ba15d5a4-4ccf-4d70-9ec9-78594797286a` | macOS PID 82606; durable terminal `compaction_failed`, request `33fa99c0-a7ca-4500-ab1f-53325f2e54f1`, request sequence 723. BUG-278 issue records `generation must advance with a selected model`. This is the earlier distinct generation-domain failure. |
| 04:15:57.473 | BUG-278 | Card transitioned to Done. Its controller verification explicitly excludes live execution of the newly built fix: a separately observed native compaction used the unchanged daemon with already aligned generations. |
| 08:40:33.847645151 | `7c55a508-ff74-4a68-895f-c01e9bdbb03c` | Linux terminal `compaction_failed`. Paired daemon lines 346–347 identify exact assistant/tool-result/DAG lineage rejection. Preserved bounded RPC receipt identifies daemon PID 366915, port 32893. |
| 09:30:06.560 | BUG-282 | First Done transition. Later review explicitly calls its negative-coverage/completion claims premature. |
| 09:31:38.256287043 | `c09c6649-9724-42d6-8aa7-005a885b8d27` | Separate `provider_error`, identified by attached bounded RPC receipt as network setup failure. **Not counted as compaction recurrence.** |
| 09:35:42.439 | BUG-282 | Reopened to Validating, then Specifying/Testing/Implementing/Validating for substantive coverage review. |
| 09:52:33.108 | BUG-282 | Second Done transition after review. Review states no new production file changed and no daemon restart occurred. |
| 10:06:14.642602975 | `c09c6649-9724-42d6-8aa7-005a885b8d27` | Linux terminal `compaction_failed`; paired daemon lines 400–401 identify exact assistant/tool-result/DAG lineage rejection. Registered second-Linux-failure receipt explicitly says installed daemon remained unchanged. |
| 19:27:45.375942118 | `4d8eedb6-fe1d-43cb-9aa9-70252fb8cd61` | Linux terminal `compaction_failed`; paired daemon lines 728–729 identify exact assistant/tool-result/DAG lineage rejection, including `terminal compaction failed: compaction commit failed`. |
| 20:22:53.006611441 | `7b41d57c-d7aa-47f3-b2c7-b689012703c4` | Linux terminal **`provider_error`**, but paired daemon lines 1096–1097 identify `compaction candidate rejected: selected request preparation unavailable: RequestError: Codex checkpoint lineage mismatch: assistant, tool result or DAG ancestry`. It is the fourth lineage failure despite the different terminal label. |

The four Linux diagnostics are in
`/home/rquast/.fspec/logs/private/fspec-combined.log.2026-10-28`.
Each event appears twice (WARN error-chain record and ERROR stream record).
The **eight matching log lines represent four occurrences**, not eight failures.
Multiple JSON attachments of the first Linux error share RPC generation
`17e67009-0f44-464a-906f-a89a0e3f5282`, sequence 25283, and therefore are duplicate
receipts for the same occurrence.

## Durable evidence identities

Exact terminal files copied without modification into this directory:

| Session prefix | Request-local sequence | SHA256 |
| --- | --- | --- |
| `7c55a508` | 289 | `8384477db1138a19350544bd8f9422affb2d432187558260995b423dba5021e0` |
| `c09c6649` | 271 | `e5632feae5bb4df64e625826f53298cc1899dcaf67587c79acecf8f98f8c69f2` |
| `4d8eedb6` | 318 | `a0aa243132d515466aa5075f1a458e6cdbbb6ac6139d547c495b1459848a897f` |
| `7b41d57c` | 72 | `67d90ceacd81c0894be0aa4f53c8fd20bccf8604850ed4f6f0bcb61676f8fcb2` |

Filenames also contain a session-global sequence; the table above uses the
stored event's request-local sequence. `noncompleted-terminal-events.json`
retains the full identity object, source path and source SHA256 for disambiguation.
The unrelated c09 network-error terminal is retained as a fifth copied file so
it can be explicitly excluded rather than conflated.

At this read-only capture all four failed Linux manifests had `compaction: null`
and `compaction_commit: null`:

| Session prefix | Manifest revision | Message references |
| --- | --- | --- |
| `7c55a508` | 1464 | 655 |
| `c09c6649` | 1267 | 548 |
| `4d8eedb6` | 1284 | 593 |
| `7b41d57c` | 709 | 314 |

These are capture-time metadata observations, not claims that whole manifests
have never changed since failure. Watcher updates can legitimately change
metadata. Exact capture hashes, timestamps and provider-state references are in
`failed-manifest-metadata.json`.

## Scan scope, exclusions and confidence

The first metadata-only sweep covered 32,568 JSON transcript event files in 25
local session directories. It found 67 terminal events, of which 16 were not
`completed`. No full conversation text was emitted or copied. Only terminal
metadata, exact diagnostic lines and explicitly scoped attachment evidence were
retained. Ongoing sessions may add files after the sweep; this is a snapshot.

A separate diagnostic sweep of the retained combined logs found eight matching
lines for the four Linux events above, and no additional `compaction failed`,
`compaction candidate rejected`, `compaction commit failed`, or generation-must-
advance diagnostics. The older generation event is supported by the macOS
receipt, not invented as an absent Linux log event.

User cancellations, network setup failure, server overload and generic provider
errors are excluded unless an actual compaction diagnostic establishes the
classification. Therefore neither generic terminal labels nor all error lines
are adequate recurrence counters.

The identical lineage error string does **not** establish a shared root cause.
This report does not establish which exact executable handled every historical
failure, what content diverged, whether BUG-282's canonical text fix is present
in the current runtime, or whether any preserved session is repairable. Those
questions require the separate provenance and lineage replay investigations.

## Supporting receipt locations

- `spec/attachments/BUG-278/fspec-codex-compaction-issue.md`
- `spec/attachments/BUG-278/bug278-causal-evidence.md`
- `spec/attachments/BUG-278/bug278-controller-verification.json`
- `spec/attachments/BUG-271/bug271-272-274-final-recovery.json`
- `spec/attachments/BUG-282/native-compaction-failure.json`
- `spec/attachments/BUG-282/failed-compaction-session-evidence.json`
- `spec/attachments/BUG-282/bug282-second-linux-compaction-failure.json`
- `spec/attachments/BUG-282/bug282-review-network-failure.json`

Their capture hashes are in `source-attachment-hashes.json`. Relevant prior card
state histories are frozen in `prior-card-state-history.json`. The original
source data was not changed, retried, resumed, deleted or reset.
