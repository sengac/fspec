# AST Research: NAPI Session Functions

## Existing Session Functions in Rust (codelet/napi/src/session.rs)

```rust
pub async fn new_with_model(model_string: String) -> Result<Self>
pub async fn new_with_credentials(model_string: String, provider_config: NapiProviderConfig) -> Result<Self>
pub async fn compact(&self) -> Result<CompactionResult>
pub async fn switch_provider(&self, provider_name: String) -> Result<()>
pub async fn select_model(&self, model_string: String) -> Result<()>
pub async fn prompt(&self, ...) -> Result<String>
```

## Key Observations

1. Session functions follow `pub async fn` pattern
2. Functions that return data use `Result<DataType>` return type
3. Methods on BackgroundSession use `&self` parameter
4. NAPI automatically converts Rust structs to TypeScript interfaces

## Implementation Plan for Anchor Functions

Need to add these functions to `codelet/napi/src/session.rs`:
1. `pub async fn get_anchor_points(&self) -> Result<Vec<AnchorPoint>>`
2. `pub async fn get_turn_details(&self, turn_index: usize) -> Result<Option<TurnDetails>>`

These will be exposed through the BackgroundSession class in TypeScript bindings.