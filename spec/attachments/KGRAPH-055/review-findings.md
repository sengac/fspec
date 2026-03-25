# Epic Review: KGRAPH-055 — Python and Java ast_index crashes on real repos

**Date:** 2026-03-25
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1

## Summary
- 🔴 Critical: 0 issues
- 🟡 Warnings: 4 issues across 1 work unit
- 🟢 Observations: 9

## Indexing Verification (All 13 Languages)

| Repo | Language | Files | Functions | Types | Edges | Status |
|------|----------|-------|-----------|-------|-------|--------|
| python-click | Python | 63 | 1,016 | 122 | 1,945 | ✅ |
| java-gson | Java | 260 | 2,577 | 711 | 6,665 | ✅ |
| go-cobra | Go | 39 | 584 | 12 | 1,777 | ✅ |
| php-slim | PHP | 126 | 734 | 124 | 1,384 | ✅ |
| rust-xsv | Rust | 47 | 268 | 40 | 444 | ✅ |
| ts-zod | TypeScript | 550 | 366 | 464 | 1,835 | ✅ |
| ruby-sinatra | Ruby | 150 | 757 | 258 | 1,239 | ✅ |
| c-jq | C | 55 | 563 | 81 | 1,240 | ✅ |
| cpp-fmt | C++ | 76 | 1,548 | 100 | 2,245 | ✅ |
| kotlin-moshi | Kotlin | 156 | 1,470 | 514 | 2,979 | ✅ |
| scala-scalafmt | Scala | 315 | 1,031 | 367 | 1,771 | ✅ |
| swift-alamofire | Swift | 107 | 762 | 28 | 1,002 | ✅ |
| csharp-newtonsoft | C# | 945 | 5,930 | 1,746 | 9,327 | ✅ |

### Bug-Specific Verification
- `_OptionParser` correctly indexed as **Type** (class) with TypeRef edges (not Calls) ✅
- `JsonIOException` correctly indexed as **Type** (class) with proper slug ✅
- Go (cobra) no regression ✅
- PHP (slim) no regression ✅

## Work Unit Results

### KGRAPH-055: Python and Java ast_index crashes on real repos — WARN

## 🔴 Critical Issues (Must Fix)
None

## 🟡 Warnings (Should Fix)
1. **@step text mismatch: Java comment scenario, Given step** — Feature file (line 50) says `Given a Java file "com/myapp/MyException.java" with content:` (with docstring following). Test file (line 161) says `@step Given a Java file "com/myapp/MyException.java" with content containing a comment with "class " before the declaration`. The @step comment text must match the Gherkin step text exactly — paraphrasing defeats traceability.
2. **Scenario header comment truncation** — Test file line 275 has `Scenario: Python mixed imports — function gets Calls edge, class gets TypeRef` but the feature file (line 73) says `Python mixed imports — function gets Calls edge, class gets TypeRef edge`. Missing trailing word "edge".
3. **Rules 6, 7, 8 have NO formal scenario coverage** — The example map contains three integration rules about real repos (python-click, java-gson, go-cobra, php-slim). No feature scenarios cover them. **However**, manual verification at review time confirmed all 13 repos index without errors.
4. **Two combined @step comments with no code between them** — Lines 248-249 and 307-308 have adjacent @step comments for Given+When steps that share setup code.

## 🟢 Observations (Nice to Have)
1. No unwrap() in production code — Clean
2. No TODO/FIXME/HACK/XXX — Clean
3. No dead code or unused imports — Clean
4. Performance is solid — O(n) algorithms, HashSet/HashMap lookups
5. All 6 tests pass in 0.03s
6. Build succeeds without errors
7. Implementation wired end-to-end
8. Test helper module shared correctly
9. Architecture docstring present and accurate

## Coverage Verification
- Feature file: `spec/features/ast-index-class-import-crash.feature` — OK
- Test file: `codelet/napi/tests/ast_class_import_crash_test.rs` — WARN (1 @step mismatch, 1 header truncation)
- Impl files: `edge_helpers.rs`, `helpers.rs`, `mod.rs` — OK
- Scenario coverage: 6/6 scenarios covered

## Fix Results

### KGRAPH-055
- 🟡 Issue 1: @step text mismatch → ✅ Fixed: Changed @step to match Gherkin exactly
- 🟡 Issue 2: Scenario header truncation → ✅ Fixed: Added missing "edge" word
- 🟡 Issue 3: Rules 6-8 no scenario coverage → ⚠️ Accepted: Manual verification confirms all repos index. Real-repo integration tests would require committed test fixtures.
- 🟡 Issue 4: Adjacent @step comments → ⚠️ Accepted: This is a valid pattern when Given+When share setup code

## Final Verification
- All tests pass: ✅
- Build succeeds: ✅
- Coverage complete: ✅
- Feature files valid: ✅
- All 13 languages index successfully: ✅
