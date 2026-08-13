# AST Research: Provider Module Analysis

## Command Executed
```bash
fspec research --tool=ast --operation=list-functions --file=src/providers/mod.rs
```

## Results

### Functions Found
| Function | Line | Description |
|----------|------|-------------|
| `fmt` | 20 | Display trait impl for ProviderType enum |

### Key Observations

1. **ProviderType Enum**: Defines three provider types: `Anthropic`, `OpenAI`, `Google`
2. **LlmProvider Trait**: Defined but has no implementations yet
3. **Trait Methods Required**:
   - `name() -> &str`
   - `model() -> &str`
   - `context_window() -> usize`
   - `max_output_tokens() -> usize`
   - `supports_caching() -> bool`
   - `supports_streaming() -> bool`
   - `complete(&self, messages: &[Message]) -> Result<String>`

### Implementation Location
New ClaudeProvider should be created at: `src/providers/claude.rs`

### Module Structure
```
src/providers/
├── mod.rs          # LlmProvider trait + ProviderType enum (exists)
└── claude.rs       # ClaudeProvider impl (to be created)
```

## Code Context
```rust
// From src/providers/mod.rs
pub enum ProviderType {
    Anthropic,
    OpenAI,
    Google,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::Anthropic => write!(f, "anthropic"),
            ProviderType::OpenAI => write!(f, "openai"),
            ProviderType::Google => write!(f, "google"),
        }
    }
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &str;
    fn model(&self) -> &str;
    fn context_window(&self) -> usize;
    fn max_output_tokens(&self) -> usize;
    fn supports_caching(&self) -> bool;
    fn supports_streaming(&self) -> bool;
    async fn complete(&self, messages: &[crate::agent::Message]) -> Result<String>;
}
```
