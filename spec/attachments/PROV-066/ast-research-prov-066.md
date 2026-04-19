# PROV-066 AST Research — Tool Facade Integration

Conducted 2026-04-17.

## Reuse targets

- `codelet/tools/src/facade/traits.rs` — 8 facade traits (ToolFacade, FileToolFacade, BashToolFacade, SearchToolFacade, LsToolFacade, FspecToolFacade, etc.) and their Internal*Params enums.
- `codelet/tools/src/facade/mod.rs` — central dispatch between facades and base tools. Reference for wiring.
- `codelet/providers/src/custom/script_loader.rs` — ScriptLoader + engine access.
- `codelet/providers/src/custom/rhai_call.rs` — spawn_blocking helpers.
- `codelet/providers/src/custom/conversion.rs` — Rhai ↔ serde_json conversion.

## New files under `codelet/providers/src/custom/`

- `tool_facade.rs` — `pub struct RhaiToolDef { name, description, parameters: serde_json::Value, maps_to: String }` + `pub struct RhaiToolFacadeAdapter { def: Arc<RhaiToolDef>, provider_name: String, engine: Arc<Engine>, ast: Arc<AST>, config: Dynamic, dispatcher: Arc<dyn ToolDispatcher> }` implementing `rig::Tool`.
- `tool_resolve.rs` — `pub fn resolve_tools(config: &mut ProviderConfig, loader: &ScriptLoader) -> Result<Vec<RhaiToolDef>, ToolError>`. Calls Rhai `define_tools(config)` inside spawn_blocking; validates maps_to; falls back to preset on absence or error.
- `tool_presets.rs` — `pub fn preset_tools(style: ToolStyle) -> Vec<RhaiToolDef>`. Returns claude/openai/gemini/codex preset tool lists.
- `tool_dispatch.rs` — `pub trait ToolDispatcher` + concrete dispatcher that turns maps_to + params into calls on base tool implementations from codelet-tools.

## Changes to existing PROV-062 file

- `codelet/providers/src/custom/config.rs` — add `pub resolved_tools: Option<Vec<RhaiToolDef>>` (or similar) so system prompt functions can reference it. Keep backward-compat: tests from PROV-062 don't check this field.

## Known maps_to identifiers (from research §2 and PROV-061 rules)

`file:read, file:write, file:edit, bash, search:grep, search:glob, ls, web_search:search, fspec, bridge, exec:run, hitl`

## Tests

For PROV-066 focus on the facade-adapter surface and resolver, NOT actual tool execution (base tool execution = PROV-067 integration). The end-to-end dispatcher call path can be mocked with a minimal test implementation of ToolDispatcher.
