# Review: KGRAPH-056 — Add Dart language support to AstGrep and AstGrepRefactor tools

**Date:** 2026-03-26
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1

## Summary
- 🔴 Critical: 4 issues across 1 work unit (all previously fixed)
- 🟡 Warnings: 4 issues across 1 work unit (all previously fixed)
- 🟢 Observations: 2 (pre-existing, out of scope)
- 🟡 New issues found in second review: 2 (fixed)

---

## Work Unit Results

### KGRAPH-056: Add Dart language support — PASS

## 🔴 Critical Issues (Must Fix) — Previously Fixed

1. **Feature file Scenario 2 @step mismatch** — Feature said `'void $NAME($$$PARAMS) { $$$BODY }'` but Dart splits function_signature and function_body as sibling AST nodes, making full-function patterns fail with "Multiple AST nodes". Tests correctly used signature-only pattern `'void $NAME($$$PARAMS)'` but the feature file and @step comments were out of sync.
   → ✅ Fixed (prior review)

2. **Feature file Scenario 3 test/spec divergence** — Feature described replacing a function `'oldName'→'newName'` but test actually renamed a class `OldService→NewService` (correct for Dart's AST structure). @step comments matched the old feature text, not the actual test behavior.
   → ✅ Fixed (prior review)

3. **Feature file Scenario 4 test/spec divergence** — Feature described `print→debugPrint` replacement but test actually batch-replaced class body fields. Same root cause as #2 (Dart AST splitting makes function-level patterns unreliable).
   → ✅ Fixed (prior review)

4. **Bad Gherkin in Scenario 7** — Multiple When/Then pairs in a single scenario violates Gherkin structure. Had `When...Then... When...Then... When...Then...` for Solidity, Nix, Hcl in one scenario.
   → ✅ Fixed (prior review): Split into 3 separate scenarios.

## 🟡 Warnings (Should Fix) — Previously Fixed

5. **Missing 'dart' in AstGrepArgs doc comment** — `language` field schema listed "Supported: rust, typescript, tsx, javascript, python, go, java, c, cpp, ruby, kotlin, swift, scala, php, bash, html, css, json, yaml, lua, elixir, haskell" — missing dart, solidity, nix, hcl, csharp.
   → ✅ Fixed (prior review)

6. **Missing 'dart' in AstGrepRefactorArgs doc comment** — Same issue as #5.
   → ✅ Fixed (prior review)

7. **Duplicated `parse_language()` in astgrep_refactor.rs** — Re-implemented identically to `astgrep.rs`. Both are in the same crate so should share.
   → ✅ Fixed (prior review): Delegates via `AstGrepTool::parse_language()`.

8. **NAPI error message missing dart** — `codelet/napi/src/astgrep.rs` line 207 listed incomplete supported language list.
   → ✅ Fixed (prior review)

## 🟡 New Issues Found (Second Review)

9. **Feature file stale example mapping comments** — The `BUSINESS RULES` comment block had 3 stale entries:
   - Rule #2 referenced "expando_char 'µ'" but implementation uses no expando_char ($ is valid in Dart identifiers)
   - Rule #7 referenced expando_char-based meta-variable parsing, but it works natively
   - Example #4 referenced full-function pattern `'void $NAME($$$PARAMS) { $$$BODY }'` but should be signature-only
   - Rule #8 (Dart grammar split behavior) was missing entirely
   → ✅ Fixed: Updated all 3 stale entries and added rule #8.

10. **NAPI DRY violation: duplicated `parse_language()` and `get_extensions()`** — `codelet/napi/src/astgrep.rs` had its own copies of `parse_language()` (lines 142-151) and `get_extensions()` (lines 154-187) that duplicated `AstGrepTool::parse_language()` and `AstGrepTool::get_extensions()`. The NAPI `get_extensions` also had a weaker `_ => vec![]` catch-all that would silently ignore new SupportLang variants.
   → ✅ Fixed: Both functions now delegate to `AstGrepTool::parse_language()` and `AstGrepTool::get_extensions()`. Removed unused `SupportLang` and `DartLang` imports from NAPI module.

## 🟢 Observations (Nice to Have)

11. **File sizes exceed 300-line guideline** — `astgrep.rs` (493 lines), `astgrep_refactor.rs` (1172 lines), `codelet/napi/src/astgrep.rs` (995 lines). Pre-existing issue beyond this work unit's scope; would require a separate refactoring work unit.

12. **NAPI layer remaining code duplication** — `codelet/napi/src/astgrep.rs` still duplicates `search_file()`, all transform functions (`apply_transforms`, `apply_transform`, `convert_case`, `split_into_words`, `apply_replacement_template`, `validate_transforms`, `topological_sort`, cycle detection), and `MatchData` struct from `codelet/tools/src/`. This cross-crate DRY concern would require extracting a shared helper module or re-exporting from codelet-tools. Out of scope for KGRAPH-056.

## Coverage Verification
- Feature file: `spec/features/dart-astgrep-support.feature` — ✅ OK (9 scenarios, valid Gherkin, comments aligned with work unit)
- Test file: `codelet/tools/tests/astgrep_dart_test.rs` — ✅ OK (9 tests, all @steps match feature file exactly)
- Impl files: `dart_lang.rs`, `astgrep.rs`, `astgrep_refactor.rs` — ✅ OK
- NAPI layer: `codelet/napi/src/astgrep.rs` — ✅ OK (delegates to codelet-tools for parse_language/get_extensions)
- Scenario coverage: 9/9 scenarios covered (100%)

## Final Verification
- All tests pass: ✅ (9 dart tests + all astgrep tests green)
- Build succeeds: ✅ (`cargo check -p codelet-napi --features noop`)
- Coverage complete: ✅ (100% — 9/9 scenarios)
- Feature files valid: ✅ (`fspec validate`)
- Feature file comments aligned with work unit example map: ✅

## Files Reviewed
- `spec/features/dart-astgrep-support.feature`
- `codelet/tools/tests/astgrep_dart_test.rs`
- `codelet/tools/src/dart_lang.rs`
- `codelet/tools/src/astgrep.rs`
- `codelet/tools/src/astgrep_refactor.rs`
- `codelet/tools/Cargo.toml`
- `codelet/napi/src/astgrep.rs`
- `codelet/napi/Cargo.toml`
- `src/research-tools/ast.ts`
