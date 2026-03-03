# AST Research: PROV-012 Anthropic OAuth Parent

This is a parent/umbrella story. All AST research was performed in child cards:

- **PROV-020**: `spec/attachments/PROV-020/ast-research-claude-oauth-patterns.md`
- **PROV-021**: Browser OAuth server implementation
- **PROV-022**: Device auth flow 
- **PROV-023**: Token refresh client
- **PROV-024**: NAPI bindings
- **PROV-025**: TUI integration
- **PROV-026**: Routing and model availability
- **PROV-027**: Parity regression hardening

## Key Files (Summary)

| File | Purpose |
|------|---------|
| `codelet/providers/src/claude_oauth.rs` | Core OAuth: PKCE, authorize URL, token exchange, refresh, headers |
| `codelet/providers/src/claude_oauth_server.rs` | Browser OAuth local HTTP server |
| `codelet/providers/src/claude_headless_login.rs` | Headless login flow |
| `codelet/providers/src/claude_auth.rs` | Auth persistence (claude_auth.json) |
| `codelet/providers/src/claude.rs` | RefreshingClaudeClient with OAuth integration |
| `codelet/napi/src/claude_oauth_napi.rs` | NAPI bindings for TUI |
| `src/tui/hooks/useProviderSettingsState.ts` | TUI state management |
| `src/tui/components/ProviderSettingsPanel.tsx` | TUI rendering |
