# Reverse ACDD: Document Existing Codebases

## Overview

**Reverse ACDD** reverse engineers existing codebases to create specifications, tests, and coverage mappings. Perfect for:

- Legacy codebases without specs or tests
- Projects transitioning to ACDD workflow
- Understanding inherited code through BDD lens
- Documenting what you already built

**Key feature:** Interactive strategy planner guides you through the best approach based on gap analysis.

---

## Quick Start

```bash
# Interactive mode (recommended)
fspec reverse

# Direct strategy selection
fspec reverse --strategy=A  # Spec gap (generate from tests)
fspec reverse --strategy=B  # Test gap (generate from code)
fspec reverse --strategy=C  # Coverage gap (link tests to code)
fspec reverse --strategy=D  # Full reverse ACDD
```

**Interactive mode:**
- Analyzes your codebase
- Identifies gaps (specs, tests, coverage)
- Suggests best strategy
- Guides through decision tree
- Saves session state for resumption

---

## The Four Strategies

### Strategy A: Spec Gap

**When to use:**
- ✅ You have tests
- ❌ You don't have specs

**What it does:**
1. Analyzes test files
2. Infers scenarios from test structure
3. Generates feature files
4. Links tests to scenarios

**Example:**
```typescript
// Existing test file
describe('POST /login', () => {
  it('should authenticate valid user', async () => { ... });
  it('should reject invalid credentials', async () => { ... });
});

// Strategy A generates:
Feature: User Login
  Scenario: Authenticate valid user
    Given I have valid credentials
    When I submit login form
    Then I should be authenticated

  Scenario: Reject invalid credentials
    Given I have invalid credentials
    When I submit login form
    Then I should see an error message
```

**Coverage:**
```
Scenario → Test File (✅ linked)
Scenario → Implementation (❌ not linked)
```

### Strategy B: Test Gap

**When to use:**
- ✅ You have specs (or can write them)
- ❌ You don't have tests

**What it does:**
1. Uses existing feature files (or generates from code)
2. Creates skeleton test files
3. Structures tests to map to scenarios
4. Links tests to specs

**Example:**
```gherkin
# Existing spec (spec/features/user-login.feature)
Feature: User Login
  Scenario: Authenticate valid user
    Given I have valid credentials
    When I submit login form
    Then I should be authenticated

# Strategy B generates skeleton:
/**
 * Feature: spec/features/user-login.feature
 * NOTE: Skeleton test - implement the test logic
 */
describe('Scenario: Authenticate valid user', () => {
  it('should authenticate user', async () => {
    // TODO: Implement this test
    // Given I have valid credentials
    // When I submit login form
    // Then I should be authenticated
  });
});
```

**Coverage:**
```
Scenario → Test File (✅ linked, ❌ not implemented)
Scenario → Implementation (❌ not linked)
```

### Strategy C: Coverage Gap

**When to use:**
- ✅ You have specs
- ✅ You have tests
- ❌ They're not linked

**What it does:**
1. Uses existing feature files
2. Uses existing test files
3. Uses similarity matching to link scenarios to tests
4. Links tests to implementation code
5. Generates coverage files

**Example:**
```bash
# Existing: spec/features/user-login.feature (scenarios)
# Existing: src/__tests__/auth.test.ts (tests)
# Existing: src/auth/login.ts (implementation)

# Strategy C links them:
fspec reverse --strategy=C

Analyzing feature files... 5 found
Analyzing test files... 8 found
Analyzing implementation... 12 files

Similarity matching:
  ✓ "Authenticate valid user" → auth.test.ts:15-28 (95% match)
  ✓ "Reject invalid credentials" → auth.test.ts:30-42 (92% match)

Creating coverage mappings...
✓ user-login.feature.coverage created
```

**Coverage:**
```
Scenario → Test File (✅ linked)
Scenario → Implementation (✅ linked)
```

### Strategy D: Full Reverse ACDD

**When to use:**
- ❌ No specs
- ❌ No tests
- ✅ Just code

**What it does:**
1. Analyzes code structure (routes, APIs, UI components)
2. Infers user stories and scenarios
3. Creates feature files
4. Creates skeleton test files
5. Links tests to specs
6. Links implementation to tests

**Example:**
```typescript
// Existing code only
app.post('/login', async (req, res) => {
  const { email, password } = req.body;
  if (!email || !password) {
    return res.status(400).json({ error: 'Missing credentials' });
  }
  const user = await authenticateUser(email, password);
  if (!user) {
    return res.status(401).json({ error: 'Invalid credentials' });
  }
  res.json({ token: generateToken(user) });
});

// Strategy D generates:
// 1. spec/features/user-login.feature (inferred from route)
// 2. src/__tests__/auth-login.test.ts (skeleton)
// 3. user-login.feature.coverage (mappings)
```

**Coverage:**
```
Scenario → Test File (✅ linked, ❌ not implemented)
Scenario → Implementation (✅ linked)
```

---

## Interactive Strategy Planner

### How It Works

**Step 1: Analysis**

```
You: fspec reverse

AI: Analyzing your codebase for reverse ACDD...

    Scanning feature files... 0 found
    Scanning test files... 15 found
    Analyzing routes and APIs... 23 endpoints found

    GAP ANALYSIS:
    ❌ Spec gap: No feature files found
    ✅ Test coverage: 15 test files exist
    ⚠️  Coverage gap: Tests not linked to specs

    Based on this analysis, I recommend Strategy A (Spec Gap).
```

**Step 2: Strategy Suggestion**

```
AI: RECOMMENDED STRATEGY: A (Spec Gap)

    WHY:
    - You have test files (good foundation)
    - No feature files (need to generate from tests)
    - Tests can guide spec creation

    ALTERNATIVE STRATEGIES:
    B. Test Gap (if you write specs manually first)
    C. Coverage Gap (not applicable - no specs yet)
    D. Full Reverse ACDD (overkill since you have tests)

    Would you like to:
    1. Proceed with Strategy A (recommended)
    2. Choose a different strategy
    3. See detailed decision tree
```

**Step 3: Execution**

```
You: 1

AI: Proceeding with Strategy A (Spec Gap)...

    PHASE 1: Analyzing test structure
    ✓ Identified 15 test files
    ✓ Detected 47 test scenarios
    ✓ Grouped into 8 feature areas

    PHASE 2: Generating feature files
    ✓ Created spec/features/user-login.feature (6 scenarios)
    ✓ Created spec/features/password-reset.feature (4 scenarios)
    ✓ Created spec/features/user-registration.feature (5 scenarios)
    ...

    PHASE 3: Linking coverage
    ✓ Linked 47 scenarios to test files
    ✓ Generated 8 coverage files

    PHASE 4: Creating work units
    ✓ Created AUTH-001: User Login
    ✓ Created AUTH-002: Password Reset
    ✓ Created AUTH-003: User Registration
    ...

    REVERSE ACDD COMPLETE!

    Coverage: 100% (47/47 scenarios linked to tests)
    Test gap: 0 scenarios without tests
    Implementation gap: 47 scenarios (run Strategy C to link implementation)

    Next steps:
    1. Review generated feature files
    2. Run `fspec reverse --strategy=C` to link implementation
    3. Implement missing tests (if any skeleton tests exist)
```

### Session Management

**Sessions are saved automatically:**

```
AI: Reverse ACDD session started...
    Session ID: rev-acdd-2025-10-21-14-30-15
    Session file: .fspec-reverse-session.json

    ... (analysis and execution)

    Session saved. You can resume later with:
    fspec reverse --resume
```

**Resume later:**

```
You: fspec reverse --resume

AI: Resuming reverse ACDD session from 2025-10-21 14:30:15...

    Previous state:
    - Strategy: A (Spec Gap)
    - Phase completed: Analysis
    - Next phase: Generate feature files

    Continuing from where we left off...
```

---

## Decision Tree Guidance

**The interactive planner uses this decision tree:**

```
Do you have feature files?
  → YES: Continue to Q2
  → NO: Continue to Q3

Q2: Do you have tests?
  → YES: Strategy C (Coverage Gap - link specs to tests to code)
  → NO: Strategy B (Test Gap - generate skeleton tests from specs)

Q3: Do you have tests?
  → YES: Strategy A (Spec Gap - generate specs from tests)
  → NO: Strategy D (Full Reverse ACDD - analyze code, generate everything)
```

**You can view this tree:**

```bash
fspec reverse --help
# Shows decision tree in help output
```

---

## Gap Analysis

**fspec analyzes three types of gaps:**

### Spec Gap

**Definition:** Feature files missing.

**Detection:**
- Scans `spec/features/` directory
- Counts `.feature` files
- If 0, spec gap exists

**Resolution:** Strategy A or D

### Test Gap

**Definition:** Test files missing or incomplete.

**Detection:**
- Searches for test files (*.test.ts, *.spec.ts, etc.)
- Analyzes test structure
- Checks for scenario coverage

**Resolution:** Strategy B or D

### Coverage Gap

**Definition:** Specs and tests exist but aren't linked.

**Detection:**
- Checks for `.coverage` files
- Analyzes scenario-to-test mappings
- Identifies unmapped scenarios

**Resolution:** Strategy C

---

## Similarity Matching (Strategy C)

**How fspec links scenarios to tests:**

### Hybrid Similarity Algorithm

Uses 5 algorithms weighted by accuracy:

1. **Exact match** (100% weight) - Scenario name matches test description exactly
2. **Normalized match** (95% weight) - After lowercasing, removing punctuation
3. **Keyword overlap** (70% weight) - Significant word overlap
4. **Fuzzy match** (60% weight) - Levenshtein distance similarity
5. **Semantic similarity** (40% weight) - Meaning-based matching

**Threshold:** 60% minimum for automatic linking

**Example:**

```gherkin
Scenario: User logs in with valid credentials
```

**Test candidates:**
```typescript
it('should authenticate user with valid credentials', ...) // 95% match
it('logs in user successfully', ...)                        // 75% match
it('handles login request', ...)                            // 45% match (below threshold)
```

**Result:** Links to first test (95% match)

### Manual Review

**If match < 80%:**

```
AI: ⚠️  Low confidence match detected:

    Scenario: "User logs in with valid credentials"
    Test: "should authenticate user with valid credentials" (75% match)

    Should I link these? (y/n)

You: y

AI: ✓ Linked scenario to test
```

---

## Handling Ambiguous Code

**When encountering unclear business logic:**

```gherkin
# AMBIGUOUS: magic number 42 in discount logic - needs human clarification
Scenario: Apply special discount
  Given a customer has a discount code
  And the discount value is greater than 42  # Why 42? Ask human.
  When they complete checkout
  Then a special discount should be applied
```

**AI creates red card:**

```bash
fspec add-question DISC-001 "@human: Why is discount threshold 42? Business rule or magic number?"
```

**After clarification:**

```bash
fspec answer-question DISC-001 0 --answer "Business rule: Discounts > 42% trigger special processing"
```

**Spec updated:**

```gherkin
Scenario: Apply special discount
  Given a customer has a discount code with value greater than 42%
  When they complete checkout
  Then special discount processing should be triggered
```

---

## Coverage Tracking

**Link existing code to scenarios:**

```bash
# Strategy A or D creates skeleton tests
# You need to link implementation manually or use Strategy C

# Link implementation to test mapping
fspec link-coverage user-login \
  --scenario "Authenticate valid user" \
  --test-file src/__tests__/auth.test.ts \
  --impl-file src/auth/login.ts \
  --impl-lines 15-42

# Check coverage
fspec show-coverage user-login
```

**Result:**
```
Coverage for user-login:

✅ Authenticate valid user (FULLY COVERED)
   Test: src/__tests__/auth.test.ts:10-22
   Implementation: src/auth/login.ts:15-42

⚠️  Reject invalid credentials (PARTIAL COVERAGE)
   Test: src/__tests__/auth.test.ts:24-35
   Implementation: (not linked)

Coverage: 50% (1/2 scenarios fully covered)
```

---

## Epics and Work Units

**Reverse ACDD creates epics automatically:**

```
Analyzing codebase structure...

Found business domains:
  - Authentication (routes: /login, /logout, /register)
  - User Profile (routes: /profile, /settings)
  - Payment (routes: /checkout, /payment)

Creating epics:
  fspec create-epic "Authentication" AUTH "User authentication features"
  fspec create-epic "User Profile" PROF "User profile management"
  fspec create-epic "Payment Processing" PAY "Payment and checkout"

Creating work units:
  fspec create-work-unit AUTH "User Login" --epic=AUTH
  fspec create-work-unit AUTH "User Registration" --epic=AUTH
  fspec create-work-unit PROF "Edit Profile" --epic=PROF
  ...
```

---

## Completion Criteria

**Reverse ACDD is complete when:**

- ✅ All user-facing interactions have feature files
- ✅ All epics have at least one work unit
- ✅ All scenarios linked to tests (if Strategy A/C/D)
- ✅ All tests linked to implementation (if Strategy C/D)
- ✅ Ambiguous scenarios documented with questions
- ✅ Coverage files exist for all features

**Check completion:**

```bash
fspec show-coverage  # All features
fspec query-metrics  # Coverage statistics
fspec board          # Work unit status
```

---

## Transitioning to Forward ACDD

**After reverse ACDD, use forward ACDD for new features:**

1. **Create work unit**
   ```bash
   fspec create-work-unit AUTH "Password Reset" --type story
   ```

2. **Example Mapping** (Discovery)
   ```bash
   fspec add-rule AUTH-004 "Reset token valid for 1 hour"
   fspec add-example AUTH-004 "User requests reset, receives email, clicks link, resets password"
   ```

3. **Generate spec**
   ```bash
   fspec generate-scenarios AUTH-004
   ```

4. **Write tests** (must fail first)

5. **Implement** (make tests pass)

6. **Validate** (all tests, quality checks)

**See:** [ACDD Workflow](./acdd-workflow.md)

---

## Commands Reference

```bash
# Interactive mode (recommended)
fspec reverse

# Strategy selection
fspec reverse --strategy=A  # Spec gap
fspec reverse --strategy=B  # Test gap
fspec reverse --strategy=C  # Coverage gap
fspec reverse --strategy=D  # Full reverse ACDD

# Session management
fspec reverse --resume       # Resume previous session
fspec reverse --clear-session  # Clear saved session

# Help
fspec reverse --help         # Decision tree and guidance
```

---

## Best Practices

✅ **DO:**
- Start with interactive mode (analyzes and suggests)
- Review generated specs for accuracy
- Answer questions about ambiguous code
- Link implementation for full coverage (Strategy C)
- Use forward ACDD for new features after reverse ACDD

❌ **DON'T:**
- Blindly accept generated specs (review first)
- Ignore ambiguous code warnings
- Skip coverage linking
- Use reverse ACDD for new features (use forward ACDD)

---

## Troubleshooting

**"No strategy suggested"**
- Codebase may be too minimal
- Try manual strategy selection
- Check that code exists in expected locations

**"Low confidence matches"**
- Review and manually confirm/reject
- Adjust test descriptions to match scenarios
- Use explicit linking if needed

**"Ambiguous scenarios"**
- Use Example Mapping to clarify with human
- Answer questions in work unit
- Update scenarios based on answers

---

## See Also

- [Getting Started](./getting-started.md) - Learn forward ACDD workflow
- [ACDD Workflow](./acdd-workflow.md) - Understanding the process
- [Coverage Tracking](./coverage-tracking.md) - Traceability
- [Work Units](./work-units.md) - Project management

---

**Reverse ACDD: Document what you built. Then build with discipline.**
