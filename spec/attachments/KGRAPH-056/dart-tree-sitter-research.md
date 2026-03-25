# Dart tree-sitter-dart Research Findings

## Background: Why ast-grep Dropped Dart

### Timeline

| Date | Event | Version |
|------|-------|---------|
| Jan 2023 | Dart added to ast-grep via `tree-sitter-dart` v0.0.2 | ast-grep 0.2.x |
| May 2024 | ast-grep forks it to `ast-grep/tree-sitter-dart` v0.0.4 | ast-grep ~0.26.x |
| Nov 2024 | **Dart REMOVED** from ast-grep — commit `cd25a62` | ast-grep 0.30.0 |
| Mar 2026 | New `tree-sitter-dart` v0.1.0 published by `nielsenko/tree-sitter-dart` | N/A |

### Why It Was Removed (commit `cd25a62`)

Commit message:
```
fix: remove builtin dart support

BREAKING CHANGE: dart does not meet ast-grep's builtin language criteria
```

ast-grep's builtin language criteria (from their [add-lang guide](https://ast-grep.github.io/contributing/add-lang.html)):
1. **Language popularity** — must be widely used (TIOBE/GitHub Octoverse)
2. **Grammar quality** — well-written, up-to-date, regularly maintained
3. **Grammar size** — binary budget <10MB compressed
4. **Published on crates.io**

The old `tree-sitter-dart` v0.0.4 had known issues:
- **Issue #1404**: `tree_sitter_dart` had an older `tree-sitter` dependency version causing `From<tree_sitter::Language>` trait bound failures
- **Issue #172**: Parser outputs multiple AST nodes for function declarations instead of one single node (a "peculiarity" for pattern matching)
- The crate was published by `ast-grep/tree-sitter-dart` — a fork they maintained themselves, which became a maintenance burden

### Our Version

We use `ast-grep-language = "0.40.5"` (resolved in Cargo.lock). Dart was removed in 0.30.0, so **no version from 0.30 onwards has built-in Dart**. The latest upstream (0.42.0) still does not have it.

---

## New tree-sitter-dart v0.1.0

### Key Facts

| Property | Value |
|----------|-------|
| Crate | `tree-sitter-dart` |
| Version | 0.1.0 |
| Published | March 11, 2026 |
| Repository | [github.com/nielsenko/tree-sitter-dart](https://github.com/nielsenko/tree-sitter-dart) |
| Maintainer | Kasper Overgård Nielsen (`nielsenko`) — different from old v0.0.4 maintainer |
| License | MIT |
| Downloads | ~7,100 (as of March 25, 2026) |

### Dependency Graph

```toml
[dependencies]
tree-sitter-language = "0.1"    # Modern bridge crate — same as all current ast-grep parsers

[build-dependencies]
cc = "1.2"

[dev-dependencies]
tree-sitter = "0.25.4"          # Only a dev-dependency
```

**Compatibility**: Uses `tree-sitter-language = "0.1"` which is the same bridge crate used by all parsers in ast-grep 0.40+. Our Cargo.lock resolves `tree-sitter-language` to v0.1.7. This means `tree-sitter-dart` 0.1.0 is **fully compatible** with our dependency chain.

### Comparison with Old Version

| Aspect | v0.0.4 (old) | v0.1.0 (new) |
|--------|-------------|-------------|
| Maintainer | ast-grep fork | nielsenko (community) |
| tree-sitter dep | `tree-sitter = "0.20"` (direct) | `tree-sitter-language = "0.1"` (bridge) |
| Compatibility | Broke with tree-sitter 0.22+ | Works with tree-sitter 0.22–0.26 |
| API surface | `language()` → `tree_sitter::Language` | `LANGUAGE` const → `tree_sitter_language::LanguageFn` |

---

## Integration Approach

Since ast-grep's `SupportLang` enum is defined in the `ast-grep-language` crate (which we use as a dependency), we **cannot** add a new enum variant without forking. Instead, we integrate at our tool layer.

### Architecture

```
codelet/tools/src/astgrep.rs          — AstGrepTool
codelet/tools/src/astgrep_refactor.rs — AstGrepRefactorTool
                ↓
    parse_language("dart") → DartLang (custom struct)
                ↓
    Uses ast-grep-core directly with tree-sitter-dart LANGUAGE
```

### Implementation Plan

1. **Add dependency** in `codelet/tools/Cargo.toml`:
   ```toml
   tree-sitter-dart = { version = "0.1.0", optional = false }
   ```

2. **Create a Dart language adapter** that implements ast-grep's `Language` trait:
   ```rust
   use ast_grep_core::language::Language;
   use ast_grep_core::tree_sitter::{StrDoc, TSLanguage, LanguageExt};

   #[derive(Clone, Copy, Debug)]
   pub struct DartLang;

   impl Language for DartLang {
       fn kind_to_id(&self, kind: &str) -> u16 {
           self.get_ts_language().id_for_node_kind(kind, true)
       }
       fn field_to_id(&self, field: &str) -> Option<u16> {
           self.get_ts_language().field_id_for_name(field).map(|f| f.get())
       }
       fn expando_char(&self) -> char {
           'µ'  // Dart identifiers don't accept $
       }
       fn pre_process_pattern<'q>(&self, query: &'q str) -> Cow<'q, str> {
           pre_process_pattern('µ', query)
       }
       fn build_pattern(&self, builder: &PatternBuilder) -> Result<Pattern, PatternError> {
           builder.build(|src| StrDoc::try_new(src, self.clone()))
       }
   }

   impl LanguageExt for DartLang {
       fn get_ts_language(&self) -> TSLanguage {
           tree_sitter_dart::LANGUAGE.into()
       }
   }
   ```

3. **Modify `parse_language()`** to handle "dart" before falling through to `SupportLang::from_str()`:
   ```rust
   fn parse_language(lang: &str) -> Option<LanguageChoice> {
       match lang.to_lowercase().as_str() {
           "dart" => Some(LanguageChoice::Dart(DartLang)),
           other => other.parse::<SupportLang>().ok().map(LanguageChoice::AstGrep),
       }
   }
   ```

4. **Add "dart" to `get_extensions()`** and update error messages / supported language lists.

5. **Update LLM tool descriptions** in the TypeScript tool schema layer.

### Important: expando_char

Dart uses `$` for string interpolation (`'Hello $name'`, `'${expr}'`), so `$VAR` in a pattern is **not** valid Dart syntax. We must use an expando char — `µ` is the standard choice used by Go, Python, Ruby, etc. in ast-grep. The `pre_process_pattern()` function replaces `$` with `µ` before parsing.

---

## Missing Languages While We're At It

Our `get_extensions()` match arms in astgrep.rs are missing entries for three languages that **do** exist in our `ast-grep-language 0.40.5`:

| Language | SupportLang variant | Extensions | Status |
|----------|-------------------|------------|--------|
| Solidity | `SupportLang::Solidity` | `.sol` | Missing from get_extensions() |
| Nix | `SupportLang::Nix` | `.nix` | Missing from get_extensions() |
| Hcl | `SupportLang::Hcl` | `.hcl`, `.tf` | Missing from get_extensions() |

These fall through to the `_ => vec![]` catch-all, meaning they'll parse patterns correctly but won't find files when searching directories. Fix these alongside the Dart addition.

---

## References

- [ast-grep removal commit](https://github.com/ast-grep/ast-grep/commit/cd25a628f07bba546b9b7f7333079de481995def)
- [ast-grep issue #172 — original Dart feature request](https://github.com/ast-grep/ast-grep/issues/172)
- [ast-grep issue #1404 — tree-sitter version incompatibility](https://github.com/ast-grep/ast-grep/issues/1404)
- [ast-grep custom language docs](https://ast-grep.github.io/advanced/custom-language.html)
- [ast-grep add-lang guide](https://ast-grep.github.io/contributing/add-lang.html)
- [nielsenko/tree-sitter-dart GitHub](https://github.com/nielsenko/tree-sitter-dart)
- [tree-sitter-dart on crates.io](https://crates.io/crates/tree-sitter-dart)
- [kvnxiao/ast-grep-tree-sitter-dart — community NAPI wrapper](https://github.com/kvnxiao/ast-grep-tree-sitter-dart)
