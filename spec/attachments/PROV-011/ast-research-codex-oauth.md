# AST Research: Codex OAuth Login Flow (PROV-011)

## Existing Codex Provider Public API (codelet/providers/src/codex/)

### mod.rs - CodexProvider
- `pub fn new() -> Result<Self, ProviderError>` (line 63) - Creates provider from ~/.codex/auth.json
- `pub fn from_api_key(api_key: &str, model: &str) -> Result<Self, ProviderError>` (line 79)
- `pub fn client() -> &openai::CompletionsClient` (line 105)
- `pub fn create_rig_agent(session_id, preamble, thinking_config) -> Agent` (line 123)

### codex_auth.rs - Auth File/Keychain
- `pub fn get_auth_path() -> PathBuf` (line 69) - Returns ~/.codex/auth.json path
- `pub fn read_codex_auth() -> Result<Option<CodexAuthJson>>` (line 124) - Reads from keychain then file
- `pub fn write_codex_auth(auth: &CodexAuthJson) -> Result<()>` (line 136)
- `pub fn get_codex_api_key_sync() -> Result<String>` (line 218) - Full refresh+exchange flow
- `fn compute_store_key(codex_home) -> String` (line 76) - macOS keychain key
- `fn read_keychain_credentials() -> Result<Option<CodexAuthJson>>` (line 90)
- `fn read_file_credentials() -> Result<Option<CodexAuthJson>>` (line 109)

## Structs:
- `CodexAuthJson` { openai_api_key: Option<String>, tokens: Option<CodexTokens>, last_refresh: Option<String> }
- `CodexTokens` { id_token, access_token, refresh_token, account_id }

## Key Integration Points:
1. `credentials.rs` - `has_codex_auth()` calls `read_codex_auth()` 
2. `manager.rs` - `get_codex()` calls `CodexProvider::new()`
3. Provider uses `openai::CompletionsClient` with standard OpenAI API key

## What Needs to Change for OAuth Login:
1. New `codex_oauth.rs` module with PKCE, browser OAuth server, device auth
2. `CodexProvider::new()` needs to check for OAuth tokens and use Codex API endpoint
3. NAPI bindings for `codex_oauth_browser_login()` and `codex_oauth_device_login()`
4. Custom HTTP client that rewrites URLs and adds Bearer/account headers
