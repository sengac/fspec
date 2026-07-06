# AST Research — PROV-140 (non-streaming wiring)

Structural analysis of the runtime call path the streaming flag must reach.
Tool: AstGrep + ripgrep + Read over `codelet/`.

## Symbols located

| Symbol | File:Line | Change needed |
|---|---|---|
| `pub struct OpenAIProvider` | `providers/src/openai.rs:60` | add `streaming: bool` field |
| `fn from_api_key_with_options` | `providers/src/openai.rs:185` | read `OPENAI_STREAMING` env (default true) into the field |
| `fn supports_streaming` | `providers/src/openai.rs:520` | return the field (currently hardcoded `true`) |
| existing env reads | `openai.rs:140/154/169/195` | `OPENAI_BASE_URL` / `OPENAI_CONTEXT_WINDOW` pattern to mirror |
| `fn apply_profile_env_vars` | `sessions/src/model_resolution.rs:169` | set `OPENAI_STREAMING` from `profile.streaming` (after :195) |
| `pub struct LocalServerProfile` | `sessions/src/profile_sections.rs:82` | **add** `#[serde(rename="streaming", default)] pub streaming: Option<bool>` — the loader struct `apply_profile_env_vars` reads does NOT yet carry the flag (PROV-139 added it to `ProfileDef`, a different struct) |
| `fn load_local_server_profiles` | `sessions/src/profile_sections.rs:213` | returns `Vec<LocalServerProfile>` — will surface `streaming` once the field is added |
| rig `stream: true` merge | `patches/rig-core/src/providers/openai/completion/streaming.rs:127` | Strategy B: gate on a stream flag; send `stream:false` + drop `stream_options` |
| `CompletionModel::stream` | `patches/rig-core/.../streaming.rs:110` | entry point for the transport-level branch |
| non-streaming rig method | `patches/rig-core/src/providers/openai/completion/mod.rs:1331` (`completion`) | reference single-response path |
| `RigAgent::prompt_streaming_with_history_and_hook` | `core/src/rig_agent.rs:148` | streaming multi-turn (`.stream_prompt()...multi_turn()`) |
| `RigAgent::prompt` | `core/src/rig_agent.rs:60` | non-streaming single-String (`.prompt().multi_turn()`) — no events/history/hook |
| stream construction site | `cli/src/interactive/stream_loop.rs:506` (+ recovery 714/1317/1684/1788) | branch on `supports_streaming()` to pick the stream source |
| `enum StreamEvent` | `cli/src/interactive/output.rs:143` | `Text(:144)`, `Done(:152)`; `emit_done_with_stop_reason(:243)` |
| agent_runner OpenAI dispatch | `cli/src/interactive/agent_runner.rs:65-81` | builds `RigAgent` from `get_openai(...)` |
| keystone request-shape test | `providers/tests/rhai_rig_agent_keystone_tests.rs:351` | pattern for asserting `model.stream(request)` body |

## Notes
- **Two structs named ~"profile"**: PROV-139 added `streaming` to `ProfileDef`
  (persistence) and `ProfileDefinition` (wire). The RUNTIME env bridge reads a
  THIRD struct, `LocalServerProfile` (`profile_sections.rs:82`) — it must also
  gain the `streaming` field for the bridge to work end-to-end.
- **Risk**: rig `.prompt().multi_turn()` panics on OpenAI Completions with tools
  (`providers/tests/openai_fireworks_deepsearch_repro.SUMMARY.md`) — spike first;
  prefer transport-level Strategy B.
- **Patch discipline**: rig-core edits go in `patches/rig-core/` AND
  `patches/rig-core.patch`.
- **File-size**: keep new/edited files < 300 LoC; extract the non-streaming
  adapter into its own module.
