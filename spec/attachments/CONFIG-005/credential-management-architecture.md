# CONFIG-005: Move Credential Management to Rust

## Problem Statement

Currently, credential management is split between TypeScript and Rust:

1. **TypeScript** reads `~/.fspec/credentials/credentials.json`
2. **TypeScript** passes API key to Rust at session creation time
3. **Rust** sets environment variable and creates ProviderManager

This creates a **stale credentials problem**:
- If user updates API key via TUI, existing Rust sessions don't see the new key
- The key is baked into the session at creation time
- No way to refresh credentials without destroying and recreating the session

## Current Implementation (CONFIG-004)

### TypeScript Side (`src/utils/credentials.ts`)
```typescript
export async function getProviderConfig(providerId: string): Promise<ProviderConfigResult> {
  // 1. Try credentials file first
  const credentials = await loadCredentials();
  const providerCred = credentials.providers[providerId];
  if (providerCred?.apiKey) {
    return { apiKey: providerCred.apiKey, source: 'file' };
  }
  
  // 2. Try environment variables
  // 3. Try .env file
  // ...
}
```

### TypeScript Side (`src/tui/components/AgentView.tsx`)
```typescript
// At session creation time:
const providerConfig = await getProviderConfig(providerId);
apiKey = providerConfig.apiKey;

await sessionManagerCreateWithId(
  activeSessionId,
  modelPath,
  project,
  sessionName,
  apiKey  // <-- Passed once at creation, never updated
);
```

### Rust Side (`codelet/napi/src/session_manager.rs`)
```rust
pub async fn create_session_with_id(..., api_key: Option<&str>) -> Result<()> {
    // Sets env var ONCE at session creation
    if let Some(key) = api_key {
        let env_var_name = match *registry_provider {
            "anthropic" => "ANTHROPIC_API_KEY",
            // ...
        };
        std::env::set_var(env_var_name, key);  // <-- Set once, never refreshed
    }
    
    // ProviderManager uses env var
    let mut provider_manager = ProviderManager::with_model_support().await?;
}
```

## Proposed Solution

### Move ALL credential resolution to Rust

1. **Rust reads credentials file directly**
   - Add `codelet-credentials` crate or module
   - Reads `~/.fspec/credentials/credentials.json`
   - Implements same priority chain as TypeScript

2. **Rust checks credentials before each API call**
   - Or at minimum, on session resume
   - Compare file mtime to detect changes
   - Refresh ProviderManager if credentials changed

3. **TypeScript only writes to file**
   - `saveCredential()` - writes to JSON file
   - `deleteCredential()` - removes from JSON file
   - Does NOT pass credentials to NAPI

4. **Remove api_key parameter from NAPI functions**
   - `sessionManagerCreateWithId` no longer needs 5th param
   - Rust handles credential resolution internally

### Credential Priority Chain (in Rust)

```rust
fn resolve_credential(provider_id: &str, project_dir: &Path) -> Option<String> {
    // 1. Credentials file (~/.fspec/credentials/credentials.json)
    if let Some(key) = read_credentials_file(provider_id) {
        return Some(key);
    }
    
    // 2. Environment variable (already in process)
    if let Ok(key) = std::env::var(get_env_var_name(provider_id)) {
        return Some(key);
    }
    
    // 3. .env file in project directory
    if let Some(key) = read_dotenv_file(project_dir, provider_id) {
        return Some(key);
    }
    
    None
}
```

### Credential Refresh Strategy

Option A: **Check before each API call**
- Most robust
- Small overhead (file stat)
- Immediate credential updates

Option B: **Check on session resume**
- Less overhead
- Credentials refresh when switching sessions
- Good enough for most use cases

Option C: **File watcher**
- React to file changes
- More complex
- May be overkill

**Recommendation: Option B** - Check on session resume. Simple, effective, covers the main use case (user updates key, switches back to session).

## Files to Modify

### Rust Side
- `codelet/napi/src/credentials.rs` (new) - Credential resolution
- `codelet/napi/src/session_manager.rs` - Remove api_key param, call credential resolver
- `codelet/providers/src/lib.rs` - May need to support credential refresh

### TypeScript Side
- `src/utils/credentials.ts` - Keep save/delete, remove resolution from NAPI calls
- `src/tui/components/AgentView.tsx` - Remove api_key passing
- `src/tui/services/sessionService.ts` - Remove api_key passing

## Testing Strategy

1. **Unit tests for Rust credential resolution**
   - Priority chain works correctly
   - File parsing handles edge cases
   - Missing file returns None

2. **Integration tests**
   - Create session, update credentials, verify new key used on resume
   - Multiple providers with different credential sources

3. **Manual testing**
   - TUI credential dialog updates
   - Session continues working after credential change

## Migration Path

1. Implement Rust credential resolution (no breaking changes)
2. Update Rust session creation to use internal resolution
3. Deprecate api_key parameter (keep for backwards compat)
4. Remove TypeScript credential passing
5. Remove api_key parameter from NAPI

## Security Considerations

- Credentials file permissions (600) enforced by TypeScript on write
- Rust should verify permissions on read (warn if too open)
- Never log credentials - only masked versions
- Credentials stay in Rust, never returned to TypeScript via NAPI
