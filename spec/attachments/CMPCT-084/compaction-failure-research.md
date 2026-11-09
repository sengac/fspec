# Compaction failures on local OpenAI-compatible models and Codex: research (2026-10-30)

Sources: `~/.fspec/logs/fspec-combined.log.2026-10-25..27` and `~/.fspec/logs/private/fspec-combined.log.2026-10-27..30`, persisted manifests in `~/.fspec/sessions`, and code at commit c81986cf.

## TL;DR

1. **Crux (local qwen on spark/Strata):** the compactor sub-agent almost never produces a DAG that passes validation. On qwen3.8-27b, 34 of 34 compaction operations had rejected candidates, all with `invalid DAG candidate: missing or unclosed dag-node structure`. Each attempt re-runs ~100 s of SessionSearch exploration plus ~90–200 s of synthesis. Two attempts consume the 600 s budget, the third hits `compaction synthesis deadline exceeded`, and the turn ends with `automatic continuation blocked; submit fresh input to retry` (33 such turn errors on 2026-10-29). The supervisor schedule re-prompts every 10 minutes, so the cycle repeats all day.
2. **Most likely cause of the invalid candidates (strongly inferred, not yet observed directly):** the compactor runs with `max_tokens = min(8192, session output_reservation)`, which is **4096** for these profiles. Server-side thinking stays on, and synthesis keeps only `Text` chunks while ignoring reasoning and the stop reason. A thinking model that reaches the cap mid-thought or mid-node returns an empty or truncated candidate, which is exactly "missing or unclosed". Rejected candidates and their stop reasons are never logged, so this can't be confirmed from logs yet.
3. **Codex:** 2 of 3 Codex compactions failed at commit with `Codex checkpoint lineage mismatch: generation must advance with a selected model` (Oct 27 12:24Z, Oct 28 02:29Z). Codex's own `prepare_compaction_generation` (codex/compaction.rs) landed in `042d3974` (Oct 28 06:23Z), after both failures. The third Codex compaction (Oct 28 04:06Z) validated and continued. No Codex compactions have happened since, so the fix is unconfirmed in production. Codex compaction is also slow: about 100 s exploration plus 142–311 s synthesis at `reasoning.effort = high`.
4. **"DAG injection corrupts":** no stored DAG is corrupt. All 53 persisted `compaction.summary` values pass the validator's rules. The validator is what rejects the bad candidates. The damage is that a rejected candidate blocks the session. There are real validation gaps (below), but none of them produced a bad summary in this data.
5. **"Stores things superfluous":** confirmed for storage. Every manifest keeps its full `compaction_commit` after publication (`publication_pending: false`), and in 53/53 sessions `covered_prefix` duplicates `source.prefix` exactly: 1.32 MB of 6.30 MB of manifest bytes (21%). Not a crash cause, but it is dead weight rewritten on every manifest revision. The compactor also sends its full instruction prompt, including the embedded previous DAG, twice: once as the system preamble and once as the user task.

## Evidence

### Error clusters (all logs)

| count | error |
|---|---|
| 33 | `compaction builder failed: compaction synthesis deadline exceeded; submit fresh input to retry` |
| 37 + 33 | `compaction builder output requires correction attempt=1/2 error=invalid DAG candidate: missing or unclosed dag-node structure` |
| 16 | HTTP 400 `Assistant tool call function.arguments must be a JSON object` (BUG-303, fixed) |
| 2 | `Codex checkpoint lineage mismatch: generation must advance with a selected model` |
| 2 + 2 | `summary submission blocked ordinary inference: compaction cancelled / deadline` |
| 5 | `No space left on device` while persisting a manifest |

### Per-model compaction outcomes (operation-level)

| provider/model | ops | ops with rejected candidates | synthesis ms min / median / max |
|---|---|---|---|
| openai qwen3.8-27b (spark) | 34 | 34 | 659 / 149 447 / 246 420 |
| openai qwen3.8-flash-next-iq3_s (Strata) | 4 | 3 | 29 741 / 104 035 / 199 559 |
| codex gpt-6.1-sol | 3 | 0 | 142 502 / 174 094 / 311 068 |

### One failing operation (831a24c4, 2026-10-29)

```
04:18:52 attempt 1 start (budget 600 s)
04:18:52–04:20:32 exploration 100 s (SessionSearch x4)  -> exploration deadline -> synthesis runway
04:20:32–04:21:56 synthesis 84 s                         -> validation: missing or unclosed dag-node structure
04:21:56 attempt 2 start: exploration AGAIN 104 s, synthesis 106 s -> same error
attempt 3 starts with ~7 s left -> synthesis deadline -> "submit fresh input"
```

### Code facts

- `agent-loop/src/compaction_builder.rs:197-199`: `builder.max_tokens(limits.output_reservation.map_or(maximum, |r| maximum.min(r)))`, where `maximum = SUB_AGENT_MAX_TOKENS = 8192` (`deep_search_provider_config.rs:15`) and `output_reservation` = the session's max output, i.e. 4096 from `providers/src/openai.rs:31` (see PROV-164).
- `agent-loop/src/compaction_execution.rs` `synthesize()`: collects only `StreamedAssistantContent::Text`. Reasoning, stop reason and usage are discarded, and a `max_tokens` stop is not distinguished from a model that ignored the format.
- `cli/src/compaction_operation.rs` `build_validated_dag`: every correction attempt calls `build(task)`, which starts a **new** sub-agent run with full exploration. The rejected candidate is appended to the prompt, so later attempts also grow.
- `agent-loop/src/compaction_builder.rs:150-160` + `deep_search_provider_config.rs`: `prompt` becomes both `preamble` and the `Target session ... {prompt}` task.
- No thinking control is sent for OpenAI-compatible models (no `chat_template_kwargs.enable_thinking` or equivalent), so the server's default thinking is used.
- The rejection warning logs only the error string. It omits the candidate length, an excerpt, the stop reason and the reasoning length.

### Validation gaps (latent; none produced a bad stored summary)

- **Builder vs commit context differ.** The builder validates with `required_files: empty` and `existing_dag` taken from the captured evidence (`compaction_builder.rs:107-112`). The commit re-validates with `required_files` from message annotations and `existing_dag = manifest.compaction.summary` (`cli/src/session/preservation.rs:25-55`). A candidate with its own `<dag-files>` section that omits an annotated file passes the builder and is rejected at commit, after the whole budget has been spent.
- **Replay does not re-validate.** `core/src/compaction/model.rs:367` wraps the stored summary into the compaction-dag reminder as-is.
- **Unvalidated public writer.** `codelet_core::persistence::set_compaction_state` (manifest_state.rs:34) writes any string as the summary. Only tests call it today.
- **Incremental prompt embeds the whole previous DAG message**, including the `<system-reminder>` wrapper (`compaction_dag.rs::build_generate_compaction_prompt` via `detect_existing_dag`), and requires D2 nodes to be reproduced byte-for-byte. That's a high bar for small models (`required durable node changed or missing`); it hasn't been observed yet.

### Storage

- 53 manifests carry `compaction_commit` with `publication_pending: false`. Largest: 86e15383, 338 KB manifest of which 69 KB is commit bookkeeping (`source.prefix` 34 KB plus an identical `covered_prefix` 34 KB, 1 245 UUIDs each).

## Recommended fix plan (proposed cards)

1. **Compactor synthesis budget and truncation (crux).** Give the compactor its own output budget, independent of the main session's 4096 reservation. Use at least `SUB_AGENT_MAX_TOKENS`, bounded by context capacity, and admit the request against that budget. Treat `max_tokens`/empty-text synthesis as a typed truncation failure with an exact correction message. Log the stop reason, text and reasoning lengths, and a short candidate excerpt on every rejection. Consider disabling or limiting thinking for synthesis on OpenAI-compatible models.
2. **Correction reuses evidence.** A validation failure should re-run synthesis only, with the rejected candidate and the error, against the already-gathered exploration history, not restart exploration. Time-box attempts so the last attempt always has synthesis runway.
3. **One validation context.** Build the `DagValidationContext` in one place and use it for both builder and commit. Optionally re-validate on replay and make `set_compaction_state` validate or become test-only.
4. **Commit bookkeeping.** Drop `covered_prefix` when it equals `source.prefix` (or store one list plus a hash), and clear or trim `compaction_commit` once publication is complete.
5. **Codex.** Confirm lineage on the next real Codex compaction. Consider `reasoning.effort = medium` for the compactor to cut the 2.5–5 minute synthesis.

## Open questions

1. Is it acceptable for the compactor to request more output tokens (e.g. 8192–16384) than the main session's profile, or should PROV-164's profile setting drive both?
2. Should synthesis disable thinking on OpenAI-compatible servers (qwen `enable_thinking: false`), or keep it and only raise the budget?
3. OK to run one live compaction probe against the spark server to capture a real rejected candidate before implementing?
