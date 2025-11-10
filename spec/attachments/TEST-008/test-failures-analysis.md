# Test Failures Analysis - fspec v0.7.2

**Total Failing Tests: 15**
**Test Suites with Failures: 3**

---

## 1. TUI Work Unit Details Formatting Tests (5 FAILURES)

**File:** `src/tui/__tests__/work-unit-details-formatting.test.tsx`
**Feature:** Improve work unit details panel formatting

### Failing Tests:
1. ❌ should display ID+Title on line 1, description on line 2, empty lines 3-4, metadata on line 5, NO header, NO padding
2. ❌ should display ID+Title on line 1, empty lines 2-4, metadata on line 5, NO header, NO padding  
3. ❌ should display ID+Title on line 1, first 2 desc lines on lines 2-3 with ... truncation, metadata on line 4
4. ❌ should display ID+Title on line 1, both desc lines on lines 2-3 WITHOUT ... truncation, metadata on line 4
5. ❌ should display "No work unit selected" centered on line 1, empty lines 2-4, NO header

### Analysis:
**Scenario Validity:** ❓ NEEDS INVESTIGATION
- These tests validate TUI rendering layout and formatting
- Need to check if implementation changed or tests expectations are outdated
- All 5 tests in this suite are failing, suggesting systematic issue

**Possible Causes:**
- TUI component rendering logic changed
- Test expectations don't match current implementation
- React Testing Library or Ink rendering changes

**Action Required:**
- Read test file and TUI component implementation
- Determine if tests are testing outdated behavior or if implementation regressed
- Either fix implementation OR update test expectations

---

## 2. AST Research Tool Tests (8 FAILURES)

**File:** `src/__tests__/ast-research-tool.test.ts`
**Feature:** AST code analysis using tree-sitter

### Failing Tests:
1. ❌ should find and return all async functions with metadata
2. ❌ should analyze and return functions with more than 5 parameters
3. ❌ should find similar patterns in TypeScript, Python, and Go files
4. ❌ should perform error-tolerant parsing of broken code
5. ❌ should find all classes implementing UserRepository interface
6. ❌ should find functions with cyclomatic complexity greater than 10
7. ❌ should extract function signatures with JSDoc comments in markdown format
8. ❌ should find and report unused import statements

### Error Messages:
```
RESEARCH TOOL ERROR
Tool: ast
Error: At least one of --query or --file is required
```

### Analysis:
**Scenario Validity:** ⚠️ TESTS ARE OUTDATED
- Tests were written for OLD stub implementation
- AST tool was recently connected to actual tree-sitter parser (RES-016)
- Tests expect stub behavior, not real parsing behavior
- Error shows tests aren't passing required arguments correctly

**Root Cause:**
- Tests written before actual AST parser was connected
- Test expectations based on stub JSON response
- CLI invocation in tests doesn't match new implementation

**Action Required:** ✅ TESTS NEED REWRITE
- Tests are validating OLD stub behavior
- Need to rewrite tests for ACTUAL tree-sitter parsing
- Update test expectations to match real AST output
- Fix CLI invocations to pass --query or --file flags

**Recommendation:** DELETE outdated tests, write new integration tests

---

## 3. Research Integration Tests (2 FAILURES)

**File:** `src/commands/__tests__/integrate-research-guidance.test.ts`
**Feature:** Integrate research guidance into AI agent assistance

### Failing Tests:
1. ❌ should include RESEARCH section in bootstrap output
2. ❌ should create CLAUDE.md with research documentation when using claude agent

### Analysis:
**Scenario Validity:** ❓ NEEDS INVESTIGATION
- Tests validate research tool integration with bootstrap/setup
- Need to check if bootstrap command changed or feature removed

**Possible Causes:**
- Bootstrap command implementation changed
- CLAUDE.md generation logic modified
- Test expectations don't match current behavior

**Action Required:**
- Read test file and bootstrap command implementation
- Determine if feature was removed or implementation changed
- Either fix implementation OR remove tests if feature deprecated

---

## Summary & Recommendations

### Immediate Actions:
1. **AST Research Tool Tests (HIGH PRIORITY)** - ✅ DELETE/REWRITE
   - Tests are definitively outdated (testing stub, not real parser)
   - Remove all 8 failing tests
   - Write new integration tests for actual tree-sitter behavior
   
2. **TUI Formatting Tests (MEDIUM PRIORITY)** - ❓ INVESTIGATE
   - Read implementation to determine if regressed or tests outdated
   - Fix whichever is wrong (tests or implementation)

3. **Research Integration Tests (LOW PRIORITY)** - ❓ INVESTIGATE  
   - Check if feature still exists
   - Update tests or remove if deprecated

### Exit Criteria:
- ✅ ALL tests must pass (0 failures)
- ✅ Outdated tests removed or rewritten
- ✅ Implementation bugs fixed if found
- ✅ Test coverage maintained or improved

