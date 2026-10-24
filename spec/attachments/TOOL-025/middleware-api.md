# TOOL-025 middleware API handoff

Patched Rig has no dependency on application session management.

```rust
pub trait ToolMiddleware: WasmCompatSend + WasmCompatSync {
    fn scope_id(&self) -> &str;
    fn decorate_definition(
        &self,
        definition: ToolDefinition,
    ) -> Result<ToolDefinition, ToolError>;
    fn call<'a>(
        &'a self,
        tool: &'a dyn ToolDyn,
        args: String,
    ) -> WasmBoxedFuture<'a, Result<String, ToolError>>;
}
```

The default call implementation delegates unchanged. ToolSet calls the policy
before typed ToolDyn deserialization, facade/Rhai mapping or MCP forwarding.
Embedding tools upcast through ToolEmbeddingDyn to ToolDyn at the same boundary.

- `ToolSet::configure_middleware(Arc<dyn ToolMiddleware>).await` preflights fresh
  original definitions before binding. Different scope IDs reject rebinding;
  repeated same-scope installation retains the first policy, preventing downgrade.
- `ToolServerHandle::configure_middleware(...).await` acknowledges actor binding
  and propagates actionable schema/scope errors before any provider request.
- Definition retrieval is centralized for static, dynamic and direct ToolSet
  paths. Repeated retrieval decorates original copies, never decorated caches.
- Hot additions and appended toolsets preflight the destination policy before
  mutation. Cross-scope bound toolsets reject append. Rejected replacements do
  not replace the previous implementation; repeated additions do not duplicate it.

Application helpers in `codelet_tools::tool_attribution`:

```rust
decorate_definition(ToolDefinition) -> Result<ToolDefinition, AttributionSchemaError>
configure_tool_server(&ToolServerHandle, impl Into<String>) -> Result<(), ToolServerError>
configure_agent_tools<M>(&Agent<M>, impl Into<String>) -> Result<(), ToolServerError>
SchemaMiddleware::new(impl Into<String>)
```

The wire contract requires a non-null top-level `work_unit_id` string containing
an existing project card ID or exact `NONE`. Existing business root `work_unit_id`
properties are reserved-name collisions and must be rejected, never overwritten.
Business targets remain nested or use `target_work_unit_id`. There is no wrapper
compatibility or fallback.

The async configuration helpers currently construct schema-only policy. TOOL-026
should construct the trusted session/project admission policy there from the
first installation, rather than trying to replace an already-bound policy.
Unknown managed contexts must fail closed; explicitly registered auxiliary scopes
must validate independently without changing owner focus or permission bindings.

Current installation points pass trusted UUID scope strings:

- CLI `run_agent_stream_internal`, before request preparation. RigAgent exposes
  its underlying handle through `tool_server_handle()`.
- Shared DeepSearch `run_agent!` and its separate custom-provider branch, in both
  agent-loop and the live NAPI implementation. NAPI Codex now uses the strict
  wrapper completion model, not its raw client.
- Unified `compaction_execution::run` now accepts an explicit `session_id: Uuid`;
  production builders pass the ephemeral ID. Both streaming and nonstreaming
  execution configure before request preparation.
- `prompt_provider` is genuinely tool-free, matching its documented contract.

Runtime admission belongs in `ToolMiddleware::call`: validate original args,
obtain `SessionManager::admit_tool_invocation` receipt, strip only top-level
`work_unit_id`, then wrap the entire actual `tool.call(business_args)` future with
`codelet_tools::execution_context::scope_invocation(receipt, future)`. Keep the
original caller-owned arguments/history unchanged. Do not call back into the
same tool-server actor from inside this callback. Admission order is actor
dequeue order; no actor concurrency refactor was introduced.

Direct completion APIs decorate raw definitions but return tool calls without
executing them or establishing session identity. Any execution consumer must use
a configured trusted tool-server boundary. Custom Rig bridges have separate
trusted entry points to avoid confusing genuine business collisions with schemas
already decorated by the harness. Post-Rhai validation compares entire schemas.

Strict Codex production constructors preserve JSON function schemas and omit
hosted native tools; late catalog replacement fails. Non-strict raw transport
fixtures remain supported independently. Gemini uses `parametersJsonSchema` to
preserve complete business/harness semantics rather than the lossy OpenAPI subset.
