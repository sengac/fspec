# TOOL-026 AST research

AstGrep Rust query `impl $NAME { $$$BODY }` on
`rust/sessions/src/focus_reconciliation.rs` identified SessionManager at line 13
and BackgroundSession at line 134. The manager provides revisioned admission and
status reconciliation; BackgroundSession retains independent authorization.

The Rig ToolMiddleware seam precedes ToolDyn dispatch, including manually
implemented MCP and facade tools. TOOL-025 currently installs schema-only
middleware. Runtime validation must replace that policy at construction, not
depend on optional lifecycle hooks or business argument targets.
