# AST Research — CMPCT-046/047/048/049 (compaction fixes)

Performed 2026-09-18 via the AstGrep tool (language: rust) during discovery
for CMPCT-046 (self-target deadlock + zero-basis), CMPCT-047 (shared Level-3
fallback shape), CMPCT-048 (stash-lock status restore), CMPCT-049 (log-level
hygiene), and the CMPCT-046c provider-arm parity.

## 1. The 045 handler's fallback builder (CMPCT-047 target)

- Query: `fn build_fallback_dag(sub_agent_output: &str, total_turns: u32) -> String { $$$BODY }`
  in `rust/agent-loop/src`
- Match: `rust/agent-loop/src/generate_compaction_handler.rs:847`
- Finding: the handler's private `build_fallback_dag` now delegates to the
  shared `codelet_cli::compaction_dag::build_recovered_or_generic_dag` with
  label "Auto-recovered: compaction timeout" and body "Session was
  auto-compacted due to a compaction-sub-agent timeout." — no inline
  `format!(r#"<dag-node...>"#)` template remains.

## 2. Shared Level-3 primitives in codelet-cli (CMPCT-047 source of truth)

- Query: `pub fn build_generic_fallback_dag_node($$$ARGS) -> String { $$$BODY }`
  in `rust/cli/src`
- Match: `rust/cli/src/compaction_dag.rs:244`
- Siblings in the same module (text search corroboration):
  - `extract_partial_dag_nodes_from_text` (compaction_dag.rs:231)
  - `build_recovered_or_generic_dag` (compaction_dag.rs:263) — joins
    `extract_partial_dag_nodes_from_text(failed_output)` verbatim, else the
    generic D1 node.
- Call sites verified by text search:
  - `rust/cli/src/compactor_sub_agent.rs:253` (the 044 round)
  - `rust/agent-loop/src/generate_compaction_handler.rs` (the 045 handler)
  - `rust/agent-loop/src/agent_loop.rs` watchdog Level-3 generic-node arm
    (via `build_generic_fallback_dag_node`, label "Auto-recovered: compaction
    timeout", body "Session was auto-compacted due to convergence timeout.")

## 3. Self-target capture in execute_generate_compaction (CMPCT-046 target)

- Text-search corroboration (the AST full-body query for
  `pub async fn execute_generate_compaction($$$ARGS) -> Result<String, String>`
  did not match — the fn has no `pub` + `Result<String, String>` in that
  exact shape at module scope; the function is `pub(crate)`-ish and spans the
  whole capture block):
  - `rust/agent-loop/src/generate_compaction_handler.rs` — `is_callee`
    branch: `target_bg.inner.try_lock()` → on Ok captures
    (detect_existing_dag, pre_compaction_basis, message count); on Err
    degrades to `(None, cached_input_tokens.load(Acquire),
    completed_turn_count().max(1))`. Cross-session branch still awaits
    `target_bg.inner.lock().await` (a different session — safe) with a
    SLOW-lock warn at ≥5s.
  - New lock-free accessor: `rust/sessions/src/background_session.rs`
    `pub fn completed_turn_count(&self) -> u32` — counts
    `StreamChunk::Done` in `output_buffer` (the same basis `get_info()`
    uses for `message_count`).
  - New shared honest-basis helpers: `rust/cli/src/interactive_helpers.rs`
    `estimate_message_tokens(&[Message]) -> u64` and
    `pre_compaction_basis(tracker_input_tokens, &[Message]) -> u64`.
  - End-of-turn zero-basis guards: `rust/agent-loop/src/agent_loop.rs` and
    the napi twin `rust/napi/src/agent_loop.rs` both call
    `pre_compaction_basis(0, &inner_session.messages)` when
    `pre_compaction_tokens == 0 && has_pending_dag` before
    `apply_pending_dag_and_emit` clears the messages.

## 4. CMPCT-048 stash-failure status restore

- `rust/agent-loop/src/generate_compaction_handler.rs` — captures
  `let pre_compaction_status = target_bg.get_status();` before
  `set_status(Compacting)`; on the `is_callee` stash path,
  `if let Ok(mut guard) = target_bg.pending_dag_content.lock()` — the
  `else` arm restores `pre_compaction_status`, logs an ERROR naming the
  target + caller session, and returns the execution error.

## 5. CMPCT-046c provider-arm parity

- `rust/agent-loop/src/generate_compaction_handler.rs` —
  `build_and_run_compactor_agent`'s built-in match now has
  `"github-copilot" | "copilot" =>` (get_github_copilot +
  request_config_for_provider + distinct pending-builder error, mirroring
  `rust/agent-loop/src/deep_search_handler.rs:650` DeepSearch arm); the
  `_ =>` fallthrough names "claude, openai, gemini, codex, zai,
  github-copilot" as supported.

## 6. CMPCT-049 log-level markers (source-shape targets)

- `rust/agent-loop/src/agent_loop.rs` — DEBUG: "turn START — status →
  Running", "turn END — checking for pending DAG", "turn END — compaction
  flag was set", "turn END — watchdog retry pending".
- `rust/agent-loop/src/background_output.rs` — DEBUG: "Done: no pending
  compaction → status set to Idle"; INFO (rare branch): "Done:
  compaction/pending-DAG active → NOT setting Idle".
- `rust/sessions/src/background_session.rs` — INFO: "set_status
  transition"; DEBUG: "set_compaction_progress", "update_compaction_progress".
- `rust/fspec-tui/src/app/bootstrap.rs` — DEBUG: "TUI received status
  change".
- `rust/fspec-tui/src/app/dispatch_stream_chunks.rs` — DEBUG: "TUI store
  updated (SessionStateChange chunk)" and "TUI store updated (push channel)".
- `rust/fspec-tui/src/views/agent/animation.rs` — INFO, only on change:
  "TUI display decision: spinner shows ..." (flipped via
  `AgentView::last_compaction_diag_display`, new field in
  `rust/fspec-tui/src/views/agent.rs`).

## 7. BUG-186 (rpc002 test env)

- `rust/agent-loop/tests/rpc002_session_persistence.rs` — 4 tests call
  `handle.create_session(None)` with default model "anthropic/claude-sonnet-4"
  after `setup_data_dir()` (fresh temp data dir, no model cache) →
  `resolve_provider_manager → ModelRegistry::new → ModelCache::get()` fails
  offline → Err swallowed into an empty SessionId → `Uuid::parse_str("")`
  panics. Fix: seed `data_dir/cache/models.json` from
  `include_str!("fixtures/prov101_models.json")` in `setup_data_dir()` (the
  cmpct041/rpc386 offline-cache pattern) + set `ANTHROPIC_API_KEY` dummy
  creds, and switch the default model to a fixture model
  ("anthropic/claude-opus-4-5") so the offline registry accepts it.
- Pre-existing failing state confirmed on this machine:
  `cargo test -p codelet-agent-loop --test rpc002_session_persistence` →
  4 failed (valid UUID: ParseSimpleLength{len: 0}), 1 passed, before the
  fixture seed.
