# Work Units: Project Management with ACDD

## Overview

**Work units** are discrete pieces of work tracked through a Kanban workflow. Unlike flat TODO lists, work units have:

- **Type** - Story, Bug, or Task
- **Status** - Kanban state (backlog → specifying → testing → implementing → validating → done)
- **Estimate** - Story points (Fibonacci scale)
- **Dependencies** - Relationships between work units
- **Epic** - Grouping into larger initiatives
- **Example Mapping** - Rules, examples, questions
- **Coverage** - Links to specs, tests, implementation

**Storage:** `spec/work-units.json` (single source of truth)

---

## Work Unit Types

### Story

**Definition:** User-facing feature that delivers value.

**When to use:**
- New functionality visible to users
- Features that change how users interact with the system
- Capabilities that deliver business value

**Examples:**
- User login
- Password reset
- Dashboard widgets
- API rate limiting

**Requirements:**
- Full ACDD workflow (Discovery → Spec → Test → Implement → Validate)
- Feature file with validated Gherkin
- Test coverage for all scenarios
- Story point estimation

**Create:**
```bash
fspec create-work-unit AUTH "User Login" --type story
```

### Bug

**Definition:** Broken functionality that needs fixing.

**When to use:**
- Feature used to work, now doesn't
- Behavior doesn't match specifications
- Regression from recent changes

**Examples:**
- Session timeout not working
- Password validation allows weak passwords
- Dashboard loads slowly

**Requirements:**
- Full ACDD workflow
- Feature file describing correct behavior
- Tests that reproduce the bug (must fail first)
- Story point estimation (for effort tracking)

**Create:**
```bash
fspec create-work-unit BUG "Session timeout not working" --type bug
```

**Note:** Bugs link to existing feature files or create new ones if specs missing.

### Task

**Definition:** Non-user-facing work (refactoring, infrastructure, documentation).

**When to use:**
- Technical debt
- Refactoring
- Infrastructure changes
- Documentation updates
- Configuration changes

**Examples:**
- Refactor authentication middleware
- Update API documentation
- Migrate to new database
- Add TypeScript strict mode

**Requirements:**
- Simplified workflow (tests optional)
- No feature file required (unless documenting behavior)
- Story point estimation optional (use judgment)

**Create:**
```bash
fspec create-work-unit TASK "Refactor auth middleware" --type task
```

**Note:** Tasks are exempt from test file temporal validation since tests are optional.

---

## Kanban Workflow

### The 7 States

```
backlog → specifying → testing → implementing → validating → done
                                            ↓
                                        blocked
```

**State descriptions:**

1. **backlog** - Work identified but not started
2. **specifying** - Defining acceptance criteria (Example Mapping + Gherkin)
3. **testing** - Writing tests that map to scenarios
4. **implementing** - Writing code to pass tests
5. **validating** - Running all quality checks
6. **done** - All acceptance criteria satisfied
7. **blocked** - Cannot proceed (external dependency)

**See:** [ACDD Workflow](./acdd-workflow.md) for detailed state descriptions.

### Moving Through States

```bash
fspec update-work-unit-status AUTH-001 specifying
fspec update-work-unit-status AUTH-001 testing
fspec update-work-unit-status AUTH-001 implementing
fspec update-work-unit-status AUTH-001 validating
fspec update-work-unit-status AUTH-001 done
```

**Enforcement:**
- Cannot skip states
- Temporal ordering prevents retroactive changes
- Test file required to move from testing to implementing (except tasks)
- Automatic checkpoints created before transitions (if changes exist)

### Moving Backward

**You CAN move backward when needed:**

```bash
# Discovered incomplete specs while testing
fspec update-work-unit-status AUTH-001 specifying

# Need to fix tests
fspec update-work-unit-status AUTH-001 testing

# Implementation approach was wrong
fspec update-work-unit-status AUTH-001 implementing
```

**When to move backward:**
- Tests revealed incomplete specs
- Need to add/fix test cases
- Discovered missing scenarios
- Quality checks failed

**Don't create new work units for:**
- Fixing mistakes in current work
- Refining existing specs/tests/code
- Correcting errors in same feature

---

## Estimation

### Fibonacci Scale

Use Fibonacci sequence for story points:

- **1 point** - Trivial (< 30 minutes)
- **2 points** - Simple (30 min - 1 hour)
- **3 points** - Moderate (1-2 hours)
- **5 points** - Complex (2-4 hours)
- **8 points** - Very Complex (4-8 hours)
- **13 points** - Large (8+ hours, acceptable upper limit)
- **21+ points** - Epic (TOO LARGE, must break down)

### When to Estimate

✅ **Estimate AFTER:**
- Example Mapping complete
- Feature file generated
- No prefill placeholders in specs

❌ **Don't estimate:**
- Before understanding requirements
- Based on title alone
- Before feature file exists

**Set estimate:**
```bash
fspec update-work-unit-estimate AUTH-001 5
```

### Estimation Validation

**fspec enforces:**
- Cannot estimate stories/bugs without completed feature file
- Feature file must not have prefill placeholders
- Warnings for estimates > 13 points

**Error example:**
```
fspec update-work-unit-estimate AUTH-001 5

Error: Cannot estimate AUTH-001 until feature file is complete.
Feature file contains prefill placeholders: [role], [action], [benefit]

Use `fspec set-user-story` before generating scenarios.
```

**Solution:**
```bash
fspec set-user-story AUTH-001 \
  --role "user" \
  --action "log in" \
  --benefit "access account"

fspec generate-scenarios AUTH-001

fspec update-work-unit-estimate AUTH-001 5
✓ Estimate set to 5
```

### Large Estimate Warning

**If estimate > 13 points:**

```
⚠️  LARGE ESTIMATE WARNING: AUTH-001 estimate is 21 points.

21 points is too large for a single story. Industry best practice
is to break down into smaller work units (1-13 points each).

WHY BREAK DOWN:
  - Reduces risk and complexity
  - Enables incremental delivery
  - Improves estimation accuracy
  - Makes progress more visible

STEP-BY-STEP WORKFLOW:
1. REVIEW FEATURE FILE for natural boundaries
2. CREATE CHILD WORK UNITS for each logical group
3. LINK DEPENDENCIES between children and parent
4. ESTIMATE EACH CHILD (1-13 points)
5. HANDLE PARENT (delete or convert to epic)
```

**Note:** 13 points is acceptable (upper limit). 21+ must be split.

---

## Dependencies

### Adding Dependencies

```bash
fspec add-dependency AUTH-002 --depends-on AUTH-001
```

**What this means:**
- AUTH-002 cannot start until AUTH-001 is done
- Visualized in dependency graphs
- Helps with prioritization and planning

### Removing Dependencies

```bash
fspec remove-dependency AUTH-002 --depends-on AUTH-001
```

### Dependency Patterns

**Linear dependency:**
```
AUTH-001 → AUTH-002 → AUTH-003
```

**Parallel with merge:**
```
AUTH-001 ─┬→ AUTH-003
AUTH-002 ─┘
```

**Feature prerequisite:**
```
AUTH-001 (Login) → DASH-001 (Dashboard requiring auth)
```

---

## Epics

### What are Epics?

**Epics** group related work units into larger business initiatives.

**Example:**
- Epic: "User Management"
  - AUTH-001: User Login
  - AUTH-002: User Registration
  - AUTH-003: Password Reset
  - AUTH-004: Profile Management

### Creating Epics

```bash
fspec create-epic "User Management" USER-MGMT "All user-related features"
```

### Associating Work Units

```bash
fspec create-work-unit AUTH "User Login" --epic=USER-MGMT
```

### Epic Progress

**Track epic completion:**
```bash
fspec list-work-units --epic=USER-MGMT
```

**Output:**
```
Work units for epic USER-MGMT:

AUTH-001: User Login (done) - 5 pts
AUTH-002: User Registration (implementing) - 8 pts
AUTH-003: Password Reset (backlog) - 3 pts
AUTH-004: Profile Management (backlog) - 5 pts

Progress: 1/4 work units done (25%)
Story points: 5/21 completed (24%)
```

---

## Viewing Work Units

### Kanban Board

```bash
fspec board
```

**Output:**
```
┌─────────┬────────────┬─────────┬───────────────┬───────────┬──────┐
│ Backlog │ Specifying │ Testing │ Implementing  │Validating │ Done │
├─────────┼────────────┼─────────┼───────────────┼───────────┼──────┤
│ AUTH-003│            │         │ AUTH-001      │           │AUTH-004│
│ (3 pts) │            │         │ User Login    │           │(5 pts) │
│         │            │         │ (5 pts)       │           │        │
│ AUTH-002│            │         │               │           │        │
│ (8 pts) │            │         │               │           │        │
└─────────┴────────────┴─────────┴───────────────┴───────────┴──────┘

Total: 21 story points (5 done, 16 remaining)
```

### List Work Units

```bash
fspec list-work-units
fspec list-work-units --status=implementing
fspec list-work-units --type=story
fspec list-work-units --epic=USER-MGMT
```

### Show Work Unit Details

```bash
fspec show-work-unit AUTH-001
```

**Output:**
```
Work Unit: AUTH-001
Title: User Login
Type: story
Status: implementing
Estimate: 5 points
Epic: USER-MGMT

User Story:
  As a user
  I want to log in securely
  So that I can access account features

Business Rules:
  1. Password must be at least 8 characters with 1 uppercase and 1 number
  2. Account locks after 3 failed login attempts

Examples:
  1. User enters valid credentials and is authenticated
  2. User fails login 3 times and account is locked

Feature File: spec/features/user-login.feature
Coverage: 80% (4/5 scenarios tested)

Dependencies:
  Blocks: AUTH-002 (User Registration)

Virtual Hooks:
  post-implementing: npm test (blocking)
```

### Advanced Querying & Analysis

Search and compare work units, scenarios, and implementations for patterns and consistency:

```bash
# Search scenarios across all features
fspec search-scenarios --query="validation"
fspec search-scenarios --query="user.*login" --regex --json

# Find function usage across work units
fspec search-implementation --function=validateInput
fspec search-implementation --function=queryWorkUnits --show-work-units

# Compare implementation approaches for tagged work units
fspec compare-implementations --tag=@cli --show-coverage
fspec compare-implementations --tag=@authentication --json

# Analyze testing patterns across similar work units
fspec show-test-patterns --tag=@high --include-coverage
fspec show-test-patterns --tag=@cli --json

# Query work units with enhanced filtering
fspec query-work-units --status=done --tag=@cli --format=table
fspec query-work-units --type=story --prefix=AUTH --format=json
```

**Use cases:**
- Find similar scenarios to reuse patterns
- Identify inconsistencies in testing approaches
- Compare implementation strategies across features
- Ensure architectural consistency
- Discover function usage across the codebase

---

## Best Practices

### For Work Unit Creation

✅ **DO:**
- Specify type explicitly (story/bug/task)
- Use descriptive titles
- Add descriptions when needed
- Link to epics for organization
- Set dependencies for blocked work

❌ **DON'T:**
- Create work units for every tiny task
- Use vague titles ("Fix stuff")
- Skip type specification
- Create work units for in-progress refinements

### For Estimation

✅ **DO:**
- Estimate AFTER Example Mapping
- Use Fibonacci scale
- Base estimates on scenarios, not guesses
- Break down large estimates (>13 points)
- Track estimation accuracy over time

❌ **DON'T:**
- Estimate before understanding requirements
- Use hours instead of story points
- Skip estimation for stories/bugs
- Let estimates > 13 points exist

### For Workflow

✅ **DO:**
- Follow ACDD order strictly
- Move backward when discovering mistakes
- Use checkpoints for experimentation
- Add virtual hooks for quality gates
- Clean up when work is done

❌ **DON'T:**
- Skip workflow states
- Create new work units for same-feature fixes
- Ignore temporal ordering
- Leave work in limbo (blocked without reason)

---

## Metrics and Tracking

### Velocity Tracking

```bash
fspec query-metrics
```

**Shows:**
- Story points completed per time period
- Average velocity
- Estimation accuracy
- Completion trends

### Estimate Accuracy

```bash
fspec query-estimate-accuracy
```

**Shows:**
- Estimated vs actual story points
- Over/under-estimation patterns
- Improvement trends

### Summary Reports

```bash
fspec generate-summary-report
```

**Includes:**
- Work unit counts by status and type
- Story points completed vs remaining
- Coverage statistics
- Epic progress
- Dependency graphs

---

## Work Units vs TODO Lists

| Aspect | TODO Lists | Work Units |
|--------|------------|------------|
| State | Done/Not Done | 7 Kanban states |
| Types | None | Story/Bug/Task |
| Dependencies | Manual tracking | Built-in, visualized |
| Estimation | None | Story points (Fibonacci) |
| Coverage | None | Specs-Tests-Code traceability |
| Workflow | Freeform | Enforced ACDD |
| Queryable | No | Yes (by status, type, epic) |
| Persistent | No (in conversation) | Yes (JSON file) |

---

## Commands Reference

```bash
# Create
fspec create-work-unit <PREFIX> "<Title>" --type <story|bug|task>
fspec create-epic "<Name>" <PREFIX> "<Description>"

# Update
fspec update-work-unit-status <ID> <status>
fspec update-work-unit-estimate <ID> <points>

# Dependencies
fspec add-dependency <ID> --depends-on <OTHER-ID>
fspec remove-dependency <ID> --depends-on <OTHER-ID>

# View
fspec board
fspec list-work-units [--status=<status>] [--type=<type>] [--epic=<epic>]
fspec show-work-unit <ID>

# Delete
fspec delete-work-unit <ID>

# Metrics
fspec query-metrics
fspec query-estimate-accuracy
fspec generate-summary-report
```

---

## See Also

- [ACDD Workflow](./acdd-workflow.md) - Understanding the process
- [Example Mapping](./example-mapping.md) - Discovery techniques
- [Coverage Tracking](./coverage-tracking.md) - Traceability
- [Git Checkpoints](./checkpoints.md) - Safe experimentation
- [Virtual Hooks](./virtual-hooks.md) - Quality gates

---

**Work units aren't tasks. They're artifacts of disciplined development.**
