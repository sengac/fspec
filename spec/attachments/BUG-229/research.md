# BUG-229: Compactor exploration runs out of admitted request capacity

## Incident and confidence

- Screenshot: `/Users/rquast/Desktop/Screenshot 2026-10-21 at 3.32.33 pm.png`.
- A copy of the screenshot is registered on BUG-229 alongside this research attachment.
- Session: `c655c22f-b638-4189-890a-616fb9227f99`.
- Local log: `~/.fspec/logs/fspec-combined.log.2026-10-21:8648`.
- Compaction started at 05:30:20 UTC and failed at 05:32:14 UTC on 2026-10-21.
- Runtime error and the corresponding source recovery gap are confirmed. Source baseline is `7e60de8e`; the exact running executable revision was not established.

> terminal compaction failed: compaction builder failed: Compactor sub-agent failed: RequestError: complete request input 947739 plus output reservation 128000 exceeds capacity 1050000

The reservation is present and positive. Available input budget is 922,000; estimated input occupancy is 947,739, exceeding that budget by 25,739. This is not BUG-215's missing-output-metadata failure and resolving optional metadata alone cannot fix it.

Admission currently estimates non-media occupancy conservatively from serialized bytes, plus known media reservations. The reported input is not measured model-token usage. Do not weaken that safety contract merely to make this example pass.

## Source chain

1. `rust/cli/src/compactor_sub_agent.rs:165` invokes the registered builder and returns its failure before committing a candidate.
2. `rust/agent-loop/src/compaction_builder.rs:15` bounds exploration to 20 rounds, with seven tools and captured originals. There is no occupancy-aware evidence/synthesis runway in this builder.
3. `rust/agent-loop/src/compaction_execution.rs:48` sends only `MaxDepthError` into explicit tool-free synthesis. Other request failures become `Compactor sub-agent failed`, matching the screenshot. This points to exploration or an internal physical continuation, not the separate final-synthesis admission branch.
4. `rust/providers/src/codex/streaming.rs:107` admits each complete physical request before transport, including continuations.
5. `rust/providers/src/codex/physical_admission.rs:4` preserves admission failures as typed `RequestError` sources.
6. `rust/providers/src/request_admission.rs:161` rejects occupancy beyond capacity minus the reserved output allowance.

The exact offending evidence page, request ordinal, and proportion of evidence versus replayed output are unconfirmed. The session retained its earlier generation 1, committed at 05:07:56 UTC; this failure did not install a newer compaction.

## Acceptance and regression scope

- With valid explicit limits, both registered recovery and GenerateCompaction must budget growing evidence and synthesis overhead before request capacity is exhausted; depth alone is not a sufficient bound.
- Use real persisted originals and local HTTP fixtures, including Codex physical continuation growth. Distinguish capacity exhaustion from ordinary depth exhaustion and provider/network errors.
- With capacity 1,050,000 and reservation 128,000, complete estimated input 922,000 fits and 922,001 does not. Rejected bytes must never reach transport.
- Preserve gathered final-tool evidence and full authoritative source coverage. If a single evidence-bearing synthesis cannot fit, use bounded evidence partitioning or fail explicitly without pretending omitted evidence was summarized.
- Keep request allowance and reservation consistent; do not bypass admission, invent capacity, or lower reservation without enforcing the corresponding output limit.
- Preserve cancellation, DAG validation, durable commit/publication gates, and the prior durable generation when recovery is impossible.

## Related work and validation boundary

- BUG-215 resolves absent optional limits through existing effective provider defaults; it is independently actionable.
- BUG-211 and `spec/features/compaction-builder-budget-recovery.feature` provide depth-exhaustion synthesis, not occupancy-aware exploration.
- `spec/features/compaction-synthesis-admission.feature` already requires genuinely oversized synthesis to remain fail-closed; retain that regression.
- Investigation was read-only source/log/session analysis. BUG-229 is in specifying with rules and examples captured; no new reproduction test or implementation has been run.
