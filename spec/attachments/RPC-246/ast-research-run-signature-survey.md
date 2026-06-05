# AST Research — RPC-246: Port list-foundation-sections to Rust

Pattern queried: `pub async fn run($$$ARGS) -> Result<String, FspecCoreError> { $$$BODY }`
Scope: `codelet/fspec-core/src/commands/`

## Findings

The fspec-core crate uses TWO `run` signature shapes for command modules:

1. **Stub shape** (NotYetPorted):
   `pub async fn run(_args_json: &str) -> Result<String, FspecCoreError>`
   - Used by ~140 unported command stubs (including the current `list_foundation_sections` at line 6).

2. **Ported shape** (project-root-aware):
   `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`
   - Used by all ported commands: `list_hooks.rs:109`, `list_prefixes.rs:41`, `list_features.rs:71`, `list_attachments.rs:70`, `list_epics.rs:41`, `list_tags.rs:52`, `list_work_units.rs:42`.

## Decision for RPC-246

The new `list_foundation_sections::run` MUST adopt the **ported shape** —
`pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>` —
for dispatcher contract parity, even though the command performs zero filesystem I/O
(the section list is a static constant). The `project_root` parameter will be
accepted and ignored. This is identical to how `list_hooks` accepts but only
reads from `project_root.join("spec/fspec-hooks.json")`; here we accept it and
never touch it.

## Reference Port

The closest analogue is `list_hooks` (RPC-247): same dispatcher shape, same
`format?: 'text'|'json'` arg deserialization, same `serde_json::to_string_pretty`
path for JSON output, and similar text rendering with a static-shaped layout.
The integration test file `tests/list_hooks.rs` is the byte-for-byte template
for `tests/list_foundation_sections.rs`.
