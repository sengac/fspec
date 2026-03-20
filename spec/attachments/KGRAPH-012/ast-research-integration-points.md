# AST Research: KGRAPH-012 Integration Points

## Current Session Scanner Signature
```
session_scanner.rs:38 — pub async fn scan_and_index_sessions() -> Result<ScanResult, String>
```
Takes NO parameters. Needs provider_name, model_id, extraction_mode added.

## Current dispatch_index Signature
```
dispatch.rs:264 — pub async fn dispatch_index(scope: Option<&str>) -> String
```
Takes only scope. Needs provider_name and model_id parameters added.

## LLM Extraction Functions (exist but not wired)
```
llm_extraction.rs:75 — pub fn build_extraction_prompt(turns: &[&ConversationTurn]) -> String
llm_extraction.rs:65 — pub fn filter_extractable_turns(turns: &[ConversationTurn]) -> Vec<&ConversationTurn>
llm_validation.rs:64 — pub fn parse_and_validate_response(...) -> Result<Vec<GraphEntity>>
```
These are ONLY called from tests — never in production code.

## Content-Only Pattern Matching (current, insufficient)
```
session_scanner.rs:216 — fn extract_entities_from_content(...)
```
Only pattern-matches "Successfully wrote to" / "Successfully edited" lines. Ignores all actual conversation content.

## ProviderManager Pattern (from DeepSearch)
```
deep_search_handler.rs:367 — ProviderManager::with_provider_and_model(provider_name, model_id)
```
Creates fresh provider for LLM calls from within tools. Also used in:
- session_manager.rs:3456 (subordinate session creation)
- Various test files

## prompt_provider Utility
```
interactive_helpers.rs:341 — pub async fn prompt_provider(manager, prompt) -> Result<String>
```
Simple one-shot LLM call. Could be reused for extraction, but it's in codelet-cli crate, not codelet-napi.

## Key Finding
All the pieces exist: extraction prompt builder, response validator, provider creation pattern.
The gap is a ~50-line function that:
1. Creates ProviderManager
2. Batches turns using filter_extractable_turns + build_extraction_prompt
3. Calls the LLM
4. Validates response via parse_and_validate_response
5. Returns Vec<GraphEntity>
