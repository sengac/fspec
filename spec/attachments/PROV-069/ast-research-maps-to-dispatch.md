# AST Research — PROV-069 maps_to dispatch extension

## Existing `default_to_internal_file` entry point
- `codelet/providers/src/custom/tool_facade.rs:181` — `pub fn default_to_internal_file(maps_to: &str, params: &Value) -> Result<InternalFileParams, CustomProviderError>` handles only `file:read / file:write / file:edit`.
- Returns `CustomProviderError::RhaiRuntimeError` for any other `maps_to` value.

## Known `maps_to` identifiers (tool_resolve.rs:22-35)
`file:read`, `file:write`, `file:edit`, `bash`, `search:grep`, `search:glob`, `ls`, `web_search:search`, `fspec`, `bridge`, `exec:run`, `hitl`

## Internal param enums/structs (codelet/tools/src/facade)
Confirmed via AstGrep:
- `pub enum InternalFileParams` — `Read`, `Write`, `Edit` (traits.rs)
- `pub enum InternalBashParams` — `Execute { command, cwd, timeout_ms }` (traits.rs:144)
- `pub enum InternalSearchParams` — `Grep { pattern, path, include, limit }`, `Glob { pattern, path }` (traits.rs:181)
- `pub enum InternalLsParams` — `List { path, offset, limit, depth }` (traits.rs:245)
- `pub enum InternalWebSearchParams` — `Search`, `OpenPage`, `FindInPage`, `CaptureScreenshot` (traits.rs:23)
- `pub struct InternalFspecParams { command, args, project_root }` (fspec_facade.rs:14)
- `pub enum InternalBridgeParams` — `Connect { url }`, `Disconnect { url }`, `List` (bridge_facade.rs:15)
- `pub enum InternalExecParams` — `Run { command, workdir, tty, yield_time_ms, max_output_tokens, timeout_secs }`, `Write`, `Poll`, `List`, `Close` (traits.rs:289)
- `pub enum InternalHitlParams` — `Request { questions: Vec<HitlQuestion> }` (traits.rs:366)
- `pub struct HitlQuestion` — `id, header, question, options: Option<Vec<HitlOption>>` (request_user_input.rs:42) — derives `Deserialize`.

## Proposed dispatch module layout
New file `codelet/providers/src/custom/tool_dispatch.rs`:
```rust
pub enum DispatchedToolParams {
    File(InternalFileParams),
    Bash(InternalBashParams),
    Search(InternalSearchParams),
    Ls(InternalLsParams),
    WebSearch(InternalWebSearchParams),
    Fspec(InternalFspecParams),
    Bridge(InternalBridgeParams),
    Exec(InternalExecParams),
    Hitl(InternalHitlParams),
}

pub fn default_to_internal(maps_to: &str, params: &Value) -> Result<DispatchedToolParams, CustomProviderError>;
```
One small `default_to_internal_<category>` helper per category. `tool_facade.rs` keeps `default_to_internal_file` (unchanged, for backwards compat with existing tests in `custom_tool_facades_tests.rs`).

## Error shape
All malformed params return `CustomProviderError::RhaiRuntimeError(msg)` where `msg` starts with `default <maps_to> mapping failed:` — matches the existing pattern.

## No impact on resolver
`tool_resolve.rs` already whitelists all 12 identifiers. No changes needed there — this work unit only adds runtime dispatch.
