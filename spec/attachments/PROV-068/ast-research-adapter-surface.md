# PROV-068 AST Research: RhaiToolFacadeAdapter public surface

Date: 2026-04-18
Scope: Confirm the real getters-only contract of `RhaiToolFacadeAdapter`
so the spec reconciliation (rules/architecture notes/feature docstring)
is grounded in code.

## Struct

`codelet/providers/src/custom/tool_facade.rs:57-61`

```rust
#[derive(Clone)]
pub struct RhaiToolFacadeAdapter {
    def: Arc<RhaiToolDef>,
    config: Arc<ProviderConfig>,
    loader: Arc<ScriptLoader>,
}
```

## Impl block

`codelet/providers/src/custom/tool_facade.rs:63-103`

Public methods:

- `pub fn new(def: Arc<RhaiToolDef>, config: Arc<ProviderConfig>, loader: Arc<ScriptLoader>) -> Result<Self, CustomProviderError>` — infallible today
- `pub fn name(&self) -> String`
- `pub fn parameters_schema(&self) -> &Value`
- `pub fn maps_to(&self) -> &str`
- `pub fn def(&self) -> &RhaiToolDef`
- `pub fn loader(&self) -> &Arc<ScriptLoader>`
- `pub fn config(&self) -> &Arc<ProviderConfig>`

No `impl rig::Tool` or `impl rig::tool::Tool` for this type exists
(verified with grep across the workspace — only the tests / module
docstring mention it, and the module docstring explicitly denies it).

## Search evidence

```
grep -r "impl .* for RhaiToolFacadeAdapter" codelet/
# no matches
```

```
grep -r "rig::Tool" codelet/providers/
# only in tests/custom_tool_facades_tests.rs doc-comments
# and in src/custom/tool_facade.rs:11-15 (the "deliberately not" note)
```

## Downstream usage

`codelet/providers/src/custom/custom_provider.rs:32-62` — `CustomRigAgent`
keeps a `Vec<RhaiToolFacadeAdapter>` and exposes only count/boolean
accessors. No rig::Tool or ToolDyn wrapper is constructed today. This
is consistent with `custom_provider.rs:12-14`: "The concrete
rig::agent::Agent construction is intentionally stubbed today".

## Decision

Option A (doc/spec reconciliation) is correct:

1. `rig::Tool` requires `const NAME: &'static str` — incompatible with
   runtime-defined Rhai names.
2. There is no planned rig-level wiring for custom providers today.
3. A wrapping `ToolDyn` adapter would be premature before non-file
   `maps_to` dispatch exists (tracked as PROV-069).

Action: update rules/architecture notes and the PROV-066 feature file
docstring to describe the getters-only design. Keep the canonical
tool_facade.rs docstring verbatim.
