# KGRAPH-062: AST Research — Cyclomatic Complexity Architecture

## CGC Reference Implementation (Complete Inventory)

### 1. Complexity Calculation — Per-Language `_calculate_complexity()` Methods

Each CGC language parser has a `_calculate_complexity(node)` method that walks the Tree-sitter AST.
Base complexity = 1, incremented per branching construct.

**Languages that wire calculation into function dicts:**
| Language | File | Decision Points |
|----------|------|----------------|
| Python | `languages/python.py` L86–102 | `if_statement`, `for_statement`, `while_statement`, `except_clause`, `with_statement`, `boolean_operator`, `list_comprehension`, `generator_expression`, `case_clause` |
| TypeScript | `languages/typescript.py` L125–139 | `if_statement`, `for_statement`, `while_statement`, `do_statement`, `switch_statement`, `case_statement`, `conditional_expression`, `logical_expression`, `binary_expression`, `catch_clause` |
| C | `languages/c.py` L153–170 | `if_statement`, `for_statement`, `while_statement`, `do_statement`, `switch_statement`, `case_statement`, `conditional_expression`, `logical_expression`, `binary_expression`, `goto_statement` |
| Dart | `languages/dart.py` L77–98 | `if_statement`, `for_statement`, `while_statement`, `do_statement`, `switch_statement`, `switch_case`, `if_element`, `for_element`, `conditional_expression`, `binary_expression` (only `&&`/`||`), `catch_clause` |
| Perl | `languages/perl.py` L68–87 | `if_statement`, `unless_statement`, `for_statement`, `foreach_statement`, `while_statement`, `until_statement`, `conditional_expression`, `logical_expression`, `binary_expression` |

**Languages with method defined but NOT wired:**
- Go, JavaScript, Ruby, Elixir — all have `_calculate_complexity()` but never call it in function dict construction

**Languages with hardcoded complexity:**
- C++ — always `1`, no real calculation

**Languages with NO complexity support:**
- PHP — no method at all

### 2. Graph Storage

`database_kuzu.py` — Function table includes `cyclomatic_complexity INT64` column.
`graph_builder.py` — Default injection: if `cyclomaticComplexity` not in item, set to 1.

### 3. Query Layer

`code_finder.py`:
- `get_cyclomatic_complexity(function_name, path?, repo_path?)` — single function lookup
- `find_most_complex_functions(limit=10, repo_path?)` — top-N sorted DESC

### 4. MCP Tools

Two dedicated tools:
- `calculate_cyclomatic_complexity` — single function, params: function_name, path?, repo_path?
- `find_most_complex_functions` — top-N, params: limit (default 10), repo_path?

---

## Our Architecture Decisions

### Decision 1: Single `complexity.rs` Module (DRY)

Instead of modifying each extractor with its own complexity logic, create ONE shared module:
```rust
pub fn calculate(source: &str, language: &str) -> i32
```

- Data-driven: per-language decision point configs as static arrays
- Strips comments + string literals before counting (reuse pattern from helpers.rs)
- All 14 extractors call this single function

### Decision 2: Extend `build_function_node()` Signature

Add `cyclomatic_complexity: i32` parameter to the shared helper. This causes
a compile error at all 14 call sites, ensuring every extractor is updated.

### Decision 3: One Action, Two Modes

Single `AstComplexity` action handles both use cases:
- **With `node_id`**: Single function lookup (CGC's `get_cyclomatic_complexity`)
- **Without `node_id`**: Top-N list sorted by complexity DESC (CGC's `find_most_complex_functions`)

Parameters:
```rust
AstComplexity {
    node_id: Option<String>,     // specific function slug
    limit: Option<usize>,        // default 20
    min_threshold: Option<u32>,  // only return >= this value
    path: Option<String>,        // glob filter
}
```

### Decision 4: Schema + Query Extension

- `ast-code.pg`: Add `cyclomaticComplexity: I32?` to Function node
- `ast-queries.gq`: Add `$fn.cyclomaticComplexity` to `all_functions` query return
- Requires database reset (`ast_index` with `reset: true`) after schema change

---

## Language-Specific Decision Point Configuration

### C-Family Languages (TS, JS, Java, C, C++, C#, Kotlin, Swift, Scala, PHP, Dart, Go)

**Keyword patterns** (word-boundary match in stripped source):
| Language | Keywords |
|----------|----------|
| TypeScript/JavaScript | `if`, `for`, `while`, `do`, `case`, `catch` |
| Java | `if`, `for`, `while`, `do`, `case`, `catch` |
| Go | `if`, `for`, `case`, `select` |
| C | `if`, `for`, `while`, `do`, `case`, `goto` |
| C++ | `if`, `for`, `while`, `do`, `case`, `catch`, `goto` |
| C# | `if`, `for`, `while`, `do`, `case`, `catch` |
| Kotlin | `if`, `for`, `while`, `when`, `catch` |
| Swift | `if`, `for`, `while`, `case`, `catch`, `guard` |
| Scala | `if`, `for`, `while`, `case`, `catch` |
| PHP | `if`, `elseif`, `for`, `while`, `do`, `case`, `catch` |
| Dart | `if`, `for`, `while`, `do`, `case`, `catch` |

**Operators** (literal text match): `&&`, `||`

### Python
**Keywords**: `if`, `elif`, `for`, `while`, `except`
**Operators**: ` and `, ` or ` (space-delimited to avoid substrings)

### Ruby
**Keywords**: `if`, `unless`, `case`, `when`, `while`, `until`, `for`, `rescue`
**Operators**: `&&`, `||`

### Rust
**Keywords**: `if`, `for`, `while`, `loop`
**Operators**: `&&`, `||`
**Special**: Count `=>` for match arms (but not in closures — heuristic)

---

## Files to Modify

| File | Change |
|------|--------|
| `codelet/napi/src/graph/ast_pipeline/complexity.rs` | **NEW** — Shared complexity calculator |
| `codelet/napi/src/graph/ast_pipeline/helpers.rs` | Update `build_function_node()` signature |
| `codelet/napi/src/graph/ast_pipeline/mod.rs` | Add `pub mod complexity;` |
| `codelet/napi/src/graph/ast_pipeline/ast_*_extractor.rs` (×14) | Call `complexity::calculate()`, pass to `build_function_node()` |
| `codelet/napi/schemas/ast-code.pg` | Add `cyclomaticComplexity: I32?` to Function |
| `codelet/napi/schemas/ast-queries.gq` | Add `$fn.cyclomaticComplexity` to all_functions |
| `codelet/napi/src/graph/ast_complexity.rs` | **NEW** — Dispatch module |
| `codelet/napi/src/graph/mod.rs` | Add `pub mod ast_complexity;` |
| `codelet/tools/src/graph_search/types.rs` | Add `AstComplexity` variant |
| `codelet/napi/src/graph_search_handler.rs` | Wire dispatch |
| `codelet/tools/src/graph_search/mod.rs` | Update tool documentation |
| `codelet/napi/tests/ast_complexity_test.rs` | **NEW** — Integration tests |
