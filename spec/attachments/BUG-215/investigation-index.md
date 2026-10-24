# Log and screenshot investigation: card index

## Scope

Research date: 2026-10-21. Source baseline: `7e60de8e`.

The original log audit covered 02:48:19–04:48:19 UTC (12:48:19–14:48:19 local, UTC+10). Follow-up source inspection and persisted-data correlation investigated the two Desktop screenshots from 13:00:35 and 13:11:02 local time. The source baseline does not establish the exact build of the running executable.

The original audit created backlog items, not implementation claims. BUG-215 subsequently completed test-first implementation and scoped validation; BUG-229 is specifying the later 15:32:33 local screenshot incident. Each card has its own registered Markdown research attachment. Related mechanisms are grouped where they share a fix boundary; independently actionable parser, status, state and test defects have separate cards.

## Cards and research

| Card | Finding | Evidence classification | Research |
| --- | --- | --- | --- |
| BUG-215 | Missing explicit compaction output reservation | Runtime failure and source mismatch confirmed | [Research](research.md) |
| BUG-216 | Loop-detector chunking, threshold and repeated-escalation false positives | Source defects; matching logged abort and persisted-delta replay | [Research](../BUG-216/research.md) |
| BUG-217 | Subordinate forwarding loses skipped broadcast chunks | Runtime lag counts and no replay confirmed | [Research](../BUG-217/research.md) |
| BUG-218 | RunPod discovery 404 marks profile unreachable | Repeated failure confirmed; endpoint/root cause unresolved | [Research](../BUG-218/research.md) |
| BUG-219 | Codex reports reconnection cancellation for ordinary interrupted streaming | Source-confirmed status defect; screenshot timeline matched | [Research](../BUG-219/research.md) |
| BUG-220 | Internal abort loses its interruption cause | Source-confirmed event/classification limitation | [Research](../BUG-220/research.md) |
| BUG-221 | Thinking-only exhaustion bypasses recovery and auto-continue | Conditional source gap confirmed; screenshot finish reason inferred | [Research](../BUG-221/research.md) |
| BUG-222 | Thinking retry claims settings were reduced without rebuilding the agent | Conditional source-confirmed recovery defect | [Research](../BUG-222/research.md) |
| BUG-223 | Gemini workaround bypasses shared completion handling | Conditional source-confirmed control-flow defect | [Research](../BUG-223/research.md) |
| BUG-224 | Gemini skips contentless terminal metadata | Conditional source-confirmed parser defect | [Research](../BUG-224/research.md) |
| BUG-225 | Auto-continue TUI cache lacks initial hydration and failure reconciliation | Source-confirmed cache/state defects | [Research](../BUG-225/research.md) |
| BUG-226 | Rejected model switches leave stale selected-model identity | Conditional source-confirmed TUI/legacy metadata defects | [Research](../BUG-226/research.md) |
| BUG-227 | Rhai SSE errors are swallowed and may look like clean completion | Conditional source defect; no matched screenshot incident | [Research](../BUG-227/research.md) |
| BUG-228 | CONT-009 source-shape test assumes removed standalone NAPI implementation | Static architectural mismatch; test not executed in audit | [Research](../BUG-228/research.md) |
| BUG-229 | Compactor exploration exhausts admitted capacity before safe synthesis | Later runtime failure and source recovery gap confirmed; precise request contributors unresolved | [Research](../BUG-229/research.md) |
| CONT-010 | Clarify auto-continue defaults on switch, resume, fork and active-stream changes | Discovery task; current creation-only contract is not itself a bug | [Research](../CONT-010/research.md) |

## Important incident distinctions

- The Codex screenshot corresponds to an internally initiated loop abort followed by a bounded new recovery turn. Its reconnection banner does not prove a network outage.
- The Qwen screenshot matches a thinking-only response ending at 4,096 output tokens without numerical reasoning usage. Its wire finish reason was not preserved in the inspected record; length termination is a leading explanation, not a fact to bake into an incident assertion.
- The same Qwen session's compaction admission failure occurred later, at 03:49:37 UTC. It must not be presented as the cause of the earlier screenshot.
- The later Codex compaction screenshot at 15:32:33 local is a different admission failure: 947,739 estimated input plus 128,000 reserved output exceeds capacity 1,050,000. Its limits are present; BUG-229 tracks exploration capacity/runway separately from BUG-215's absent reservation.
- Correctly seeded backend auto-continue can be hidden by a stale TUI cache. Separately, switching an already-existing session to a profile does not reseed it under the current contract.
- A displayed 0/300 counter is zero outstanding zero-progress nudges, not a count of all requests or a promise to retry every stop reason.

## Findings that do not need additional bug cards

- Zero reported token usage already falls back to estimation; the nine observed warnings describe mitigation, not an unhandled fatal error.
- An already-set compaction-needed flag is diagnostic and does not prevent recovery.
- The invalid overlapping-range DAG candidate was rejected, corrected and subsequently committed for its own session. That is handled validation/retry, not evidence of unresolved failure there.
- Corrected SessionSearch range/budget requests are tool-input recovery, not separate product bugs.
- The old independent-NAPI completion-sync defect is not present at this source baseline; NAPI now exports the shared engine. The obsolete test assumption is separately tracked as BUG-228.
- Interrupts, stalls, terminal API errors, accepted completion and exhausted budgets intentionally do not trigger ordinary auto-continue. Do not remove those safety gates to make the indicator appear active.

## Validation boundary

The audit used local logs, persisted records, source/history inspection and a read-only delta replay. It did not execute the proposed new regressions, contact live endpoints, or apply implementation fixes. Card creation adds planning metadata and research only. Future fixes must follow feature-first, failing-test-first ACDD and scoped validation.

Subsequent BUG-215 development followed feature-first, failing-test-first ACDD and now passes its four scenario regressions and 15 related regression tests. See its [implementation evidence](research.md#implementation-and-regression-evidence) for validation details. BUG-229 records the later compaction screenshot, with rules, examples and the screenshot attached; its occupancy/runway fix has not been implemented.
