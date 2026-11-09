# PROV-165 AST research

## fspec astgrep --pattern 'resolve_context_window($$$A)' --lang rust --path rust/providers/src/manager
rust/providers/src/manager/impl_4.rs:217:9:resolve_context_window(


## fspec astgrep --pattern 'resolve_max_output_tokens($$$A)' --lang rust --path rust/providers/src/manager
rust/providers/src/manager/impl_4.rs:231:9:resolve_max_output_tokens(


## fspec astgrep --pattern '$M.context_window()' --lang rust --path rust/cli/src
rust/cli/src/compaction_threshold/catalog_tests.rs:8:9:manager.context_window()
rust/cli/src/compaction_threshold/catalog_tests.rs:64:52:manager.context_window()
rust/cli/src/compaction_publication.rs:21:24:manager.context_window()
rust/cli/src/interactive/stream_loop/preflight.rs:31:26:session.provider_manager().context_window()


## fspec astgrep --pattern '$M.max_output_tokens()' --lang rust --path rust/cli/src
rust/cli/src/compaction_threshold/catalog_tests.rs:9:9:manager.max_output_tokens()
rust/cli/src/compaction_publication.rs:22:27:manager.max_output_tokens()
rust/cli/src/interactive/stream_loop/preflight.rs:32:29:session.provider_manager().max_output_tokens()
rust/cli/src/provider_runtime.rs:6:18:manager.max_output_tokens()


## fspec astgrep --pattern '$M.context_window()' --lang rust --path rust/agent-loop/src
rust/agent-loop/src/turn_compaction.rs:29:24:manager.context_window()
rust/agent-loop/src/generate_compaction_handler.rs:36:28:inner.provider_manager().context_window()


## fspec astgrep --pattern '$M.max_output_tokens()' --lang rust --path rust/agent-loop/src
rust/agent-loop/src/turn_compaction.rs:39:13:manager.max_output_tokens()
rust/agent-loop/src/generate_compaction_handler.rs:37:26:inner.provider_manager().max_output_tokens()


## fspec astgrep --pattern '$M.context_window()' --lang rust --path rust/sessions/src
rust/sessions/src/model_resolution.rs:236:25:pm.context_window()
rust/sessions/src/model_resolution.rs:239:13:pm.context_window()


## fspec astgrep --pattern 'resolve_compaction_threshold($$$A)' --lang rust --path rust/cli/src/interactive
rust/cli/src/interactive/stream_loop/preflight.rs:50:21:resolve_compaction_threshold(


## fspec astgrep --pattern '$S.cached_context_window.load($$$A)' --lang rust --path rust/sessions/src
rust/sessions/src/committed_publication.rs:112:19:session.cached_context_window.load(Ordering::Acquire)


## fspec astgrep --pattern '$S.cached_compaction_threshold.load($$$A)' --lang rust --path rust/sessions/src
rust/sessions/src/committed_publication.rs:111:19:session.cached_compaction_threshold.load(Ordering::Acquire)


## Path-qualified sites (astgrep matches only the bare call; found with grep)

- `codelet_cli::compaction_threshold::resolve_compaction_threshold(` —
  `rust/agent-loop/src/turn_compaction.rs:37`, `rust/sessions/src/model_resolution.rs:238`.
- `ModelLimitsResolver` implementations: claude.rs:184, openai.rs:136, gemini.rs:47, zai.rs:68,
  copilot/provider.rs:213, codex/mod.rs:110, manager.rs:184 (`ConstantResolver`), plus test
  resolvers in model_limits/tests.rs and providers/tests/refac028_module_split.rs.
- `s.cached_context_window` / `cached_max_output_tokens` / `cached_compaction_threshold` —
  construction.rs:40-42, metrics.rs:54-58, handle_impl_1.rs:236-238 (SessionModel wire),
  handle_impl_4.rs:217-218 (ModelInfo wire), committed_publication.rs:111-112.

## Findings

- Every production reader of `ProviderManager::context_window()/max_output_tokens()` is listed above:
  4 in codelet-cli, 4 in codelet-agent-loop, 4 in codelet-sessions, 1 in the manager itself
  (`impl_3.rs:177`, OpenAI output cap) and the `raw_model_*` accessors (`impl_5.rs:14,29`).
- Two readers already decode the sentinel themselves: preflight builds
  `AdmissionLimits { context_capacity: (context_window > 0).then_some(..) }`
  (preflight.rs:37-38) and `provider_runtime::output_limit` filters `> 0` (provider_runtime.rs:7).
- `CompactionHook` compacts when `total > threshold` (core/src/compaction_hook.rs:131,160), so a
  threshold computed from a 0 capacity makes every request "need" compaction.
