# @step Workflow Analysis - Pain Points and Solutions

## Current State

### How @step Comments Work

1. **Feature file** contains scenarios with Given/When/Then steps
2. **Test file** has test cases that map to scenarios
3. **@step comments** must be placed in test file, one for each Gherkin step
4. **Validation** happens when running `fspec link-coverage`
5. **Blocking** prevents progression to implementing if @step comments missing

### Validation Error Message (ALREADY SHOWS MISSING STEPS)

```
Missing step comments:
  ✗ Given I am on the login page
    Add to test file: // @step Given I am on the login page
  ✗ When I enter valid credentials
    Add to test file: // @step When I enter valid credentials
```

**This is good!** The error shows exactly what's missing with exact text to add.

---

## Pain Points from JOURNAL.md

### 1. **Requirement Hidden Until Too Late**

**Problem:** AI only learns about @step requirement when `fspec link-coverage` fails

**Evidence from JOURNAL.md:**
- BUG-064: "Only learned about mandatory @step comments when blocked trying to move testing → implementing"
- BUG-064: "System-reminder when entering testing state mentions them, but buried in long message"
- BUG-065: "Would have saved time if this was more prominent upfront"

**Root Cause:**
- System-reminder for `testing` state is 42 lines long
- @step requirement mentioned in middle (lines 94-113)
- Not visually prominent enough
- AI doesn't realize this is CRITICAL until blocked

### 2. **Unclear WHEN to Add @step Comments**

**Problem:** AI doesn't understand @step comments must be added WHILE writing tests, not after

**Evidence from JOURNAL.md:**
- BUG-064: "Tried to run fspec link-coverage → Error: coverage file doesn't exist"
- Implies AI wrote tests first, THEN tried to add @step comments
- Should be: write test code WITH @step comments simultaneously

**Root Cause:**
- Instructions say "Write tests" then separately "Link coverage"
- Not clear that @step comments are PART OF writing tests
- Appears as a separate validation step, not integral to test writing

### 3. **Wrong Test File (CRITICAL ISSUE)**

**Problem:** AI adds @step comments to WRONG test file, breaking validation

**Evidence from user:**
- "THERE'S MANY CASES WHERE IT DOESN'T DO THAT AND PUTS IT IN OTHER TESTS, WHICH BREAKS THE VALIDATION"

**Root Cause:**
- No clear guidance on ONE scenario → ONE test mapping
- AI might have multiple test files open
- No reminder to put @step in the SAME test being written for THAT scenario
- Validation checks specific test file path, so wrong file = validation failure

---

## Proposed Solutions

### Solution 1: Make @step Requirement PROMINENT in Testing State Reminder

**Change system-reminder.ts line 76-118** to put @step requirement at TOP:

```
Work unit ${workUnitId} is now in TESTING status.

⚠️⚠️⚠️ CRITICAL REQUIREMENT - READ THIS FIRST ⚠️⚠️⚠️

EVERY Gherkin step MUST have an @step comment in your test file.

Structure:
  - ONE scenario = ONE test
  - Inside that test: ONE @step comment for EACH Given/When/Then/And step
  - Place @step comment RIGHT BEFORE the code that executes that step
  - Use EXACT text from feature file

Example:
  Feature file has:
    Scenario: Login with valid credentials
      Given I am on the login page
      When I enter valid credentials
      Then I should see the dashboard

  Test file needs:
    # @step Given I am on the login page
    page = render_login_page()

    # @step When I enter valid credentials
    submit_credentials()

    # @step Then I should see the dashboard
    assert dashboard_visible()

WITHOUT @step comments, you CANNOT progress to implementing!
Validation will BLOCK you with error showing missing steps.

---

Now write FAILING tests with @step comments:
  - Tests must fail (red phase)
  - Add header comment: # Feature: spec/features/[name].feature
  - Run tests and verify they fail
```

### Solution 2: Add Reminder About ONE Scenario = ONE Test

Add to testing state reminder:

```
MAPPING RULES:
  - ONE scenario → ONE test (not multiple tests)
  - Put ALL @step comments for a scenario in THAT scenario's test
  - Do NOT spread @step comments across multiple test files
  - Each test file can have multiple scenarios (multiple tests)
```

### Solution 3: Validation Error Shows Which Test File

When validation fails, show:

```
STEP VALIDATION FAILED for scenario "Login with valid credentials"

Checking test file: src/__tests__/auth.test.ts

Missing step comments:
  ✗ Given I am on the login page
    Add to THIS test file: # @step Given I am on the login page

Make sure you're adding @step comments to the CORRECT test file!
The test file you're linking is: src/__tests__/auth.test.ts
```

---

## Summary

**What's already good:**
- ✅ Validation error shows missing steps with exact text
- ✅ Language-agnostic @step parsing works
- ✅ Blocking prevents bad progression

**What needs fixing:**
1. **Prominence** - Make @step requirement FIRST thing shown in testing state
2. **Timing** - Clarify @step comments added DURING test writing, not after
3. **Mapping** - Emphasize ONE scenario → ONE test → ALL @step in THAT test
4. **Validation** - Show which test file is being checked when validation fails

**Implementation:**
- Refactor `system-reminder.ts` testing state message (lines 76-118)
- Refactor `formatValidationError()` to show test file path prominently
- Add "ONE scenario = ONE test" rule to reminder
