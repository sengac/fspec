# CLI-001: Provider Manager Implementation Notes

## Problem Statement

Currently, `src/cli/mod.rs:99` hardcodes `ClaudeProvider::new()`, ignoring the `--provider` CLI argument. Users cannot select which LLM provider to use (Claude, OpenAI, Codex, Gemini).

## Reference Implementation

The TypeScript codelet project (`~/projects/codelet`) has a mature `ProviderManager` class that we should port to Rust.

### Key Files to Study:
- `~/projects/codelet/src/agent/provider-manager.ts` - Main ProviderManager class
- `~/projects/codelet/src/agent/codex-provider.ts` - Codex provider integration
- `~/projects/codelet/src/agent/claude-provider.ts` - Claude Code OAuth integration
- `~/projects/codelet/src/agent/runner.ts` - How runner uses ProviderManager

## Provider Manager Requirements

### 1. Credential Detection (Sync)
Must detect available credentials from:
- **Environment variables**:
  - `ANTHROPIC_API_KEY` → Claude
  - `OPENAI_API_KEY` → OpenAI
  - `GOOGLE_GENERATIVE_AI_API_KEY` → Gemini
- **Auth files**:
  - `~/.codex/auth.json` → Codex (OAuth)
  - `~/.claude/auth.json` → Claude Code (OAuth)

### 2. Default Provider Selection
Priority order (matching codelet):
1. Claude (API key if available)
2. Claude Code (OAuth if `~/.claude/auth.json` exists)
3. Gemini (if `GOOGLE_GENERATIVE_AI_API_KEY` set)
4. Codex (if `~/.codex/auth.json` exists)
5. OpenAI (if `OPENAI_API_KEY` set)

Throw error if NO credentials available.

### 3. CLI Integration
- Parse `--provider <name>` argument
- Validate provider is available (has credentials)
- Override default provider if specified
- Error with helpful message if provider unavailable

### 4. Provider Instantiation
Return the appropriate provider trait object:
```rust
pub enum ProviderInstance {
    Claude(ClaudeProvider),
    OpenAI(OpenAIProvider),
    Codex(CodexProvider),
    Gemini(GeminiProvider),
}
```

Or use trait objects:
```rust
pub fn get_provider(&self) -> Result<Box<dyn LlmProvider>>
```

## Implementation Plan

### Phase 1: Credential Detection Module
File: `src/providers/credentials.rs`

```rust
pub struct ProviderCredentials {
    pub claude_api_key: Option<String>,
    pub claude_code_available: bool,
    pub openai_api_key: Option<String>,
    pub codex_available: bool,
    pub gemini_api_key: Option<String>,
}

pub fn detect_credentials() -> ProviderCredentials {
    // Check env vars
    // Check auth files
    // Return struct
}
```

### Phase 2: Provider Manager
File: `src/providers/manager.rs`

```rust
pub struct ProviderManager {
    credentials: ProviderCredentials,
    current_provider: ProviderType,
}

pub enum ProviderType {
    Claude,
    OpenAI,
    Codex,
    Gemini,
}

impl ProviderManager {
    pub fn new() -> Result<Self> { }
    pub fn with_provider(provider_name: &str) -> Result<Self> { }
    pub fn get_provider(&self) -> Result<Box<dyn LlmProvider>> { }
}
```

### Phase 3: CLI Integration
File: `src/cli/mod.rs`

Replace:
```rust
let provider = ClaudeProvider::new()?;
```

With:
```rust
use crate::providers::ProviderManager;

let manager = if let Some(provider_name) = cli.provider {
    ProviderManager::with_provider(&provider_name)?
} else {
    ProviderManager::new()?
};
let provider = manager.get_provider()?;
```

### Phase 4: Error Messages
Helpful errors:
- "No provider credentials found. Set ANTHROPIC_API_KEY or run 'codex auth login'"
- "Provider 'codex' not available. No credentials found at ~/.codex/auth.json"
- "Available providers: claude, codex. Requested: openai"

## Testing Strategy

### Unit Tests
- `credentials::detect_credentials()` with mocked env vars
- `ProviderManager::new()` default selection
- `ProviderManager::with_provider()` override logic
- Error cases (no credentials, invalid provider)

### Integration Tests
- CLI argument parsing with `--provider codex`
- End-to-end provider selection and agent execution
- Multi-provider scenario (multiple credentials available)

## Acceptance Criteria

1. ✅ CLI accepts `--provider <name>` argument
2. ✅ Default provider auto-selected based on priority
3. ✅ Codex provider usable via `--provider codex`
4. ✅ Claude provider usable via `--provider claude`
5. ✅ OpenAI provider usable via `--provider openai`
6. ✅ Gemini provider usable via `--provider gemini`
7. ✅ Error if no credentials available
8. ✅ Error if requested provider has no credentials
9. ✅ Provider selection respects priority order
10. ✅ Works with both API keys and OAuth auth files

## Files to Create/Modify

### Create:
- `src/providers/credentials.rs` - Credential detection
- `src/providers/manager.rs` - ProviderManager class
- `tests/provider_manager_test.rs` - Comprehensive tests

### Modify:
- `src/providers/mod.rs` - Export new modules
- `src/cli/mod.rs` - Use ProviderManager instead of hardcoded ClaudeProvider

## Related Work Units
- PROV-004 (DONE) - Codex Provider implementation
- This builds on existing provider infrastructure

## Timeline Estimate
- Story points: 5
- Expected duration: 2-3 hours
- Complexity: Medium (credential detection + trait object handling)
