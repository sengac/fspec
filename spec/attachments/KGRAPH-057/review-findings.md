# Epic Review: KGRAPH-057 — Add Dart language support to GraphSearch AST index and dead code detection

**Date:** 2026-03-26
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1

## Summary
- 🔴 Critical: 0 issues
- 🟡 Warnings: 4 issues across 1 work unit (all fixed)
- 🟢 Observations: 5

## Work Unit Results

### KGRAPH-057: Add Dart language support to GraphSearch AST index and dead code detection — PASS

#### 🟡 Warnings Found (all fixed)

1. **`pub use ast_grep_language::LanguageExt;` was unnecessary re-export**
   - File: `codelet/napi/src/graph/ast_pipeline/ast_dart_extractor.rs:16`
   - The `LanguageExt` trait was re-exported publicly but never consumed by any other module. All other extractors use private `use`.
   
2. **Example map example [7] (dead code) lacked a dedicated Gherkin scenario**
   - The example map included "ast_dead_code correctly identifies uncalled Dart functions and files without importers" but there was no corresponding scenario in the feature file.
   
3. **`source.find(&*sig_text)` in `extract_calls()` was fragile**
   - When locating the function body for call extraction, the code searched the entire source for the first occurrence of the signature text. If a file had two methods with identical signatures, the second method's body lookup could match the wrong position.
   
4. **Background step had a grammar typo**
   - Feature file line 41: "So that get code navigation..." was missing "I".

#### 🟢 Observations
1. `find_braced_block()` is local to the Dart extractor — could be shared in future
2. `ast_dart_extractor.rs` is 609 lines — consistent with other extractors
3. Strong adherence to established extractor patterns
4. pubspec_dep_extractor.rs uses justified simple YAML parsing
5. Test file header correctly references the feature file

## Fix Results

### KGRAPH-057: Add Dart language support to GraphSearch AST index and dead code detection
- 🟡 Issue 1: `pub use` → ✅ Fixed: Changed to `use ast_grep_language::LanguageExt;`
- 🟡 Issue 2: Missing dead code scenario → ✅ Fixed: Added "Identify uncalled Dart functions and unimported files as dead code" scenario + test + coverage link
- 🟡 Issue 3: Fragile `source.find()` → ✅ Fixed: Replaced with `node.range().end` byte offset from AST node
- 🟡 Issue 4: Grammar typo → ✅ Fixed: "So that I get code navigation..."

## Final Verification
- All tests pass: ✅ (9/9 dart tests pass, including new dead code test)
- Build succeeds: ✅ (`cargo build` clean)
- Coverage complete: ✅ (9/9 scenarios, 100% coverage)
- Feature files valid: ✅ (`fspec validate` — all 702 features valid)
- Clippy: ✅ (no warnings for dart/pubspec files)
