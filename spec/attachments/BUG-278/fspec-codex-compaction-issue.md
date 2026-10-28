# Codex compaction rejects checkpoint generation in a valid selected session

## Observed issue

The Fspec native session `ba15d5a4-4ccf-4d70-9ec9-78594797286a` was running the verified selection `codex/gpt-6.1-sol` while completing BUG-271, BUG-272 and BUG-274. At terminal compaction it failed with:

```
terminal compaction failed: compaction commit failed: compaction candidate rejected: Codex checkpoint lineage mismatch: generation must advance with a selected model
```

Those three work units are completed and must stay completed. This is a separate bug. Roland authorized documenting and fixing it through Fspec ACDD on 2026-10-28. The failed session is evidence, not a repair target: do not resume it, reset ancestry, rewrite its history, or retry its compaction blindly.

## Reproduction context and durable evidence

- Discovered daemon PID 82606, cwd `/Users/rquast/projects/fspec`, MCP endpoint `ws://127.0.0.1:56382`. PID 371 belongs to BPD and is excluded. Discover again if the connection changes.
- Supported connection: `fspec client --connect ws://127.0.0.1:56382 --mcp`.
- MCP confirms effective provider `codex`, model `gpt-6.1-sol`, idle status, and no pending pause/HITL interaction.
- Durable transcript request `33fa99c0-a7ca-4500-ab1f-53325f2e54f1`, sequence 723, timestamp `2026-10-28T02:29:30.432108Z`, records terminal outcome `compaction_failed`.
- At recovery the committed manifest had 372 message references and no promoted compaction: `compaction` and `compaction_commit` were null. Provider-state content identity `62e420aa471e2d6c3a0af3bd7583122652e47d3d3b8506084dc99b81fa0e0c7d` verified. Its Codex checkpoint generation was 1, source revision 932, prefix length 370; the prefix hash verified. No pending persistence-recovery file existed.
- Completing BUG-271 caused ordinary watcher metadata to advance manifest revision 936 to 937 and clear its work-unit focus. This was recorded honestly; whole-manifest byte equality was not claimed.
- Existing read-only evidence: `spec/attachments/BUG-271/bug271-272-274-final-recovery.json` and `.md`. Original three-card validation receipts remain under BUG-272. Preserve all of them.

## Expected behavior

A valid selected Codex session must be able to compact successfully: admit a real validated DAG against committed originals, advance correct checkpoint lineage, durably promote exactly one compaction generation, publish that exact identity, restore it cold and continue inference without losing or repeating completed tool work. Successful compaction cannot be a fallback, fabricated result, alternate model, reset ancestry or truncated-history workaround.

Invalid, stale or incompatible checkpoint evidence must still fail explicitly without promotion, publication, changed original history, or false successful inference. Lost acknowledgement and publication failure must retain strict exact-identity reconciliation.

## Scope and research leads

Investigate the relationship between durable compaction generations and Codex provider checkpoint generations. The observed manifest has no compaction commit but a valid provider checkpoint at generation 1. Source currently raises the error from `rust/providers/src/codex/history_generation.rs::prepare_generation`; caller wiring includes `rust/cli/src/session/compaction.rs`. Establish the actual causal path with a deterministic fixture before changing production.

Review existing selected-provider, durable-boundary, restoration, replay, no-substitute, physical-freshness and reset-authority coverage. `bug212_selected_provider_checkpoints` includes an intentional invalid-lineage refusal and must not be weakened to obtain a green result. If an old scenario encodes an obsolete assumption, document the distinction and replace it with equally strict evidence-based safety coverage through supported specification commands.

## Acceptance criteria

1. A real deterministic selected-Codex fixture reproduces the valid first-compaction failure before production changes, with causal behavioral RED rather than setup or compilation failure.
2. First and successive valid compactions advance compatible durable and provider lineage, persist real DAG/checkpoint evidence, publish the exact promoted generation and open inference.
3. Cold restore and continuation preserve original messages, selected model identity, genuine call/outcome pairs and exactly-once effects across the successful boundary.
4. Invalid or stale lineage, changed selected model, missing originals/outcomes and candidate mutation still fail safely; no fallback, fabricated success or destructive repair is introduced.
5. Publication failure/lost acknowledgement retain strict inference blocking and exact durable-identity reconciliation.
6. Final scoped tests, coverage, feature/tag gates, applicable builds/checks and Clippy with `-D warnings` pass against final source, with precise receipts and documented limitations.

## Test and safety plan

Use real temporary persistence and shared test helpers; serialize fixtures that change global stores. Set isolated temporary `FSPEC_USER_DIR`, `XDG_CONFIG_HOME`, `XDG_RUNTIME_DIR`, synthetic data roots and temporary credential fixtures for every test command. Never reuse the real config, credentials or live session store as test inputs. Check real config SHA-256 before/after; expected baseline `8eed6007c58b8f902efcf640fb8625a2662367cf4551f532ab354773650fce4c`. Stop on mismatch without guessed restoration.

Use named Cargo packages and targets, `--offline --profile ci-test -j2`, `CARGO_PROFILE_CI_TEST_DEBUG=0`, one compiler queue and a free-space guard. Never run unscoped workspace tests or perform speculative cleanup. Preserve pre-existing source changes. Do not install or replace binaries, restart the daemon, commit, push, deploy or touch unrelated cards. Standard Fspec workflow checkpoint behavior must remain explicit and must not be suppressed.

Maintain feature-first, executable RED, implementation, GREEN and final validation order in the separate work unit. Register this document using the actual `fspec add-attachment` command and verify the stored path in the work-unit record.
