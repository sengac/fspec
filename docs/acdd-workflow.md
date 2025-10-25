# ACDD Workflow: Understanding the Process

## What is ACDD?

**Acceptance Criteria Driven Development (ACDD)** is a discipline that builds on Specification by Example and BDD by enforcing a specific order:

```
Discovery → Specifications → Tests → Implementation → Validation → Done
```

**The key insight:** **ORDER MATTERS.** Writing code before tests, or tests before specs, leads to:
- ❌ Implementations that don't match requirements
- ❌ Tests that don't test the right things
- ❌ Specifications that document what was built, not what should be built

**ACDD enforces the right order** so you build what you need, not what you think you need.

---

## The ACDD Cycle

### 1. Discovery

**Question everything. Assume nothing.**

Before writing any specs, code, or tests, understand the requirements through Example Mapping:

- **Ask questions** (red cards) - Surface uncertainties
- **Capture rules** (blue cards) - Document business logic
- **Gather examples** (green cards) - Concrete scenarios

**Goal:** Shared understanding between you and AI about what needs to be built.

**Outcome:** Clarity on requirements, edge cases, and constraints.

**Time box:** ~25 minutes per story (stop when red cards are answered).

**See:** [Example Mapping Guide](./example-mapping.md)

### 2. Specifications

**Write acceptance criteria in validated Gherkin.**

Transform your example map into structured scenarios:

- Rules → Background context or scenario preconditions
- Examples → Given-When-Then scenarios
- User story → Background section

**Goal:** Machine-readable acceptance criteria that define "done".

**Outcome:** Feature file with validated Gherkin syntax.

**Tools:** `fspec generate-scenarios` or manual creation with `fspec create-feature`

**Validation:** `fspec validate` ensures syntax correctness.

### 3. Testing

**Write tests that map to scenarios. Tests must fail first (red phase).**

For each scenario in the feature file, write corresponding tests:

```typescript
/**
 * Feature: spec/features/user-login.feature
 */

describe('Scenario: Login with valid credentials', () => {
  it('should authenticate user and redirect to dashboard', async () => {
    // Given I am on the login page
    // When I enter valid credentials
    // Then I should be logged in and see dashboard

    // Test implementation...
  });
});
```

**Goal:** Prove that acceptance criteria are testable.

**Outcome:** Failing tests (red phase) that will pass when implementation is complete.

**Critical:** Tests MUST fail. If they pass, you have implementation code already.

**Link coverage:** `fspec link-coverage` to track scenario-to-test mappings.

### 4. Implementation

**Write minimal code to make tests pass (green phase).**

Implement ONLY what's needed to satisfy the acceptance criteria:

- Start with the simplest solution
- Make tests pass (green phase)
- Refactor while keeping tests green

**Goal:** Minimal viable implementation.

**Outcome:** Passing tests, working feature.

**Anti-pattern:** Over-engineering, implementing features not in specs.

**Link coverage:** `fspec link-coverage` to track test-to-implementation mappings.

### 5. Validation

**Run ALL tests and quality checks.**

Before marking work done, verify:

```bash
npm test           # All tests (not just new ones)
npm run typecheck  # Type safety
npm run lint       # Code quality
fspec validate     # Gherkin syntax
fspec check        # Complete validation
fspec review <id>  # ACDD compliance and quality review
```

**Goal:** Ensure nothing broke, quality standards met.

**Outcome:** Green tests, clean code, valid specs.

**Why ALL tests?** Catch regressions introduced by new code.

### 6. Done

**Work is complete when all acceptance criteria are satisfied.**

Criteria for "done":
- ✅ All scenarios have test coverage
- ✅ All tests pass
- ✅ Code implements all acceptance criteria (no more, no less)
- ✅ Quality checks pass
- ✅ Documentation updated (coverage files, tags, etc.)

**Final step:** `fspec update-work-unit-status <id> done`

---

## Why Order Matters

### Code Before Tests = Guessing

**What happens:**
1. Write code based on vague requirements
2. Write tests that pass for existing code
3. Tests don't actually test requirements

**Result:** False confidence. Tests pass but feature is wrong.

**Example:**
```typescript
// Code written first (WRONG)
function login(email: string, password: string) {
  return true; // Oops, always succeeds
}

// Test written second (WRONG)
it('should log in user', () => {
  expect(login('test@example.com', 'password')).toBe(true); // Passes!
});
```

**Problem:** Test passes but login is broken. Real requirements ignored.

### Tests Before Specs = Guessing

**What happens:**
1. Write tests based on assumptions
2. Implement to pass tests
3. Discover actual requirements don't match

**Result:** Wasted work. Built the wrong thing efficiently.

**Example:**
```typescript
// Test written first (WRONG)
it('should allow 4-character passwords', () => {
  expect(validatePassword('abcd')).toBe(true); // Assumption!
});

// Later discover: Password must be 8+ characters
// Now you have to rewrite tests AND code
```

**Problem:** Assumptions codified before understanding requirements.

### ACDD Order = Certainty

**What happens:**
1. **Discovery:** "Password must be 8+ chars, 1 uppercase, 1 number"
2. **Spec:** Gherkin scenario captures exact requirement
3. **Test:** Verify password validation logic (fails first)
4. **Code:** Implement validation to pass test
5. **Validation:** All tests pass, requirement satisfied

**Result:** Built exactly what was needed, proven by tests.

---

## Temporal Ordering Enforcement

**fspec prevents retroactive state walking** to ensure ACDD discipline.

### What is Temporal Ordering?

Work must progress forward through Kanban states in the correct sequence. You cannot:
- ❌ Jump from specifying to implementing (skipping testing)
- ❌ Move from backlog to implementing (skipping specifying and testing)
- ❌ Retroactively change state timestamps (dishonest workflow)

### How It Works

**Test file detection:**
- When moving from `testing` to `implementing`, fspec checks for test file
- Test file must have header comment linking to feature file
- If no test file found, transition blocked

**State sequence validation:**
- Each state has valid "next states"
- Invalid transitions are rejected
- Example: `implementing` can only go to `validating` or backward

**Exception:** Tasks (non-user-facing work) can skip test file validation.

### Why This Matters

**Without enforcement:**
- AI agents skip steps to "get things done faster"
- Code written before tests (no TDD)
- Specs written after code (documentation, not specification)
- Quality suffers

**With enforcement:**
- Honest workflow progression
- Red-Green-Refactor discipline maintained
- ACDD temporal ordering preserved
- Quality built-in

---

## Moving Backward (Intentional)

**You CAN move backward** when you discover mistakes or gaps:

### When to Move Backward

✅ **Move backward when:**
- Tests revealed incomplete specs → `testing` to `specifying`
- Need to add/fix tests → `implementing` to `testing`
- Discovered missing scenarios → any state to `specifying`
- Implementation approach was wrong → `validating` to `implementing`

**Example:**
```
You: "These tests revealed we're missing the OAuth scenario"

AI: "You're right. Let me move AUTH-001 back to specifying..."
    fspec update-work-unit-status AUTH-001 specifying

    Now let's add the OAuth scenario to the feature file...
    [adds scenario]

    Moving to testing to write tests for OAuth...
    fspec update-work-unit-status AUTH-001 testing
```

### When NOT to Create New Work Units

❌ **Don't create new work units for:**
- Fixing mistakes in current work
- Refining existing specs/tests/code
- Correcting errors in the same feature

✅ **Only create new work units for:**
- Genuinely new features (out of scope)
- Bugs in already-completed work (marked `done`)
- Technical debt to track separately

---

## Kanban States Explained

### backlog

**Definition:** Work identified but not started.

**What to do:** Prioritize, estimate (if feature file exists), plan dependencies.

**Next states:** `specifying` (start work), `blocked` (if blocked)

### specifying

**Definition:** Defining acceptance criteria through Example Mapping and Gherkin.

**What to do:**
1. Example Mapping (rules, examples, questions)
2. Generate or write feature file
3. Validate Gherkin syntax

**Next states:** `testing` (specs complete), `blocked`, `backlog` (cancel)

**Checkpoint:** Auto-created before leaving (if changes exist)

### testing

**Definition:** Writing tests that map to scenarios.

**What to do:**
1. Write test file with feature reference header
2. Map tests to scenarios
3. Ensure tests FAIL (red phase)
4. Link coverage: scenario → test

**Next states:** `implementing` (tests written), `specifying` (need more specs), `blocked`

**Validation:** Test file must exist to move to `implementing` (except for tasks)

**Checkpoint:** Auto-created before leaving

### implementing

**Definition:** Writing code to make tests pass.

**What to do:**
1. Write minimal code to pass tests
2. Achieve green phase
3. Refactor while keeping tests green
4. Link coverage: test → implementation

**Next states:** `validating` (code complete), `testing` (need more tests), `blocked`

**Checkpoint:** Auto-created before leaving

### validating

**Definition:** Running all quality checks.

**What to do:**
1. Run ALL tests (not just new ones)
2. Run linting, type checking
3. Validate Gherkin syntax
4. Check coverage

**Next states:** `done` (all checks pass), `implementing` (fixes needed), `blocked`

**Checkpoint:** Auto-created before leaving

### done

**Definition:** All acceptance criteria satisfied, quality verified.

**What to do:**
1. Update feature tags (`@wip` → `@done`)
2. Clean up virtual hooks
3. Clean up old checkpoints
4. Celebrate!

**Next states:** None (terminal state)

**Note:** Bugs found later become new work units

### blocked

**Definition:** Cannot proceed due to external dependency or blocker.

**What to do:**
1. Document blocker reason
2. Add `@blocked` tag to feature file
3. Work on other items
4. Resolve blocker when possible

**Next states:** Return to previous state when unblocked

---

## Prefill Detection

**fspec prevents estimating before specs are complete.**

### The Problem

AI agents might:
1. Create work unit
2. Immediately estimate story points
3. Write specs later

**Result:** Estimates based on guesses, not actual acceptance criteria.

### The Solution

**Prefill detection** scans generated feature files for placeholder text:

```gherkin
Background: User Story
  As a [role]
  I want to [action]
  So that [benefit]
```

**Placeholders detected:**
- `[role]`, `[action]`, `[benefit]`
- `[placeholder text]`
- `[QUESTION: ...]`

**Enforcement:**
```bash
fspec update-work-unit-estimate AUTH-001 5

Error: Cannot estimate AUTH-001 until feature file is complete.
Feature file contains prefill placeholders.

Use `fspec set-user-story` to set role/action/benefit before generating scenarios.
```

### Solution

**Set user story FIRST:**
```bash
fspec set-user-story AUTH-001 \
  --role "user" \
  --action "log in securely" \
  --benefit "I can access my account"

fspec generate-scenarios AUTH-001
# Feature file has no placeholders

fspec update-work-unit-estimate AUTH-001 5
✓ Estimate set to 5
```

---

## Story Point Estimation

**Estimate AFTER specifications, based on complexity.**

### Fibonacci Scale

Use Fibonacci sequence to reflect increasing uncertainty:

- **1 point** - Trivial (< 30 min)
- **2 points** - Simple (30 min - 1 hour)
- **3 points** - Moderate (1-2 hours)
- **5 points** - Complex (2-4 hours)
- **8 points** - Very Complex (4-8 hours)
- **13 points** - Large (8+ hours, acceptable upper limit)
- **21+ points** - Epic (TOO LARGE, break down)

### Estimation Timing

✅ **Estimate AFTER:**
- Example Mapping complete
- Feature file generated
- No prefill placeholders

❌ **Don't estimate:**
- Before understanding requirements
- Based on title alone
- Before feature file exists

### Large Estimate Warning

If estimate > 13 points, fspec warns:

```
⚠️  LARGE ESTIMATE WARNING: 21 points is too large.

Industry best practice: Break down into smaller work units (1-13 each).

Step-by-step:
1. Review feature file for natural boundaries
2. Create child work units for each group
3. Link dependencies
4. Estimate each child (1-13 points)
```

**Why break down:**
- Reduces risk
- Enables incremental delivery
- Improves estimation accuracy
- Makes progress visible

---

## Benefits of ACDD

### For Developers

- ✅ **Clarity** - Know exactly what to build
- ✅ **Confidence** - Tests prove correctness
- ✅ **Safety** - Can't skip critical steps
- ✅ **Quality** - Built-in, not bolted-on

### For AI Agents

- ✅ **Structure** - Clear workflow to follow
- ✅ **Validation** - Continuous correctness checks
- ✅ **Traceability** - Coverage links specs-tests-code
- ✅ **Recovery** - Checkpoints enable experimentation

### For Projects

- ✅ **Living documentation** - Specs stay synchronized
- ✅ **Test coverage** - Every scenario has tests
- ✅ **Quality metrics** - Estimation accuracy, velocity tracking
- ✅ **Audit trail** - Full history of decisions and changes

---

## Common Pitfalls

### Pitfall 1: Skipping Discovery

**Symptom:** Jumping straight to feature file without Example Mapping.

**Result:** Missing edge cases, incomplete understanding.

**Fix:** Always do Example Mapping. Ask questions. Gather examples.

### Pitfall 2: Tests Pass Immediately

**Symptom:** Tests pass on first run (green without red).

**Result:** Tests don't actually test anything useful.

**Fix:** Ensure tests fail first. Delete implementation code and verify.

### Pitfall 3: Over-Implementation

**Symptom:** Implementing features not in specs.

**Result:** Wasted work, increased complexity, untested code.

**Fix:** Implement ONLY what specs require. Resist "wouldn't it be nice if..."

### Pitfall 4: Skipping Validation

**Symptom:** Not running all tests before marking done.

**Result:** Regressions, broken features, quality issues.

**Fix:** Run ALL tests. Check coverage. Validate syntax.

### Pitfall 5: Retroactive State Walking

**Symptom:** Changing work unit state without doing the work.

**Result:** Dishonest workflow, false progress.

**Fix:** Let temporal ordering enforcement prevent this.

---

## ACDD vs Traditional Development

| Aspect | Traditional | ACDD |
|--------|-------------|------|
| Requirements | Often vague or missing | Concrete scenarios in Gherkin |
| Tests | Written after (if at all) | Written before implementation |
| Documentation | Drifts from reality | Stays synchronized (validated) |
| AI Guidance | None (chaos) | Enforced workflow |
| Traceability | Manual or missing | Automatic (coverage files) |
| Quality | Bolted-on later | Built-in from start |

---

## Getting Started with ACDD

1. **Read:** [Getting Started Guide](./getting-started.md)
2. **Practice:** Create your first feature using ACDD
3. **Understand:** [Example Mapping](./example-mapping.md)
4. **Track:** [Work Units](./work-units.md)
5. **Experiment:** [Git Checkpoints](./checkpoints.md)
6. **Enforce:** [Virtual Hooks](./virtual-hooks.md)

---

**ACDD isn't about being slow. It's about being certain.**

**Build the right thing. Build it right. Prove it works.**
