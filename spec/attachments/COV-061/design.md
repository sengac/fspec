# COV-061 design: delete or port the unbuilt rust/tests and rust/examples sources

## Problem

`rust/Cargo.toml` is a virtual workspace (`[workspace]` only, no `[package]`), so Cargo builds no
target from `rust/tests/*.rs` (29 files, ~7500 lines) or `rust/examples/*.rs` (6 files). Their
assertions have never run since the monolithic `codelet` crate was split; most import that
crate, which no longer exists.

## Evidence used for each decision

1. A same-named, built copy in a crate's `tests/` directory (`ls */tests/<name>`).
2. Whether the file's header `Feature:` still exists in spec/features (it does not for any of
   them — every referenced feature was retired with the TypeScript/`codelet` era).
3. Whether the API it calls still exists in a workspace crate.

## Decisions

| File | Decision | Reason |
|---|---|---|
| agent_multi_turn_test, astgrep_tool_test, bash_tool_test, cache_token_extraction_test, claude_oauth_test, context_window_limits_test, core_file_tools_test, env_config_test, grep_glob_tools_test, large_write_intent_test, prompt_cache_control_test, provider_manager_test, rig_agent_multi_turn_test, rig_dependency_test, rig_provider_streaming_test, system_reminder_cache_preservation_test, system_reminder_infrastructure_test, tool_display_test, verify_rig_reexport (19) | delete | a built copy with the same name already lives in core/, tools/, cli/ or providers/ tests; the root copy is a stale duplicate |
| agent_runner_test | delete | `codelet::agent::Runner` / `ToolRegistry` no longer exist; the agent loop is tested in agent-loop/tests |
| claude_provider_test, gemini_provider_test, openai_provider_test | delete | old provider constructors; current providers are covered by providers/tests (claude_*, gemini, openai_local_provider_test, prov171_*) and the lib unit tests |
| integration_test | delete | runs a `codelet` binary that is no longer built |
| integration_wiring_test | delete | RigAgent + ClaudeProvider wiring is exercised by agent-loop/tests (bug214, bug220, bug253) |
| project_scaffold_test | delete | asserts the old single-crate layout |
| context_gathering_test | delete | `discover_agents_md` / `gather_environment_info` have 18 unit tests in cli/src/session/context_gathering.rs |
| debug_capture_test | delete + card | the API exists and per-session capture, toggling and header redaction are tested (common/src/debug_capture/tests, common/tests/refac026); correlation-id request/response capture, tool timing, tracing merge, session summary, zero-overhead and directory permissions are not, and their feature no longer exists -> new card to specify and test them |
| logging_system_test | delete + card | `init_logging` exists with one unit test; its feature no longer exists -> same new card |
| rust/test_debug (a scratch package with its own [workspace], so never built) and rust/test_websearch_json.json | delete | a WebSearchAction deserialisation experiment; the behaviour lives in codelet_common::web_search |
| benches/token_tracking.rs (found by the guard test) | delete | a placeholder criterion bench whose body is commented out (`black_box(1 + 1)`), against `codelet::agent::token_tracker`; no package depends on criterion |
| examples/agent_tool_calling, detailed_demo, openai_demo, test_all_17_events, test_all_tools, test_debug_e2e | delete | demos against `codelet::`; not built, not tests |

Porting the debug-capture and logging tests verbatim was rejected: they assert behaviour
against a retired feature (no acceptance criteria), the debug-capture file is 844 lines (it would
have to be split) and several assertions name the removed `codelet` binary and log file names.
ACDD requires the acceptance criteria first, so the card captures them.

## Guard

New feature `workspace-sources-belong-to-cargo-packages.feature` and test
`rust/common/tests/cov061_workspace_sources.rs`: every `.rs` file under rust/ (outside target/)
has a parent directory below rust/ whose Cargo.toml declares a `[package]`.

## Wire / API changes

None.
