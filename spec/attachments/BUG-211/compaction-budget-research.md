# BUG-211: compaction depth exhaustion and retained inference block

## Evidence

Investigated revision: `af50d4193005b27a552f634d5677eb94fc44977c`.

Screenshots supplied by the user:

- `/Users/rquast/Desktop/Screenshot 2026-10-21 at 9.45.21 am.png`
- `/Users/rquast/Desktop/Screenshot 2026-10-21 at 9.45.32 am.png`

Matching runtime session: `bcc63ae1-b811-49ce-8c62-5a032a7fa3c3`,
provider/model `codex/gpt-6.1-sol`.

Log: `~/.fspec/logs/fspec-combined.log.2026-10-20` (UTC timestamps).

| Local time, UTC+10 | Event |
| --- | --- |
| 2026-10-21 09:34:50 | Terminal compaction fails: builder fails with `DeepSearch sub-agent failed: MaxDepthError: (reached limit: 20)`. |
| 2026-10-21 09:45:22 | Fresh user input starts another turn. |
| 2026-10-21 09:45:24 | Inference guard rejects the turn with the stored builder failure. No second depth-exhaustion event is necessary. |

The persisted manifest contains 579 originals and a previous summary covering
231 originals. The compactor's individual failed tool calls are not persisted in
these logs; their exact sequence cannot be recovered from this evidence.

## What the latest commit fixes

`af50d419` resumes unfinished tasks after successful durable compaction,
distinguishes synthetic retry provenance, bounds successful automatic recovery,
forwards progress/metrics, and routes NAPI through the shared loop.

It does not change `COMPACTION_SUB_AGENT_MAX_DEPTH`, reserve a synthesis request,
or recover a fresh task after a failed builder. Its new-task restore branch runs
only when three successful recoveries have occurred. A builder failure occurs
before that counter increments, so this branch does not address the screenshot.
The BUG-210 failing-builder test checks only that failure stops the current turn;
it does not submit another user message after the failure.

## Root causes and provenance

1. `rust/agent-loop/src/compaction_builder.rs` configures an exploration limit of
   20 and returns a depth error immediately instead of allowing a final synthesis
   from the already gathered evidence. The prompt asks for searches and bounded
   original-evidence pages, so useful exploration can consume this budget.
2. The streaming collector shared with DeepSearch converts typed errors to
   strings, losing the `MaxDepthError` history needed for safe final synthesis.
   Its label explains why a compactor error mentions DeepSearch.
3. Rig's streaming depth error stores `current_prompt.rag_text()` as a new text
   message rather than preserving the structured final tool-result message.
   The last original-evidence page must not disappear during final synthesis.
4. `rust/cli/src/compactor_sub_agent.rs` deliberately retains the inference block
   when compaction fails. This safety gate is correct, but there is no runtime
   path explicitly restarting a failed pre-commit builder on fresh user input.
   `resolve_compaction_operation` exists but has no production caller and retains
   the old operation's deadline, so calling it alone is not sufficient.

`git blame` identifies `b746189bb` (`refactor: redo compaction`, 2026-10-19) as
the source of the hard-coded cap and retained failure block. The latter is not
itself unsafe behavior; the missing recovery path makes it a user-visible trap.

The displayed 18% is the parent session's token display, not the sub-agent's
execution budget. Nothing in these errors establishes disk exhaustion or a NAPI
compilation failure. The underlying overflow request is replaced by the terminal
recovery error in the available logs, so its exact payload is not established.

## Fix constraints

- Keep exploration bounded; reserve exactly one tool-free synthesis per exhausted
  builder candidate, preserving structured evidence and selected provider/model.
- Apply the existing bounded correction, DAG validation, admission, durable
  commit, and publication rules to synthesized output. Never fabricate a summary.
- Preserve typed errors through the compactor collector; other errors must not
  masquerade as depth exhaustion or trigger synthesis.
- Treat fresh user input as explicit permission to restart only a builder that
  failed before commit. Give that new operation a fresh bounded deadline/budget.
- Retain the inference block until recovery commits and publishes successfully.
  Do not replay the oversized ordinary request while retrying the builder.
- Do not restart publication/commit failures or unrelated provider-session
  failures. Cancellation and synthetic retries must not trigger implicit retries.

## ACDD validation

Acceptance criteria are split into three feature/target pairs to preserve the
project's one-feature/one-test-file rule and keep new Rust files below 300 lines:

- `compaction-builder-budget-recovery.feature` →
  `bug211_compaction_budget_recovery.rs` (8 scenarios).
- `compaction-synthesis-admission.feature` →
  `bug211_synthesis_admission.rs` (2 scenarios).
- `compaction-explicit-retry-safety.feature` →
  `bug211_explicit_retry_safety.rs` (2 scenarios).

The initial seven-test red run had three passing and four failing tests:
exhaustion returned `MaxDepthError`, invalid candidates never reached correction,
fresh input remained blocked, and a repeated failure never invoked a second
builder. The additional streaming regression reproduced `MaxDepthError` too.
Provider errors, unrelated blocks, and background notifications already stayed
blocked. Two subsequent provider-boundary regressions failed before their fixes:
Claude rejected synthesis without historical tool declarations, and an oversized
OpenAI synthesis request was dispatched instead of being rejected locally.

The evidence fixtures now persist canonical original envelopes, not just readable
message text. Both streaming and non-streaming tests inspect real serialized
provider requests for the final gathered originals page. Streaming additionally
checks its structured function-call output identity and absence of duplication.

## Implemented recovery

- `rust/agent-loop/src/compaction_execution.rs` retains typed depth errors and
  allows exactly one direct synthesis request per exhausted candidate. It does
  not execute tools or start another multi-turn loop during synthesis. Other
  provider failures still return immediately.
- Rig preserves the final structured streaming prompt instead of converting it
  to `rag_text()`. The driver restores a popped streaming tail, while avoiding
  duplication of the non-streaming tail already included in history.
- Synthesis reuses the selected agent/model, preamble, and provider settings.
  Claude retains declarations required to interpret historical tool blocks and
  requests `tool_choice: none`; other built-in providers receive no tool list.
  Any attempted synthesis tool call is rejected, never executed.
- Non-Codex synthesis passes serialized-request admission before dispatch. Codex
  retains its native physical admission. Configured output tokens are capped to
  the selected reservation where supplied through the builder token setting.
  Invalid output still passes through the existing three-candidate validator;
  durable commit and publication remain mandatory before inference is unblocked.
- `CompactionOperation` records whether the failure was a pre-commit builder
  failure. A genuinely fresh user-channel input may replace that operation with
  a fresh bounded deadline, identity, and attempt budget. The inference guard
  remains set until the normal commit/publication path succeeds.
- Synthetic retries, supervisor/MCP inputs, and background notifications cannot
  authorize this restart. Cancellation or a later unrelated/publication block
  revokes retry permission. Failed explicit retries bypass terminal-overflow
  retry scheduling and return idle after one attempt.

## Observed verification

All commands use scoped targets and disabled debug information:

```sh
export CARGO_PROFILE_CI_TEST_DEBUG=0
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
cd rust
cargo test --profile ci-test -p codelet-agent-loop \
  --test bug211_compaction_budget_recovery \
  --test bug211_synthesis_admission --test bug211_explicit_retry_safety
cargo test --profile ci-test -p codelet-agent-loop \
  --test bug210_automatic_continuation --test bug210_compaction_lifecycle \
  --test bug210_completion_metrics --test compaction_evidence_lifecycle \
  --test cmpct054_cancellation_cleanup
cargo test --profile ci-test -p codelet-cli \
  --test stream_recovery_compaction_builder --test compaction_builder_validation \
  --test cmpct052_inference_handoff
cargo test --profile ci-test -p codelet-providers --test prov149_codex_history
cargo test --profile ci-test -p codelet-agent-loop --test prov149_codex_reload
cargo check --profile ci-test -p codelet-agent-loop -p codelet-cli --lib
cargo clippy --profile ci-test -p codelet-providers -p codelet-cli \
  -p codelet-sessions -p codelet-agent-loop --lib
```

Results: all 12 new scenarios pass; the successful neighboring targets contribute
38 additional passing tests (50 distinct passing tests across these targets).
Scoped compilation and Clippy pass. All three feature files pass formatting,
Gherkin validation, tag validation against the repository tag registry, and
coverage audits. Each scenario has test and implementation mappings.

Broader validation limitations, left unchanged:

- `cmpct044_terminal_overflow_test` does not compile: three existing assertions
  apply `!` to `Result<bool, TerminalRecoveryError>`. Neither this target nor the
  recovery return type was changed for BUG-211.
- `cmpct057_operation_lifecycle` has three passing tests and one failing test,
  `blocked_work_cannot_implicitly_start_another_builder`. Its fixture creates a
  random session ID without a persisted manifest, so the unchanged durable-source
  capture fails before its expected builder invocation (observed count 0 vs 1).
- Repository-wide tag validation reports 526 invalid older features. Isolated
  validation of the three BUG-211 features against the same tags/work units
  passes. No unrelated tags or features were modified.

The final repeat of the three BUG-211 targets also passed: 8 budget-recovery,
2 explicit-retry safety, and 2 synthesis-admission tests, with no failures.
BUG-211 is done and all three linked features carry the completion tag.

No workspace-wide Cargo test run was attempted, and no BUG-211 commit was made.
