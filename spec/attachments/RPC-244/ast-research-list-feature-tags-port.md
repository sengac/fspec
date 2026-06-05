# RPC-244 — AST Research

## Scope
Port `src/commands/list-feature-tags.ts` to Rust at `codelet/fspec-core/src/commands/list_feature_tags.rs`.

## TS surface
- `src/commands/list-feature-tags.ts:22` — `export async function listFeatureTags(featureFilePath, options): Promise<ListFeatureTagsResult>` — pure function, returns `{success, tags, message?, error?, categorizedTags?}`
- `src/commands/list-feature-tags.ts:118` — `export async function listFeatureTagsCommand(...)` — CLI wrapper that prints with chalk and process.exit

## Reference Rust port surface (list_hooks.rs)
- `codelet/fspec-core/src/commands/list_hooks.rs:109` — `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>` — canonical two-front-doors entrypoint.

## Reusable Rust modules
- `codelet/fspec-core/src/commands/list_features.rs:149` — `fn parse_feature_header(content: &str) -> Option<ParsedHeader>` — inline gherkin line scanner; extracts feature-level tags only. RPC-244 will mirror the same shape (a private inline scanner; tag block accumulated until the first `Feature:` keyword line).
- `codelet/fspec-core/src/types/tags.rs:27` — `pub struct TagsData` — already deserialises `spec/tags.json` (categories → tags). RPC-244 will read this via `std::fs::read_to_string + serde_json::from_str::<TagsData>` and silently fall back to plain tags on either failure (parity with TS bare catch lines 103-109).
- `codelet/fspec-core/src/types/tags.rs:40` — `pub struct TagCategory` — has `.name` and `.tags: Vec<Tag>`.
- `codelet/fspec-core/src/types/tags.rs:77` — `pub struct Tag` — has `.name` (the `@`-prefixed tag string).

## Key parity decisions
1. Inline scanner (not the `gherkin` crate) — consistent with list_features.rs and the simplicity of the read (we only need feature-level tags, not steps/data tables/etc.).
2. Errors stay inside the structured `{success:false, error:'...'}` result; only args_json deserialisation failures escalate via `FspecCoreError::InvalidArgs`.
3. `showCategories=true` reads `spec/tags.json` directly (no auto-create — that is list-tags' job). Bare `std::fs::read_to_string` + `serde_json::from_str` errors both silently degrade to omitting `categorizedTags`.
4. JSON output uses `serde_json::to_string_pretty` (2-space indent) — same as list_hooks.rs.
5. Field declaration order is preserved on the wire via `#[derive(Serialize)]` + `#[serde(skip_serializing_if = "Option::is_none")]` on the optional fields.
