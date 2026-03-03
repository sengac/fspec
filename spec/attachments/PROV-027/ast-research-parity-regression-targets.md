# AST Research: PROV-027 Parity & Regression Test Targets

## Rust Implementation Functions (Test Targets)

### claude_oauth.rs — Core OAuth Primitives
- `build_authorize_url(pkce)` — Authorize URL construction
- `parse_authorization_code(raw)` — Code#state parsing  
- `build_oauth_headers(access_token, existing_beta)` — Header merging with dedup
- `prefix_tool_name(name)` — mcp_ prefixing for tool definitions
- `strip_tool_name_prefix(name)` — mcp_ stripping for responses
- `rewrite_claude_url(url)` — ?beta=true URL rewriting
- `calculate_expiry(expires_in)` — Token expiry calculation

### claude_refreshing_client.rs — Token Refresh Middleware
- `RefreshingClaudeClient::new_oauth(access_token, refresh_token, expires_in_secs, endpoint)` — OAuth mode constructor
- `RefreshingClaudeClient::new_api_key()` — API key passthrough constructor
- `RefreshingClaudeClient::is_token_expired()` — Expiry check with 30s buffer
- `ensure_fresh_token()` — Double-check locking refresh (CRITICAL for concurrent test)
- `prepare_oauth_request()` — Auth header strip+inject

### claude_auth.rs — Persistence
- `get_claude_auth_path()` — Config directory resolution
- `read_claude_auth_sync()` — Sync reader for credential detection
- `read_claude_auth()` — Async reader for NAPI bindings
- `write_claude_auth(auth)` — JSON persistence

### claude.rs — Provider Integration
- `ClaudeProvider::from_oauth_tokens(access, refresh, expires_in, endpoint, model)` — OAuth provider constructor
- `ClaudeProvider::from_api_key_with_mode_and_model(key, mode, model)` — API key mode
- `ClaudeProvider::system_prompt()` — Returns CLAUDE_CODE_PROMPT_PREFIX in OAuth mode
- `build_beta_headers(model, is_oauth)` — Model-aware beta header builder

### system_prompt.rs — System Prompt Facade
- `CLAUDE_CODE_PROMPT_PREFIX` — "You are Claude Code, Anthropic's official CLI for Claude."
- ClaudeOAuth facade prepends prefix to system[0]

## TypeScript Implementation (TUI Test Targets)

### listModeHandler.ts — Key handler for provider settings
- 'e' key on Anthropic OAuth provider → starts browser OAuth (not API key editor)
- 'd' key on Anthropic OAuth provider → disconnects OAuth (clears tokens)

### useProviderSettingsState.ts — Provider status builder
- `buildNavItems()` — Generates nav items including oauth-login options
- OAuth connected status: hasKey=true, maskedKey='OAuth', source='Claude'

### modelInitializationService.ts — Model selector
- `checkClaudeOAuthTokens()` → `claudeOauthGetTokens()` NAPI binding
- Anthropic section `hasCredentials=true` when OAuth tokens exist

## opencode Reference (Parity Source)

### AnthropicAuthPlugin (index.mjs)
- `system.transform` — Prepends "You are Claude Code..." to system[0], replaces OpenCode→Claude Code
- `auth.loader.fetch` — Custom fetch interceptor:
  - Token refresh inline (check `auth.expires < Date.now()`)
  - Header merging: requiredBetas + incomingBetas (Set dedup)
  - Tool prefixing: `mcp_` prefix on tool definitions and tool_use blocks
  - Tool stripping: regex `/"name"\s*:\s*"mcp_([^"]+)"/g` on response stream
  - URL rewriting: `/v1/messages` → append `?beta=true`
  - Zero cost for OAuth max plan users
