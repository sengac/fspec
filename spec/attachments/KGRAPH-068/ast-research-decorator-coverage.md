# AST Research: Decorator Extraction Coverage

## Current State

DeepSearch analysis confirms that decorator extraction is **already fully implemented** across all 14 language extractors via shared `metadata.rs`.

### Architecture

- `metadata.rs` defines `DecoratorStyle` enum: `AtSign`, `HashBracket`, `SquareBracket`, `None`
- `decorator_style_for_language()` maps each language to its style
- Three extraction functions: `extract_at_decorators`, `extract_hash_bracket_attrs`, `extract_square_bracket_attrs`
- All 14 extractors call `extract_function_meta()` and `extract_type_meta()` which include decorator extraction

### Language Coverage

| Language | Style | Status |
|----------|-------|--------|
| Python | AtSign | ✅ Working |
| TypeScript/JS/TSX/JSX | AtSign | ✅ Working |
| Java | AtSign | ✅ Working |
| Kotlin | AtSign | ✅ Working |
| Dart | AtSign | ✅ Working |
| Swift | AtSign | ✅ Working |
| Rust | HashBracket | ✅ Working |
| C# | SquareBracket | ✅ Working |
| Scala | None → **AtSign** | ⚠️ Needs fix |
| PHP | None → **HashBracket** | ⚠️ Needs fix |
| C | None | ✅ Correct (no decorators) |
| C++ | None | ✅ Correct (no decorators) |
| Go | None | ✅ Correct (no decorators) |
| Ruby | None | ✅ Correct (no decorators) |

### Gap: Scala and PHP

- **Scala**: Uses `@annotation` syntax identical to Java/Kotlin. Currently mapped to `None` — should be `AtSign`.
- **PHP**: PHP 8.0+ uses `#[Attribute]` syntax similar to Rust. Currently mapped to `None` — should be `HashBracket`.

### Query Infrastructure (already done by KGRAPH-067)

- `decorator` filter on ast_search: case-insensitive, strips leading `@`/`#[`/`[`
- `parameter` filter on ast_search: contains matching on parameters string
- `dispatch_helpers.rs`: `matches_decorator()` helper
