# PROV-140 — Verified call-path trace (2026-07-06)

Supplements `streaming-wiring-design.md` with the exact, code-verified wiring
plan from a DeepSearch over `codelet/cli/src/interactive`, `codelet/core`,
`codelet/providers`, `codelet/agent-loop`, `codelet/sessions`.

## What already exists (delivered by PROV-139)

- Wire `ProfileDefinition.streaming: Option<bool>` (`rpc-types/src/lib.rs:461`)
  + `streaming_enabled()` (`:469`, `unwrap_or(true)`).
- Disk `ProfileDef.streaming` (`sessions/src/profile_persistence.rs:43`) +
  save/read; wire→disk bridge `profile_def_from_wire`
  (`sessions/src/conversions.rs:184`); TUI toggle.

## What must be built (the "last mile")

### 1. Thread the flag to the provider (env-var bridge — matches existing pattern)
- `sessions/src/model_resolution.rs` → `apply_profile_env_vars(...)` (**:169**)
  is the single source of truth that already sets `OPENAI_BASE_URL` (:188),
  `OPENAI_API_KEY` (:191), `OPENAI_CONTEXT_WINDOW` (:195) from the loaded
  profile. Add: read `profile.streaming` and set `OPENAI_STREAMING` env
  (only when `Some(false)`, or always set "true"/"false" explicitly).
  Verify the profile loader (`load_local_server_profiles`) surfaces the new
  `streaming` field.
- `providers/src/openai.rs` → `OpenAIProvider` struct (**:60**) gains a
  `streaming: bool` field; `from_api_key_with_options` (**:185**) reads
  `OPENAI_STREAMING` (default true) into it; `supports_streaming()` (**:520**,
  currently hardcoded `true`) returns the field.

### 2. Add a non-streaming driver that emits the SAME stream shape
- `RigAgent` (`core/src/rig_agent.rs`) currently exposes streaming multi-turn
  (`prompt_streaming_with_history_and_hook`, **:148**, uses
  `.stream_prompt().with_history().with_hook().multi_turn()`), and a
  single-`String` non-streaming `prompt()` (**:60**, `.prompt().multi_turn()`)
  that yields NO events/history/hook.
- The interactive driver is `run_agent_stream_internal`
  (`cli/src/interactive/stream_loop.rs:269`); it builds the stream at **:506**
  and consumes `MultiTurnStreamItem` variants → `StreamEvent::Text`
  (`output.rs:144`) … terminal `emit_done_with_stop_reason`
  (`output.rs:243`, `StreamEvent::Done` at `:152`). Recovery re-invocation
  sites: **714, 1317, 1684, 1788**.

### 3. Branch on `supports_streaming()` at the stream-construction site
- Smallest correct point: at `stream_loop.rs:506` (and the 4 recovery sites,
  ideally via a shared local helper), choose the streaming vs non-streaming
  stream SOURCE, reusing the entire existing `match stream.next()` loop so the
  `Text… then Done` output is byte-for-byte identical.

## ⚠️ CRITICAL RISK — de-risk with a spike FIRST

`providers/tests/openai_fireworks_deepsearch_repro.SUMMARY.md` documents that
rig's **`.prompt().multi_turn()` PANICS** on the OpenAI **Completions** API
with tools (`"The OpenAI Completions API doesn't support reasoning!"`), while
`.stream_prompt().multi_turn()` succeeds. Therefore a naive non-streaming path
via `RigAgent::prompt()` may be broken for the exact provider we target.

**Spike outcome decides the implementation strategy:**

- **Strategy A** — if `.prompt().multi_turn()` works (or can be made to work
  without reasoning): add a `RigAgent::prompt_nonstreaming_with_history_and_hook`
  that runs it and adapts the result into the same
  `Stream<MultiTurnStreamItem>` (synthetic Text items + terminal
  `FinalResponse`). Branch at `stream_loop.rs:506`.
- **Strategy B (transport-level, panic-proof)** — thread a `stream: bool` into
  the rig `CompletionModel` so its `stream()` method
  (`patches/rig-core/src/providers/openai/completion/streaming.rs:125`) sends
  `stream: false` (dropping `stream_options`), parses the single JSON
  response, and yields it as a one-item stream that rig's EXISTING
  `.stream_prompt().multi_turn()` state machine consumes unchanged. This keeps
  the whole multi-turn/tool/usage path intact and sidesteps the panic, at the
  cost of a vendored rig-core patch change (reflect in
  `patches/rig-core.patch`).

**Recommendation:** run the spike; prefer Strategy B if the panic reproduces,
because it reuses rig's tool loop and guarantees identical downstream output.

## Observable acceptance criteria (approach-independent — these are the scenarios)

1. `supports_streaming()` returns the per-profile flag (false when the active
   profile disabled streaming; true by default / when absent).
2. With streaming disabled, the outgoing OpenAI chat request body has
   `stream: false` (or omits `stream`) and omits `stream_options`.
3. With streaming disabled, a plain text reply is still delivered as the same
   terminal `StreamEvent` sequence (`Text…` then `Done`).
4. With streaming disabled, a tool-calling turn still executes tools and
   completes the multi-turn loop.
5. With streaming enabled (default / `None`), the request has `stream: true` +
   `stream_options.include_usage` — behaviour unchanged (regression guard).
6. Verified across both transports (embedded + WebSocket) where the flag flows.
