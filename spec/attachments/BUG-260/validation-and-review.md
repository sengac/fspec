# BUG-260 validation and direct review

Baseline: `b2475dcd` plus the independently validated BUG-259 repair. BUG-259
does not modify providers or the lineage projection under test.

## Real causal evidence

```sh
cargo test --offline --profile ci-test -p codelet-providers --test bug260_refusal_history -j2
```

- Red: **1 passed, 2 failed**. Both new physical follow-up cases returned
  `RequestError(Lineage("assistant, tool result or DAG ancestry"))` before the
  second request. Altered-text/unknown-metadata rejection control passed.
- Green: **3 passed, 0 failed**, same tests and command. The fixture module was
  made public, matching adjacent integration tests, to avoid unused-helper
  warnings; assertions and transport behavior were not changed to obtain green.
- Captured outputs: `red-ci-test.log` and `green-ci-test.log` in this directory.
- Local TCP fixture records serialized production requests. Eight combinations
  cover refusal-only/mixed answers, next-user/nudge prompts and readable reasoning
  on/off. Tool-result cases additionally verify original call identity, result,
  exact opaque reasoning and native refusal, with no duplicate replay.

## Related matrix

All use `cargo test --offline --profile ci-test -p codelet-providers -j2`
with explicit `--test` targets, captured once in `regressions.log`.

| Target | Passed |
| --- | --- |
| bug260_refusal_history | 3 |
| cmpct078_committed_lineage | 3 |
| codex_parallel_tool_lineage | 3 |
| codex_streamed_lineage_roundtrip | 4 |
| codex_terminal_regressions | 7 |
| prov148_codex_completed_answers | 1 |
| prov149_codex_history | 2 |

Total: **23 passed, zero failures**. Existing regression-target helper warnings
are retained in the log; no lint suppression or unrelated repair was added.

Additional persisted-continuation controls passed under codelet-agent-loop:
`prov149_codex_reload` **3 passed** and `bug257_fork_continuation` **6 passed**.
Output: `continuation-regressions.log`. Combined affected matrix: **32 passed**.

## Direct review

- Change is confined to the readable comparison inside `comparable`, not the
  trusted checkpoint, its generation, replay assembly or serialized native items.
- Only assistant refusal objects containing exactly the recognized type/string
  fields normalize after the already-existing annotations/logprobs removal.
  Unknown metadata remains unmatched; malformed/non-string sections do not gain
  valid comparison semantics. User content is not normalized as assistant output.
- Normalized sections use the existing adjacent-text merging, matching `Answers`
  display order. Tests exercise actual displayed strings, not invented text.
- Native refusal, encrypted reasoning, phase, item ID, call ID and arguments
  survive the physical request unchanged. Genuine ancestry mismatches still
  reject before a second inference request.
- No protocol-continuation policy, refusal behavior, cancellation mechanism,
  admission, credentials, endpoint or completion-tool semantics changed.
- Both changed Rust files remain below 300 lines. No outside card changed.

## Quality / traceability

- Scoped `rustfmt --check` and `git diff --check`: passed.
- Native Gherkin and tag validation: passed.
- Three scenarios have native test and implementation coverage mappings.
- `cargo check --offline --profile ci-test -p codelet-providers -j2`: passed.
- `cargo clippy --offline --profile ci-test -p codelet-providers --test bug260_refusal_history -j2`: passed, no reported warnings.
- Final new-target run after wording clarification: 3 passed; `final-ci-test.log`.
- Native coverage audit: all mappings valid, 6/6 files found.
- Native Done and final tag verification follow completed validation.
