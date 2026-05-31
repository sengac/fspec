# AST Research — RPC-066 (Cross-frontend integration test)

Research goal: Map the existing Rust surface area we need to widen / drive
for the cross-frontend parity test against a stub LLM provider.

## 1. `LlmProvider` trait surface

`codelet/providers/src/lib.rs:98`

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &str;
    fn model(&self) -> &str;
    fn context_window(&self) -> usize;
    fn max_output_tokens(&self) -> usize;
    fn supports_caching(&self) -> bool;
    fn supports_streaming(&self) -> bool;
    async fn complete(&self, messages: &[codelet_common::Message])
        -> Result<String, ProviderError>;
    async fn complete_with_tools(
        &self,
        messages: &[codelet_common::Message],
        tools: &[ToolDefinition],
    ) -> Result<CompletionResponse, ProviderError>;
}
```

Existing implementors (AstGrep `impl LlmProvider for $NAME { $$$BODY }`):

- `gemini.rs:317` GeminiProvider
- `zai.rs:346` ZAIProvider
- `claude.rs:680` ClaudeProvider
- `copilot/provider.rs:230` CopilotProvider
- `codex/mod.rs:506` CodexProvider
- `custom/provider.rs:216` RhaiCustomProvider
- `openai.rs:499` OpenAIProvider

`StubProvider` (`codelet/providers/src/stub_provider.rs:23`) currently does
NOT implement `LlmProvider` — only exposes `canned_chunks()`. This card
widens it.

## 2. `CompletionResponse` shape

`codelet/providers/src/lib.rs:74`

```rust
pub struct CompletionResponse {
    pub content: MessageContent,   // Text / Parts (Parts can carry ToolUse)
    pub stop_reason: StopReason,   // EndTurn / ToolUse / MaxTokens
}
```

For the scripted run our StubProvider impl needs to:

- For ordinary inputs: return `CompletionResponse { content: MessageContent::Text("hi back"), stop_reason: EndTurn }`.
- For inputs containing `"trigger-tool"`: return `CompletionResponse { content: MessageContent::Parts([ContentPart::ToolUse{ id, name: "noop_tool", input: {} }]), stop_reason: ToolUse }` so the SessionManager's tool dispatcher exercises the ToolCall + ToolResult chunk paths.

## 3. Custom-provider registration path

`codelet/providers/src/manager.rs:131`

```rust
pub fn custom_provider_registered(slug: &str) -> bool {
    match crate::custom::discover_provider_configs() {
        Ok(configs) => configs.iter().any(|c| c.name == slug),
        Err(_) => false,
    }
}
```

`discover_provider_configs` (`codelet/providers/src/custom/discovery.rs:26`)
scans `~/.fspec/providers/*.json` (or `FSPEC_HOME`) then
`.fspec/providers/*.json` from the CWD. It is the ONLY source of truth
for `custom_provider_registered`.

`ProviderConfig` (`codelet/providers/src/custom/config.rs:217`) has 14
fields including `name`, `script` (path to `.rhai`), `models`, `tool_style`,
`api_style` — i.e. the disk-driven config is heavily Rhai-flavoured.

**Implication for the architecture:** the architecture note [C] proposal
(call a new helper `register_stub_provider()` that "inserts a synthetic
ProviderConfig into the custom provider registry") cannot work with the
current shape — there is no in-memory registry to insert into. Two
viable approaches:

- **Disk path:** the daemon spawn-helper writes a minimal
  `<workspace>/.fspec/providers/stub.json` AND we add a `facade =
  "stub"` short-circuit branch in the manager that routes ProviderType::Custom("stub")
  to our LlmProvider impl WITHOUT loading a Rhai script (config.script is empty).
- **In-memory registry:** add a new `OnceCell<HashMap<String, fn() -> Arc<dyn LlmProvider>>>`
  to the manager module so `register_stub_provider()` can install the
  StubProvider factory at process boot. `custom_provider_registered`
  consults BOTH the disk discovery AND the in-memory map.

The in-memory approach keeps the test deterministic (no temp-dir disk
write race) and isolates the stub from the Rhai surface entirely.
**Decision for this card:** in-memory registry (matches AC #3 rule:
"sidesteps Rhai entirely"). Will be implemented under
`codelet_providers::stub_provider::register_stub_provider()` behind
`test-support`.

## 4. Wire-level chunk type

`codelet/rpc-types/src/lib.rs:999` — `StreamChunk` enum with 22 variants
including `Text{ text, correlation_id, observed_correlation_ids }`,
`ToolCall{ tool_call, correlation_id, observed_correlation_ids }`,
`ToolResult{ tool_result, correlation_id, observed_correlation_ids }`,
`Done`.

`StreamChunk::text(s)` constructor sets `correlation_id: None,
observed_correlation_ids: None`. Our golden file will normalise these
fields anyway.

## 5. `FspecBackend` trait

`codelet/fspec-tui/src/transport/mod.rs:64` defines the cross-transport
trait the test will drive. Methods we touch:

- `create_session(&self, role: Option<String>) -> Result<SessionId>` — **takes a role overlay, NOT a model string.** The handle's impl (`codelet/sessions/src/handle_impl.rs:71`) reads `SessionManager::get_default_model()`.
- `send_input(id, text)`
- `interrupt(id)`
- `clear_history(session_id)` (line 304/578)
- `compact_session(session_id)` (line 311/588)
- `set_thinking_level(session_id, level)` (line 189)
- `chunks_rx()` -> `broadcast::Receiver<(SessionId, StreamChunk)>`
- `status_changes_rx()` -> `broadcast::Receiver<(SessionId, SessionStatus)>`

**Implication:** `create_session` does NOT let us choose `"stub/canned"`
directly. The test will need to invoke `SessionManager::set_default_model("stub/canned")`
on the daemon side BEFORE the WS client calls `create_session(None)`.
Easiest path: the `test-stub-provider` feature in `codelet-fspec` makes
`build_service` call `manager.set_default_model("stub/canned")` AFTER
constructing the SessionManager.

## 6. `SessionManager::create_session_with_id` signature

`codelet/sessions/src/session_manager.rs:401`

```rust
pub async fn create_session_with_id(
    &self,
    id: &str,
    model: &str,
    project: &str,
    name: &str,
) -> Result<(), String>
```

The model string is `"<provider>/<model_alias>"` — e.g. `"stub/canned"`
routes through `ProviderType::from_str("stub")` which (per the manager
layer) hits `custom_provider_registered` first.

## 7. Daemon spawn helper

`codelet/fspec/tests/common/mod.rs:86`

```rust
pub fn spawn_fspec_daemon(workspace: &Path) -> (ChildGuard, u16) {
    let mut child = Command::new(fspec_bin())
        .arg("daemon")
        .arg("--workspace").arg(workspace)
        .stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().expect("spawn fspec daemon");
    // reads first stdout line, parses as u16
}
```

**Reuse verbatim** for the parity test — already proven by RPC-010 et al.

## 8. `build_service` extension point

`codelet/fspec/src/common.rs:80` constructs the SessionManager + hooks
+ shared service. The `test-stub-provider` feature flag will gate a
one-time call into the providers crate's `register_stub_provider()`
+ `manager.set_default_model("stub/canned")`.

## 9. `tests/` layout

`codelet/fspec/tests/` already has 8 integration-test binaries:
`cargo_shape.rs`, `client_mode.rs`, `combined_smoke.rs`,
`daemon_lifecycle_rpc011.rs`, `daemon_mode.rs`, `no_napi_dependency.rs`,
`stale_daemon_json_rpc011.rs`, `status_subcommand_rpc011.rs`.

`cross_frontend_parity.rs` is the new sibling. Reuses `mod common;` for
the daemon spawn helper.

## Risk register (carries over to implementation)

- **Risk A (Rhai shadow):** if `set_default_model("stub/canned")` runs BEFORE the in-memory registry is populated, `ProviderType::from_str` will fail. Mitigation: `register_stub_provider()` is called FIRST inside `build_service`.
- **Risk B (Tool dispatcher reach):** the `complete_with_tools` `ToolUse` branch may push the session into a path that requires a real `codelet_tools::*` registration for `noop_tool`. The stub provider can return a tool_use whose input is an empty object and the `noop_tool` registration must be a no-op stub registered alongside.
- **Risk C (Agent-loop wiring bugs):** acknowledged in architecture note [I]. Surface as sibling cards rather than fixing inline.
