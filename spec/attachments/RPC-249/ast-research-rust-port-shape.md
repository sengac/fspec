# AST Research — RPC-249 `list-scenario-tags` Rust Port

Performed via AstGrep over `codelet/fspec-core/src/commands/` searching for the canonical
`pub async fn run(args_json, project_root) -> Result<String, FspecCoreError>` signature shape
that all ported commands MUST converge on (per RPC-003 §7/§11 two-front-doors invariant).

## Findings

### Already-ported commands using the canonical 2-arg signature

These five commands have completed the port and serve as integration-shape references:

| Command | File | run signature |
|---------|------|---------------|
| `list-attachments` | `codelet/fspec-core/src/commands/list_attachments.rs:70` | `(args_json: &str, project_root: &Path)` |
| `list-work-units` | `codelet/fspec-core/src/commands/list_work_units.rs:42` | `(args_json: &str, project_root: &Path)` |
| `list-features` | `codelet/fspec-core/src/commands/list_features.rs:71` | `(args_json: &str, project_root: &Path)` |
| `list-epics` | `codelet/fspec-core/src/commands/list_epics.rs:41` | `(args_json: &str, project_root: &Path)` |
| `list-hooks` | `codelet/fspec-core/src/commands/list_hooks.rs:109` | `(args_json: &str, project_root: &Path)` |
| `list-prefixes` | `codelet/fspec-core/src/commands/list_prefixes.rs:41` | `(args_json: &str, project_root: &Path)` |
| `list-tags` | `codelet/fspec-core/src/commands/list_tags.rs:52` | `(args_json: &str, project_root: &Path)` |

### Stubs still using the 1-arg `_args_json` signature

~140 stubs remain on the old `pub async fn run(_args_json: &str)` shape that returns
`FspecCoreError::NotYetPorted`. `list_scenario_tags.rs:6` is among them.

### Reference Port Shape (chosen: `list_hooks.rs`)

`list_hooks.rs:109` is the closest analog to `list-scenario-tags`:
- Same shape: read a file from the project root, parse, project into a result struct.
- Uses `#[derive(Serialize)]` to control field order (no `serde_json::json!{}` BTreeMap re-sort).
- Swallows malformed JSON via `match serde_json::from_str(&raw) { Ok(p) => p, Err(_) => ... }`.
- Splits text-format render into a dedicated function (`render_text`).

### Inline-scanner pattern (from `list_features.rs:71`)

`list_features.rs` introduces `parse_feature_header` — an inline gherkin line-scanner that
extracts (Feature name, top-level tags, scenario count) without depending on the upstream
`gherkin` crate (which is NOT in the workspace Cargo.toml today). RPC-249 will reuse the same
pattern for `(scenario name → preceding `@tag` block)` extraction.

## Decision

- **Adopt the 2-arg signature** `pub async fn run(args_json: &str, project_root: &Path)` shared
  with the seven already-ported `list-*` commands. The orchestrator wires the dispatcher
  match-arm to pass `req.project_root` exactly as it does for `list-hooks`.
- **Use an inline scanner** (parity with `list_features.rs`) rather than introducing the
  `gherkin` crate dependency. The scanner detects the Feature header, top-level Scenario
  keyword lines, and accumulates `@tag` lines immediately preceding each Scenario header.
- **Split text vs JSON rendering** into separate helpers, matching `list_hooks.rs`.
