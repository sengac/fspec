# AST Research — RPC-375 (umbrella consolidation)

This umbrella carries no implementation of its own. The AST research that
informed the design lives on each child work unit (RPC-376–380). This file
consolidates the public-surface analysis of the `codelet/attachment-viewer`
crate as it stood after all children were delivered, confirming the agreed
architecture was preserved.

## Public functions (`AstGrep` `pub fn $NAME($$$ARGS) -> $RET { $$$BODY }`)

| File | Function | Role |
|------|----------|------|
| `src/state.rs` | `ViewerState::new` / `cwd` | Clone Arc state newtype (preserved) |
| `src/lib.rs` | `build_router` / `build_router_with_config` | axum router factory (preserved) |
| `src/markdown/escape.rs` | `html_escape` | HTML escaping parity |
| `src/markdown/render.rs` | `render_markdown` | pulldown-cmark → HTML pipeline |
| `src/markdown/slug.rs` | `slugify` | RPC-376 heading anchor IDs |
| `src/markdown/template/mod.rs` | `viewer_template` | template entrypoint (signature unchanged) |
| `src/markdown/template/mermaid_modal.rs` | `modal_script` | RPC-378 mermaid modal |
| `src/markdown/autolink.rs` | `autolink_events` | RPC-379 bare-URL/email autolink |
| `src/handlers/path.rs` | `validate_path` | path traversal guard |

## Findings

- `build_router` factory, `ViewerState` Clone-Arc newtype, and the
  `viewer_template(title, content_html)` signature are all intact — the
  HARD CONSTRAINTS held across every child.
- Rendering-correctness surface (slug, autolink, render) matches the parity
  scope; no child introduced an unconnected module.
- All source files remain under the 300-line limit.
