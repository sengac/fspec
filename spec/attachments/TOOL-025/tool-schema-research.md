# TOOL-025: mandatory work-unit attribution schemas

Research date: 2026-10-22. Design proposal, not implementation.
Overview: `docs/research/session-work-unit-focus.md`.

## Shared schema seam

`rust/patches/rig-core/src/tool/mod.rs:400` ToolType::definition covers simple and
embedding tools, including manually implemented ToolDyn. ToolServer obtains dynamic
and static definitions through that path at `tool/server.rs:183` and `:198`.
These objects do not currently have application session identity. Inject per-agent
schema/call policy; do not couple rig-core to codelet-sessions or process-global focus.
TOOLS structs/facades alone are insufficient: providers mix direct tools and facades.

Concrete coverage routes:

- `rust/tools/src/fspec.rs:47` FspecArgs has command, serialized args and project_root;
  its nested workUnitId is a business target, not session attribution.
- `rust/providers/src/openai.rs:455` registers both direct tools and provider facades.
- `rust/providers/src/custom/rig_tool.rs:91` exposes Rhai adapter schemas; parameter
  mapping at `:99` may drop metadata unless a shared boundary extracts it first.
- `rust/tools/src/mcp.rs:975` builds initial/mid-turn MCP wrappers with remote schemas;
  definition at `:997` currently advertises that schema unchanged.
- `rust/agent-loop/src/dispatch.rs:83` adds MCP tools at turn start;
  `rust/tools/src/mcp.rs:1150` adds them mid-turn via ToolServerHandle::add_tool.
- DeepSearch and compaction independently build toolsets with ephemeral IDs at
  `rust/agent-loop/src/deep_search_handler.rs:132` / `:312` and
  `rust/agent-loop/src/compaction_builder.rs:66` / `:177`.

## Required wire contract

Every executable tool must require a top-level `work_unit_id` non-null string
containing an existing project work-unit ID or exact NONE. Omitted/null/empty values
do not mean NONE. Avoid generating an enum of all current card IDs: it becomes stale
immediately after card creation. Explain that NONE means explicitly clear active
focus, not retain the previous card.

The flat root field is reserved. Reject MCP/custom schemas with an existing business
root `work_unit_id`; never overwrite that property or silently rename it. Business
targets remain nested or use `target_work_unit_id`. No wrapper compatibility or
fallback is supported. Non-object remote schemas require an explicit unsupported-tool
error, not quiet exemption.
Preserve required lists, refs, unions and additionalProperties constraints. Validate
the final provider-serialized request, not just an intermediate definition snapshot.

## Hard boundary: not every tool uses JSON function arguments

`rust/providers/src/codex/custom_tools.rs:93` replaces a function definition with a
raw text/grammar custom tool. `:167` reconstructs only {patch: input}. Injecting a
required JSON property upstream cannot preserve mandatory attribution on this route.
`rust/providers/src/codex/native.rs:12` injects hosted web_search directly into provider
requests. Native output handling at `:117` is not local Rig tool execution.

For a literal all-tool-calls guarantee, recommended policy is JSON-function apply_patch
and disabling/replacing hosted-native routes until they offer an attribution channel.
Alternatively obtain explicit approval for a narrower guarantee or turn-level policy.
Silently synthesizing NONE is not mandatory model-supplied attribution.

## Regression matrix

- Snapshot schema contract for every provider's registered tools, including done,
  request_user_input, shell/file tools, facades and model-switched rebuilt agents.
- MCP at startup and mid-turn: object/ref/union/strict schema shapes, rejection of
  existing business root `work_unit_id`, and preservation of nested business targets
  or `target_work_unit_id`.
- Final serialized OpenAI/Anthropic/Gemini/Codex/custom requests preserve required
  metadata and still satisfy provider schema restrictions.
- Raw custom/hosted-native configuration cannot silently bypass approved strict mode.
- Auxiliary builders advertise the contract with approved ownership semantics.
- Proptest schema decoration idempotence and preservation of business schemas; insta
  snapshots for representative final requests. Runtime nonexecution is TOOL-026's scope.

Inventory provider dialect, MCP and tool-definition integration targets before adding
focused tests. Follow feature -> failing Rust integration tests -> minimal code; never
run an unscoped workspace test to check universal schema coverage.
