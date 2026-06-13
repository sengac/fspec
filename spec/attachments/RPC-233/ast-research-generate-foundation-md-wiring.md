# AST Research — RPC-233 generate-foundation-md Rust port

Performed with AstGrep (language: rust) + ripgrep over `codelet/fspec-core/src`
and `codelet/fspec/src` to confirm the implementation and ALL call sites
("WHO CALLS THIS?") before closing the work unit.

## Core entry points
- `commands::generate_foundation_md::run(args_json, project_root)` — dispatcher/CLI JSON wrapper.
- `commands::generate_foundation_md::generate(project_root, output_path)` — core routine (read foundation.json → mermaid pre-check → render → write).
- `commands::generate_foundation_md::regenerate(project_root)` — best-effort auto-regen, swallows errors.
  - AstGrep `pub fn regenerate($$$ARGS) { $$$BODY }` → `generate_foundation_md.rs:155`.

## Rendering (split <300 lines)
- `generators::foundation_md::generate_foundation_md(&Value) -> Result<String, String>` (`generators/foundation_md.rs:37`).
- `generators/foundation_md_diagrams.rs` — Event-Storm/diagram rendering helpers.

## Two front doors (verified)
- LLM dispatcher: `codelet/fspec-core/src/dispatch.rs:446` → `commands::generate_foundation_md::run(...)`.
- clap CLI bridge: `codelet/fspec/src/generate_foundation_md.rs:35` → `generate_foundation_md::run(...)`; registered in `codelet/fspec/src/main.rs:1906`.

## regenerate() call sites — the six Event-Storm foundation commands (wired AFTER atomic write)
- `add_foundation_bounded_context.rs:134`
- `remove_foundation_bounded_context.rs:160`
- `add_command_to_foundation.rs:145`
- `remove_command_from_foundation.rs:145`
- `add_aggregate_to_foundation.rs:228`
- `remove_aggregate_from_foundation.rs:203`

## Conclusion
Implementation is complete and fully wired end-to-end through both front doors
and all six mutation commands. No orphaned/uncalled code. Byte-for-byte parity
with the TypeScript output verified during the parity sweep (FOUNDATION.md =
30592 bytes identical).
