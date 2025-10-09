# Project Management System - Brainstorm & Design

**DRAFT - BRAINSTORMING DOCUMENT**

This document explores how fspec could evolve into a command-line project management system specifically designed for AI agents practicing Acceptance Criteria Driven Development (ACDD).

---

## Table of Contents

1. [Why We Need This](#1-why-we-need-this)
2. [What Problem Does This Solve](#2-what-problem-does-this-solve)
3. [Core Concepts](#3-core-concepts)
4. [Work Unit Model](#4-work-unit-model)
5. [Hierarchical Relationships](#5-hierarchical-relationships)
6. [Workflow States (Kanban)](#6-workflow-states-kanban)
7. [Estimation vs Actuals](#7-estimation-vs-actuals)
8. [Dependencies](#8-dependencies)
9. [Example Mapping Integration](#9-example-mapping-integration)
10. [Data Model](#10-data-model)
11. [Command Structure](#11-command-structure)
12. [Integration with CAGE](#12-integration-with-cage)
13. [Query & Reporting](#13-query--reporting)
14. [Example Workflows](#14-example-workflows)
15. [Open Questions](#15-open-questions)

---

## 1. Why We Need This

### The Current Gap

AI agents working with ACDD need to manage work like human development teams do, but existing project management tools are designed for humans:

- **Traditional tools** (like commercial ticket systems) require web UIs and manual updates
- **They don't integrate** with ACDD workflow (spec → test → code)
- **They're not AI-native** - AI agents need CLI/API access
- **They lack validation hooks** - can't enforce that tests are written before code
- **They're separate systems** - disconnected from the actual specifications

### What AI Agents Need

AI agents orchestrated by systems like CAGE need:

1. **CLI-native project management** - All operations via command line
2. **ACDD workflow enforcement** - States that mirror spec → test → code progression
3. **Tight integration** with Gherkin specifications (the actual work artifacts)
4. **Programmatic access** - JSON output for querying and processing
5. **Learning capability** - Track estimated vs actual effort to improve over time
6. **Cross-cutting organization** - Work units that span features/scenarios via tags
7. **Dependency management** - Track what blocks what automatically
8. **Example mapping support** - Structured discovery phase before coding

### Design Principles

1. **Tag-based work units** - Work is organized by tags, not files
2. **Progressive enrichment** - Work units start minimal, gain detail as they progress
3. **Kanban flow** - Continuous flow through states, no sprint planning
4. **JSON-backed** - Single source of truth in version-controlled JSON
5. **AI-centric metrics** - Track tokens/iterations, not hours/days
6. **CAGE-orchestrated** - fspec stores data, CAGE manages workflow
7. **CLI-first** - Every operation available via command line

---

## 2. What Problem Does This Solve

### For AI Agents

**Problem**: AI agents don't have a structured way to:
- Track what work exists and its current state
- Know what to work on next (priority + dependencies)
- Learn from past work (estimated vs actual complexity)
- Handle discovery/refinement phase (example mapping)
- Coordinate across multiple features (cross-cutting concerns)

**Solution**: fspec becomes a CLI-native project management system where:
- Work units are tagged entities that AI can query and update
- States enforce ACDD workflow (backlog → specifying → testing → implementing → validating → done)
- Dependencies prevent working on blocked items
- Metrics help AI learn to estimate better
- Example mapping artifacts captured during discovery

### For CAGE (AI Orchestration System)

**Problem**: CAGE needs to:
- Decide what work to pull next
- Transition work through ACDD states
- Validate that each stage is complete before progressing
- Track metrics (tokens used, iterations needed)
- Handle blockers and dependencies

**Solution**: fspec provides:
- JSON queries to see available work by state
- Commands to transition work and record metrics
- Dependency graph to avoid blocked work
- State history for learning patterns

### For Human Developers/Product Owners

**Problem**: Humans need to:
- Add work to the backlog
- Answer questions during discovery (example mapping)
- Understand what AI is working on
- Review metrics and progress

**Solution**: fspec provides:
- Simple CLI to create work units
- Questions field for example mapping blockers
- JSON exports for dashboards/visualization
- Audit trail of all state changes

---

## 3. Core Concepts

### Work Unit

A **work unit** is a discrete piece of work tracked by fspec. It is NOT tied to a specific file or scenario, but rather to a **tag** that can be applied across multiple features/scenarios.

**Key characteristics:**
- Identified by a unique ID (e.g., `AUTH-001`)
- Has a lifecycle through workflow states
- Can span multiple feature files and scenarios
- Tagged onto Gherkin features/scenarios with `@AUTH-001`
- Stored in `spec/work-units.json`

**Why tag-based?**
- Cross-cutting concerns: A security work unit might touch authentication, API, and dashboard features
- Flexibility: Can group related scenarios across different feature files
- Granularity: Can tag entire features or individual scenarios
- Traceability: Easy to see which specs belong to which work

### Prefix

A **prefix** is a short code that namespaces work unit IDs (e.g., `AUTH`, `DASH`, `API`, `SEC`).

**Characteristics:**
- 2-6 uppercase letters
- Represents a functional area or project
- Used for ID generation (e.g., `AUTH-001`, `AUTH-002`)
- Not hierarchical - just a namespace

**Examples:**
- `AUTH` - Authentication features
- `DASH` - Dashboard features
- `API` - API endpoints
- `SEC` - Security improvements
- `PERF` - Performance optimizations

### Epic

An **epic** is a high-level business initiative that can span multiple prefixes and work units.

**Characteristics:**
- Named with kebab-case (e.g., `epic-user-management`)
- Contains multiple work units from different prefixes
- Represents a business goal or theme
- Optional - not all work units need an epic

**Examples:**
- `epic-user-management` - All user-related features (spans AUTH, DASH, API)
- `epic-security-hardening` - Security improvements (spans AUTH, API, SEC)
- `epic-performance-Q1` - Q1 performance goals (spans API, DASH, PERF)

### Workflow State

The **state** of a work unit in the ACDD workflow. States are designed around how AI agents work, not human developers.

**States:**
1. `backlog` - Work exists but not started
2. `specifying` - Writing/refining Gherkin specs using example mapping
3. `testing` - Writing tests that map to scenarios
4. `implementing` - Writing code to make tests pass
5. `validating` - Running all validations (CAGE hooks, build, tests)
6. `done` - All validations pass, work complete
7. `blocked` - Stuck at any stage (dependency, question, external blocker)

### Parent/Child Relationship

Work units can have **parent/child** relationships for task decomposition.

**Example:**
```
AUTH-001 (parent: "Implement OAuth 2.0")
  ├─ AUTH-002 (child: "OAuth provider integration")
  ├─ AUTH-003 (child: "Token storage and refresh")
  └─ AUTH-004 (child: "Login UI components")
```

**Rules:**
- A work unit can have 0 or 1 parent
- A work unit can have 0 to many children
- Parent cannot be marked `done` until all children are `done`
- Children can be in different states

### Dependency Relationships

Work units can have several types of dependencies:

1. **blocks** - This work unit blocks other work from starting
2. **blockedBy** - Cannot start until these are done
3. **dependsOn** - Should start after (soft dependency, not blocking)
4. **relatesTo** - Cross-cutting relationship (no workflow impact)

**Example:**
```json
{
  "AUTH-001": {
    "title": "OAuth Integration",
    "blocks": ["DASH-001"],      // Dashboard can't start until this is done
    "blockedBy": ["API-001"],    // Can't start until API endpoint exists
    "dependsOn": ["SEC-001"],    // Should do security review first (soft)
    "relatesTo": ["PERF-003"]    // Related to performance work (no blocking)
  }
}
```

### Story Points

**Story points** represent estimated complexity/effort for AI work.

**For AI agents:**
- Story points predict **token cost** and **number of iterations** needed
- NOT hours or days (AI works in seconds/minutes)
- Uses Fibonacci scale: 1, 2, 3, 5, 8, 13

**Calibration examples:**
- **1 point**: Simple scenario, single feature file, straightforward implementation
- **2 points**: Multiple related scenarios, some complexity
- **3 points**: Cross-file changes, moderate test complexity
- **5 points**: Multiple features affected, integration testing needed
- **8 points**: Architectural change, many files, complex dependencies
- **13 points**: Major feature, should be broken down into smaller work units

### Actual Metrics

**Actual metrics** are recorded by CAGE after work completes:

- **actualTokens**: Total LLM tokens consumed (prompt + completion)
- **iterations**: Number of attempts (how many times AI had to retry)
- **failedValidations**: Number of times validation failed before success
- **stateHistory**: Timestamps of state transitions

**Purpose:**
- Learn patterns: "8-point tasks usually take 150k tokens"
- Improve estimation: "We consistently underestimate API work"
- Detect bottlenecks: "Most failures happen in validating state"
- Track velocity: "We complete ~20 points per week"

---

## 4. Work Unit Model

### Core Fields

Every work unit has these core fields:

```json
{
  "id": "AUTH-001",
  "title": "Implement OAuth 2.0 login flow",
  "description": "Add OAuth 2.0 authentication supporting Google and GitHub providers",
  "status": "implementing",
  "createdAt": "2025-01-15T10:00:00Z",
  "updatedAt": "2025-01-15T14:30:00Z"
}
```

**Field descriptions:**
- `id`: Unique identifier (prefix + auto-incrementing number)
- `title`: Short, human-readable title (required)
- `description`: Longer description (optional, added during specifying)
- `status`: Current workflow state (required, defaults to "backlog")
- `createdAt`: When work unit was created (auto-generated)
- `updatedAt`: Last modification timestamp (auto-updated)

### Organization Fields

Fields for organizing work:

```json
{
  "epic": "epic-user-management",
  "parent": null,
  "children": ["AUTH-002", "AUTH-003"],
  "tags": ["@security", "@phase1"]
}
```

**Field descriptions:**
- `epic`: Epic this belongs to (optional)
- `parent`: Parent work unit ID (optional, null if top-level)
- `children`: Array of child work unit IDs (optional, empty if no children)
- `tags`: Additional custom tags beyond the work unit ID tag (optional)

### Dependency Fields

Fields for managing dependencies (all optional):

```json
{
  "blocks": ["DASH-001", "API-003"],
  "blockedBy": ["API-001"],
  "dependsOn": ["SEC-001", "SEC-002"],
  "relatesTo": ["PERF-003", "DOC-005"]
}
```

### Estimation & Metrics Fields

Fields for estimation and actual tracking:

```json
{
  "estimate": 5,
  "actualTokens": 125000,
  "iterations": 3,
  "failedValidations": 1
}
```

**Field descriptions:**
- `estimate`: Story points (optional, typically added during specifying)
- `actualTokens`: Actual tokens consumed (optional, recorded by CAGE when done)
- `iterations`: Number of attempts needed (optional, recorded by CAGE)
- `failedValidations`: Failed validation attempts (optional, recorded by CAGE)

### Example Mapping Fields

Fields for example mapping during discovery (specifying state):

```json
{
  "rules": [
    "Users must authenticate before accessing protected resources",
    "OAuth tokens expire after 1 hour",
    "Refresh tokens valid for 30 days"
  ],
  "examples": [
    "User logs in with Google account",
    "User logs in with expired token",
    "User token auto-refreshes before expiry"
  ],
  "questions": [
    "@bob: Should we support GitHub Enterprise?",
    "@security-team: Do we need PKCE flow?",
    "What happens when refresh token expires?"
  ],
  "assumptions": [
    "Users have valid OAuth accounts",
    "Network is available for OAuth callbacks"
  ]
}
```

**Field descriptions:**
- `rules`: Business rules discovered during example mapping (optional)
- `examples`: Example scenarios (will become Gherkin scenarios) (optional)
- `questions`: Unanswered questions blocking progress (optional)
- `assumptions`: Assumptions made during specification (optional)

### State History

Track state transitions for metrics and auditing:

```json
{
  "stateHistory": [
    { "state": "backlog", "timestamp": "2025-01-15T10:00:00Z" },
    { "state": "specifying", "timestamp": "2025-01-15T12:00:00Z" },
    { "state": "blocked", "timestamp": "2025-01-15T13:00:00Z", "reason": "Waiting for security team answer" },
    { "state": "specifying", "timestamp": "2025-01-15T13:30:00Z" },
    { "state": "testing", "timestamp": "2025-01-15T14:00:00Z" },
    { "state": "implementing", "timestamp": "2025-01-15T14:30:00Z" }
  ]
}
```

### Complete Example

A fully-populated work unit:

```json
{
  "AUTH-001": {
    "id": "AUTH-001",
    "title": "Implement OAuth 2.0 login flow",
    "description": "Add OAuth 2.0 authentication supporting Google and GitHub providers with token refresh and proper error handling",
    "status": "implementing",
    "createdAt": "2025-01-15T10:00:00Z",
    "updatedAt": "2025-01-15T14:30:00Z",

    "epic": "epic-user-management",
    "parent": null,
    "children": ["AUTH-002", "AUTH-003", "AUTH-004"],
    "tags": ["@security", "@critical"],

    "blocks": ["DASH-001"],
    "blockedBy": [],
    "dependsOn": ["API-001"],
    "relatesTo": ["SEC-001", "PERF-003"],

    "estimate": 8,
    "actualTokens": null,
    "iterations": null,
    "failedValidations": null,

    "rules": [
      "Users must authenticate before accessing protected resources",
      "OAuth tokens expire after 1 hour",
      "Refresh tokens valid for 30 days"
    ],
    "examples": [
      "User logs in with Google account",
      "User logs in with expired token",
      "User token auto-refreshes before expiry"
    ],
    "questions": [],
    "assumptions": [
      "Users have valid OAuth accounts",
      "Network is available for OAuth callbacks"
    ],

    "stateHistory": [
      { "state": "backlog", "timestamp": "2025-01-15T10:00:00Z" },
      { "state": "specifying", "timestamp": "2025-01-15T12:00:00Z" },
      { "state": "testing", "timestamp": "2025-01-15T14:00:00Z" },
      { "state": "implementing", "timestamp": "2025-01-15T14:30:00Z" }
    ]
  }
}
```

---

## 5. Hierarchical Relationships

### Epic → Work Units

Epics are high-level business initiatives that contain multiple work units:

```
epic-user-management
├─ AUTH-001 (OAuth integration)
├─ AUTH-002 (Password reset)
├─ DASH-001 (User profile page)
├─ API-003 (User endpoints)
└─ SEC-001 (Rate limiting)
```

**Characteristics:**
- One epic can contain work units from multiple prefixes
- One work unit belongs to 0 or 1 epic
- Epics are stored separately in the data model
- Epics have metadata: name, description, goals

### Parent/Child Work Units

Work units can be decomposed into smaller work units:

```
AUTH-001 (Implement OAuth 2.0)
├─ AUTH-002 (OAuth provider integration)
│   ├─ AUTH-005 (Google provider)
│   └─ AUTH-006 (GitHub provider)
├─ AUTH-003 (Token storage and refresh)
└─ AUTH-004 (Login UI components)
```

**Rules:**
- Parent work units cannot be marked `done` until all children are `done`
- Children inherit epic from parent (unless explicitly overridden)
- Children can be in different states than parent
- Maximum nesting depth: 3 levels (keep it manageable)

**When to use parent/child:**
- Breaking down large work (8+ points) into smaller pieces
- Parallel work on different aspects (e.g., frontend + backend)
- Incremental delivery (parent tracks overall goal)

### Cross-Cutting Relationships (relatesTo)

Work units can be related without hierarchy:

```
AUTH-001 (OAuth integration)
  relatesTo: SEC-001 (Rate limiting)
  relatesTo: PERF-003 (Token caching)
  relatesTo: DOC-005 (Auth documentation)
```

**Use cases:**
- Security review needed for auth work
- Performance optimization touches auth
- Documentation updates needed
- Cross-functional concerns

---

## 6. Workflow States (Kanban)

### State Definitions

```
┌─────────────┐
│   backlog   │  Work exists, not started
└──────┬──────┘
       │
       ↓
┌─────────────┐
│ specifying  │  Example mapping, writing Gherkin scenarios
└──────┬──────┘
       │
       ↓
┌─────────────┐
│   testing   │  Writing tests that map to scenarios
└──────┬──────┘
       │
       ↓
┌─────────────┐
│implementing │  Writing code to make tests pass
└──────┬──────┘
       │
       ↓
┌─────────────┐
│ validating  │  CAGE hooks, build, all checks
└──────┬──────┘
       │
       ↓
┌─────────────┐
│    done     │  Complete, all validations passed
└─────────────┘

       ↓ (can happen from any state)

┌─────────────┐
│   blocked   │  Cannot progress (dependency, question, external)
└─────────────┘
```

### State: backlog

**Purpose:** Work exists but hasn't started.

**Entry criteria:**
- Work unit created with `fspec create-work-unit`
- Has a title

**Activities:**
- Prioritizing (reordering in backlog)
- Assigning to epic
- Initial grooming

**Exit criteria:**
- CAGE pulls work to start discovery
- Moves to `specifying`

**Data typically present:**
- `id`, `title`
- Maybe `epic`, `parent`, basic `description`

### State: specifying

**Purpose:** Discovery phase using example mapping. Define what to build.

**Entry criteria:**
- Work pulled from backlog
- Ready to define acceptance criteria

**Activities:**
- Example mapping: identify rules, examples, questions, assumptions
- Writing Gherkin scenarios in feature files
- Tagging scenarios with work unit ID (`@AUTH-001`)
- Estimating story points
- Identifying dependencies

**Exit criteria:**
- All questions answered (or acceptable to proceed)
- Gherkin scenarios written and tagged
- Story points estimated
- No blockers
- Moves to `testing`

**Data typically present:**
- `rules`, `examples`, `questions`, `assumptions`
- `estimate` (story points)
- `description` (refined)
- `blocks`, `blockedBy`, `dependsOn`, `relatesTo`

**Blocked scenarios:**
- Questions need human answers → move to `blocked`
- Missing dependency → move to `blocked`

### State: testing

**Purpose:** Write tests that map to Gherkin scenarios BEFORE writing code.

**Entry criteria:**
- Gherkin scenarios written and validated
- Estimate assigned
- Dependencies clear

**Activities:**
- Writing unit tests
- Writing integration tests
- Mapping tests to Gherkin scenarios
- Tests MUST fail (no code yet)

**Exit criteria:**
- All scenarios have corresponding tests
- Tests fail as expected (red phase)
- Test coverage adequate
- Moves to `implementing`

**Data typically present:**
- All fields from `specifying`
- Tests written and failing

**Blocked scenarios:**
- Test framework issues → `blocked`
- Unclear acceptance criteria → back to `specifying`

### State: implementing

**Purpose:** Write minimum code to make tests pass.

**Entry criteria:**
- Tests written and failing
- Clear what needs to be built

**Activities:**
- Writing implementation code
- Running tests iteratively
- Refactoring while keeping tests green
- Updating documentation

**Exit criteria:**
- All tests pass (green phase)
- Code formatted and linted
- Ready for validation
- Moves to `validating`

**Data typically present:**
- All previous fields
- Code written
- Tests passing

**Blocked scenarios:**
- Technical blocker (library issue, platform bug) → `blocked`
- Need architectural decision → `blocked`

### State: validating

**Purpose:** Run all CAGE hooks, build, comprehensive validation.

**Entry criteria:**
- Tests passing locally
- Code complete

**Activities:**
- Running CAGE validation hooks
- Running full test suite
- Running build process
- Checking Gherkin validation
- Checking tag validation
- Any other quality checks

**Exit criteria:**
- All validations pass
- No errors or warnings
- Moves to `done`

**Failure scenarios:**
- Validation fails → back to appropriate state:
  - Test failures → `implementing`
  - Spec issues → `specifying`
  - Build errors → `implementing`

**Data being recorded:**
- `actualTokens` (CAGE tracks and writes back)
- `iterations` (how many attempts)
- `failedValidations` (number of validation failures)

### State: done

**Purpose:** Work complete, all validations passed.

**Entry criteria:**
- All validations passed
- All acceptance criteria met
- If parent: all children also `done`

**Activities:**
- None (terminal state)
- May be referenced by other work units

**Exit criteria:**
- None (work is complete)

**Data present:**
- All fields populated
- `actualTokens`, `iterations`, `failedValidations` recorded
- Complete `stateHistory`

### State: blocked

**Purpose:** Work cannot progress due to external factor.

**Entry criteria:**
- Blocker encountered in any state
- Cannot proceed without resolution

**Activities:**
- Waiting for dependency to complete
- Waiting for human to answer question
- Waiting for external system/decision

**Exit criteria:**
- Blocker resolved
- Returns to previous state (or appropriate next state)

**Data present:**
- `blockedReason`: Description of blocker
- Previous state tracked in `stateHistory`
- Work on other items while blocked

### State Transitions

Valid transitions:

```
backlog → specifying
specifying → testing (if no blockers)
specifying → blocked (if questions/dependencies)
testing → implementing
testing → blocked (if test issues)
implementing → validating
implementing → blocked (if technical issues)
validating → done (if all pass)
validating → specifying (if spec issues)
validating → testing (if test issues)
validating → implementing (if code issues)
blocked → [previous state] (when unblocked)
```

Invalid transitions (enforced by fspec):
```
backlog → implementing (must specify and test first)
testing → done (must implement and validate)
any state → backlog (work doesn't go back to backlog)
```

---

## 7. Estimation vs Actuals

### Story Point Estimation

**When estimated:**
- During `specifying` state
- After example mapping clarifies scope
- Before moving to `testing`

**Fibonacci scale:**
```
1 point  - Single scenario, straightforward implementation
2 points - Few related scenarios, some complexity
3 points - Multiple scenarios, moderate complexity
5 points - Cross-feature work, integration needed
8 points - Significant feature, architectural considerations
13 points - Epic-level work, should be broken down
```

**Factors to consider:**
- Number of scenarios
- Number of files affected
- Integration complexity
- Test complexity
- Unknowns/risks

### Actual Metrics Tracked

**Token consumption:**
```json
{
  "actualTokens": 125000,
  "tokenBreakdown": {
    "specifying": 15000,
    "testing": 35000,
    "implementing": 60000,
    "validating": 15000
  }
}
```

**Iteration count:**
```json
{
  "iterations": 3,
  "iterationDetails": [
    { "attempt": 1, "state": "implementing", "result": "test failure" },
    { "attempt": 2, "state": "implementing", "result": "test failure" },
    { "attempt": 3, "state": "implementing", "result": "success" }
  ]
}
```

**Validation failures:**
```json
{
  "failedValidations": 2,
  "validationHistory": [
    { "timestamp": "...", "result": "failed", "reason": "Gherkin syntax error" },
    { "timestamp": "...", "result": "failed", "reason": "Test coverage below threshold" },
    { "timestamp": "...", "result": "passed" }
  ]
}
```

### Learning from Metrics

**Pattern detection:**
- "5-point API work consistently uses 80k-120k tokens"
- "Security features (SEC prefix) take 2x iterations vs estimates"
- "Most failures happen in validating state (suggests integration issues)"

**Velocity tracking:**
- "Completed 25 points this week (5 work units)"
- "Average cycle time: 2.5 hours from backlog to done"
- "Bottleneck: 40% of time spent in validating state"

**Estimation improvement:**
- "Our estimates are 30% low on average → adjust upward"
- "Cross-feature work (relatesTo > 2) needs +3 points buffer"
- "Work with questions.length > 0 takes 50% longer"

### Queries for Analysis

```bash
# Compare estimated vs actual for completed work
fspec query metrics --type=estimate-accuracy --output=json

# Returns:
{
  "1-point": { "avgTokens": 25000, "samples": 12 },
  "2-point": { "avgTokens": 45000, "samples": 8 },
  "3-point": { "avgTokens": 75000, "samples": 15 },
  "5-point": { "avgTokens": 110000, "samples": 6 },
  "8-point": { "avgTokens": 180000, "samples": 3 }
}

# Find bottlenecks
fspec query metrics --type=cycle-time --output=json

# Returns:
{
  "avgTimeInBacklog": "2.5 hours",
  "avgTimeInSpecifying": "1.2 hours",
  "avgTimeInTesting": "0.8 hours",
  "avgTimeInImplementing": "1.5 hours",
  "avgTimeInValidating": "3.2 hours",  // BOTTLENECK!
  "avgTimeBlocked": "4.5 hours"
}
```

---

## 8. Dependencies

### Dependency Types

**1. blocks**

This work unit blocks other work from starting.

```json
{
  "API-001": {
    "title": "Create user API endpoint",
    "blocks": ["AUTH-001", "DASH-001"]
  }
}
```

**Behavior:**
- AUTH-001 and DASH-001 cannot move from `backlog` to `specifying` until API-001 is `done`
- CAGE will not pull blocked work
- If work is in progress when blocker is added, it moves to `blocked` state

**Use when:**
- Foundation work that others depend on
- Breaking changes that need coordination
- Shared infrastructure

**2. blockedBy**

Cannot start this work unit until dependencies are done.

```json
{
  "AUTH-001": {
    "title": "Implement OAuth login",
    "blockedBy": ["API-001", "SEC-002"]
  }
}
```

**Behavior:**
- AUTH-001 stays in `backlog` or moves to `blocked` until API-001 and SEC-002 are `done`
- CAGE checks `blockedBy` before pulling work
- Auto-transitions to `blocked` if dependency added mid-flight

**Use when:**
- Need foundation work completed first
- Waiting for external dependency
- Requires another feature to exist

**3. dependsOn**

Soft dependency - should start after, but not strictly blocking.

```json
{
  "DASH-001": {
    "title": "User dashboard",
    "dependsOn": ["AUTH-001"]
  }
}
```

**Behavior:**
- CAGE prefers to start AUTH-001 before DASH-001, but doesn't enforce
- Work can proceed even if dependencies not done
- Used for prioritization hints

**Use when:**
- Logical ordering (but not required)
- Risk mitigation (do harder thing first)
- Learning dependency (need to understand before implementing)

**4. relatesTo**

Cross-cutting relationship with no workflow impact.

```json
{
  "AUTH-001": {
    "title": "OAuth integration",
    "relatesTo": ["SEC-001", "PERF-003", "DOC-005"]
  }
}
```

**Behavior:**
- No workflow impact
- Used for traceability and reporting
- Helps find related work

**Use when:**
- Cross-functional concerns (security, performance, docs)
- Impact analysis (what else might be affected?)
- Knowledge sharing (similar problems)

### Automatic Dependency Management

**Parent/child dependencies:**
- Parent automatically `blocks` all siblings' children
- Parent cannot be `done` until all children `done`

**Example:**
```
AUTH-001 (parent)
├─ AUTH-002 (child: Google provider)
├─ AUTH-003 (child: GitHub provider)
└─ AUTH-004 (child: UI components)
```

If AUTH-002 is not done, AUTH-001 cannot be marked `done`.

**Circular dependency detection:**
```bash
fspec validate-dependencies

# Detects cycles like:
AUTH-001 blockedBy API-001
API-001 blockedBy AUTH-001

# Returns error:
Circular dependency detected: AUTH-001 → API-001 → AUTH-001
```

### Dependency Queries

```bash
# What's blocking this work unit?
fspec query work-unit AUTH-001 --show-blockers

# Returns:
{
  "id": "AUTH-001",
  "status": "blocked",
  "blockedBy": ["API-001"],
  "blockerStatus": {
    "API-001": {
      "status": "implementing",
      "estimate": 5,
      "progress": "60%"
    }
  }
}

# What would this unblock?
fspec query work-unit API-001 --show-blocks

# Returns:
{
  "id": "API-001",
  "blocks": ["AUTH-001", "DASH-001", "DASH-002"],
  "blockingCount": 3,
  "totalPointsBlocked": 18
}

# Find all blocked work
fspec query work-units --status=blocked --output=json

# Dependency graph
fspec query dependencies --output=json
```

---

## 9. Example Mapping Integration

### What is Example Mapping?

**Example mapping** is a BDD technique for collaborative discovery:

- **Rules**: Business rules that govern the feature
- **Examples**: Concrete examples (become Gherkin scenarios)
- **Questions**: Unknowns that need answering
- **Assumptions**: Things we're assuming to be true

### Example Mapping in fspec Workflow

**During `specifying` state:**

1. **Start example mapping:**
```bash
fspec update-work-unit AUTH-001 --status=specifying
```

2. **Add rules:**
```bash
fspec add-rule AUTH-001 "Users must authenticate before accessing protected resources"
fspec add-rule AUTH-001 "OAuth tokens expire after 1 hour"
fspec add-rule AUTH-001 "Refresh tokens valid for 30 days"
```

3. **Add examples (will become scenarios):**
```bash
fspec add-example AUTH-001 "User logs in with Google account"
fspec add-example AUTH-001 "User logs in with expired token"
fspec add-example AUTH-001 "User token auto-refreshes before expiry"
```

4. **Add questions (blockers):**
```bash
fspec add-question AUTH-001 "@bob: Should we support GitHub Enterprise?"
fspec add-question AUTH-001 "@security-team: Do we need PKCE flow?"
```

5. **Add assumptions:**
```bash
fspec add-assumption AUTH-001 "Users have valid OAuth accounts"
fspec add-assumption AUTH-001 "Network is available for OAuth callbacks"
```

### Question Handling

**Questions block progress:**
- If `questions.length > 0`, work unit should stay in `specifying` or move to `blocked`
- Questions can mention people: `@bob: question text`
- CAGE can notify mentioned people
- Humans answer by removing/resolving questions

**Answering questions:**
```bash
# View questions
fspec query work-unit AUTH-001 --show-questions

# Returns:
{
  "id": "AUTH-001",
  "questions": [
    "@bob: Should we support GitHub Enterprise?",
    "@security-team: Do we need PKCE flow?"
  ]
}

# Answer by removing question and updating rules/examples
fspec remove-question AUTH-001 0  # Remove first question
fspec add-rule AUTH-001 "Support GitHub.com only, not Enterprise"

# Or answer inline
fspec answer-question AUTH-001 0 "No GitHub Enterprise support needed"
# This removes the question and adds to assumptions or rules
```

### Converting Examples to Scenarios

**After example mapping:**
```bash
# Generate Gherkin scenarios from examples
fspec generate-scenarios AUTH-001

# Creates/updates feature file with scenarios based on examples
# Tags scenarios with @AUTH-001
```

**Example transformation:**

Example mapping:
```
Example: "User logs in with Google account"
```

Generated Gherkin:
```gherkin
@AUTH-001
Scenario: User logs in with Google account
  Given the user is not authenticated
  When the user clicks "Login with Google"
  And completes Google OAuth flow
  Then the user should be authenticated
  And a session token should be created
```

AI then refines the steps during `specifying` state.

### Example Mapping Data Structure

```json
{
  "AUTH-001": {
    "status": "specifying",
    "rules": [
      "Users must authenticate before accessing protected resources",
      "OAuth tokens expire after 1 hour",
      "Refresh tokens valid for 30 days"
    ],
    "examples": [
      "User logs in with Google account",
      "User logs in with expired token",
      "User token auto-refreshes before expiry",
      "User logs out and token is invalidated"
    ],
    "questions": [
      "@bob: Should we support GitHub Enterprise? (added 2025-01-15)",
      "@security-team: Do we need PKCE flow? (added 2025-01-15)"
    ],
    "assumptions": [
      "Users have valid OAuth accounts",
      "Network is available for OAuth callbacks",
      "OAuth provider APIs are stable and available"
    ]
  }
}
```

---

## 10. Data Model

### File Structure

```
spec/
├── work-units.json          # All work unit data
├── epics.json               # Epic definitions
├── prefixes.json            # Prefix configuration
├── features/                # Gherkin feature files (tagged with work unit IDs)
│   ├── authentication.feature
│   ├── dashboard.feature
│   └── api.feature
├── foundation.json          # Existing foundation data
└── tags.json                # Existing tag registry
```

### spec/prefixes.json

Configuration for ID prefixes:

```json
{
  "AUTH": {
    "name": "Authentication",
    "description": "Authentication and authorization features",
    "color": "#FF6B6B"
  },
  "DASH": {
    "name": "Dashboard",
    "description": "User dashboard and widgets",
    "color": "#4ECDC4"
  },
  "API": {
    "name": "API",
    "description": "Backend API endpoints",
    "color": "#45B7D1"
  },
  "SEC": {
    "name": "Security",
    "description": "Security improvements and hardening",
    "color": "#F7B731"
  },
  "PERF": {
    "name": "Performance",
    "description": "Performance optimizations",
    "color": "#5F27CD"
  }
}
```

### spec/epics.json

Epic definitions:

```json
{
  "epic-user-management": {
    "name": "User Management",
    "description": "Complete user lifecycle from signup to session management",
    "goals": [
      "Secure authentication with OAuth 2.0",
      "User profile management",
      "Session handling and logout"
    ],
    "workUnits": ["AUTH-001", "AUTH-002", "DASH-001", "API-003"],
    "status": "active",
    "createdAt": "2025-01-10T00:00:00Z"
  },
  "epic-security-hardening": {
    "name": "Security Hardening",
    "description": "Q1 2025 security improvements",
    "goals": [
      "Rate limiting on all endpoints",
      "SQL injection prevention",
      "XSS protection"
    ],
    "workUnits": ["SEC-001", "SEC-002", "API-005"],
    "status": "active",
    "createdAt": "2025-01-12T00:00:00Z"
  }
}
```

### spec/work-units.json

Primary data structure organizing work units by state:

```json
{
  "meta": {
    "version": "1.0.0",
    "lastUpdated": "2025-01-15T14:30:00Z"
  },
  "states": {
    "backlog": ["AUTH-005", "DASH-003", "API-010"],
    "specifying": ["AUTH-001", "SEC-001"],
    "testing": ["AUTH-002"],
    "implementing": ["DASH-001", "API-003"],
    "validating": ["AUTH-003"],
    "done": ["AUTH-000", "API-001", "API-002"],
    "blocked": ["DASH-002"]
  },
  "workUnits": {
    "AUTH-001": {
      "id": "AUTH-001",
      "title": "Implement OAuth 2.0 login flow",
      "description": "Add OAuth 2.0 authentication supporting Google and GitHub providers",
      "status": "specifying",
      "createdAt": "2025-01-15T10:00:00Z",
      "updatedAt": "2025-01-15T14:30:00Z",

      "epic": "epic-user-management",
      "parent": null,
      "children": ["AUTH-002", "AUTH-003"],
      "tags": ["@security", "@critical"],

      "blocks": ["DASH-001"],
      "blockedBy": [],
      "dependsOn": ["API-001"],
      "relatesTo": ["SEC-001", "PERF-003"],

      "estimate": 8,
      "actualTokens": null,
      "iterations": null,
      "failedValidations": null,

      "rules": [
        "Users must authenticate before accessing protected resources",
        "OAuth tokens expire after 1 hour"
      ],
      "examples": [
        "User logs in with Google account",
        "User logs in with expired token"
      ],
      "questions": [
        "@bob: Should we support GitHub Enterprise?"
      ],
      "assumptions": [
        "Users have valid OAuth accounts"
      ],

      "stateHistory": [
        { "state": "backlog", "timestamp": "2025-01-15T10:00:00Z" },
        { "state": "specifying", "timestamp": "2025-01-15T12:00:00Z" }
      ]
    },
    "AUTH-002": {
      "id": "AUTH-002",
      "title": "OAuth provider integration - Google",
      "status": "testing",
      "parent": "AUTH-001",
      "estimate": 3,
      "..."
    }
  }
}
```

**Key design decisions:**

1. **States object**: Maintains ordered arrays of work unit IDs by state
   - Order in array = priority (first = highest)
   - CAGE can reorder by moving IDs around
   - Fast queries: "show all backlog items"

2. **workUnits object**: Full work unit data keyed by ID
   - Single source of truth for work unit metadata
   - Denormalized (status stored in both places) for query performance

3. **Auto-incrementing IDs**: Count existing work units with prefix
   - `AUTH-003` is next if `AUTH-001` and `AUTH-002` exist
   - Simple, no separate counter needed

### JSON Schema Validation

Work units validated with JSON Schema (using Ajv):

```typescript
// src/schemas/work-unit-schema.json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["id", "title", "status"],
  "properties": {
    "id": {
      "type": "string",
      "pattern": "^[A-Z]{2,6}-\\d+$"
    },
    "title": {
      "type": "string",
      "minLength": 1,
      "maxLength": 200
    },
    "status": {
      "type": "string",
      "enum": ["backlog", "specifying", "testing", "implementing", "validating", "done", "blocked"]
    },
    "estimate": {
      "type": "number",
      "enum": [1, 2, 3, 5, 8, 13]
    }
    // ... more fields
  }
}
```

---

## 11. Command Structure

### Work Unit Management

**Create work unit:**
```bash
# Minimal creation (just title, defaults to backlog)
fspec create-work-unit AUTH "Implement OAuth login"
# Creates AUTH-001 (auto-increments)

# With options
fspec create-work-unit AUTH "Implement OAuth login" \
  --epic=epic-user-management \
  --parent=AUTH-000 \
  --description="Add OAuth 2.0 with Google and GitHub providers"

# Returns:
{
  "success": true,
  "workUnit": {
    "id": "AUTH-001",
    "title": "Implement OAuth login",
    "status": "backlog"
  }
}
```

**Update work unit:**
```bash
# Update status (state transition)
fspec update-work-unit AUTH-001 --status=specifying

# Update multiple fields
fspec update-work-unit AUTH-001 \
  --status=implementing \
  --estimate=5 \
  --description="Updated description"

# CAGE writes back metrics
fspec update-work-unit AUTH-001 \
  --status=done \
  --actual-tokens=125000 \
  --iterations=3 \
  --failed-validations=1
```

**Delete work unit:**
```bash
# Delete (with confirmation)
fspec delete-work-unit AUTH-001

# Force delete (no confirmation)
fspec delete-work-unit AUTH-001 --force

# Cannot delete if:
# - Has children (delete children first)
# - Is blocking other work (resolve blocks first)
```

**Show work unit:**
```bash
# Show full details
fspec show-work-unit AUTH-001

# Show with related info
fspec show-work-unit AUTH-001 \
  --show-blockers \
  --show-children \
  --show-coverage

# Returns:
{
  "workUnit": { ... },
  "blockers": [ ... ],
  "children": [ ... ],
  "coverage": {
    "features": ["spec/features/authentication.feature"],
    "scenarios": ["Login with OAuth", "Handle token refresh"]
  }
}
```

### Example Mapping Commands

**Add rules:**
```bash
fspec add-rule AUTH-001 "Users must authenticate before accessing protected resources"
fspec add-rule AUTH-001 "OAuth tokens expire after 1 hour"
```

**Add examples:**
```bash
fspec add-example AUTH-001 "User logs in with Google account"
fspec add-example AUTH-001 "User logs in with expired token"
```

**Add questions:**
```bash
fspec add-question AUTH-001 "@bob: Should we support GitHub Enterprise?"
fspec add-question AUTH-001 "What happens when OAuth provider is down?"
```

**Add assumptions:**
```bash
fspec add-assumption AUTH-001 "Users have valid OAuth accounts"
```

**Remove items:**
```bash
# Remove by index
fspec remove-rule AUTH-001 0       # Remove first rule
fspec remove-example AUTH-001 1    # Remove second example
fspec remove-question AUTH-001 0   # Remove first question
fspec remove-assumption AUTH-001 2 # Remove third assumption
```

**Answer questions:**
```bash
# Answer question (removes question, optionally adds rule/assumption)
fspec answer-question AUTH-001 0 \
  --answer="No GitHub Enterprise support needed" \
  --add-to=assumptions

# Just remove question
fspec remove-question AUTH-001 0
```

**Generate scenarios from examples:**
```bash
# Generate Gherkin scenarios from examples
fspec generate-scenarios AUTH-001

# Creates/updates feature file with scenarios
# Tags with @AUTH-001
```

### Dependency Management

**Add dependencies:**
```bash
# This blocks other work
fspec add-dependency AUTH-001 --blocks DASH-001 DASH-002

# This is blocked by other work
fspec add-dependency AUTH-001 --blocked-by API-001 SEC-001

# Soft dependency
fspec add-dependency AUTH-001 --depends-on SEC-002

# Related work
fspec add-dependency AUTH-001 --relates-to PERF-003 DOC-005
```

**Remove dependencies:**
```bash
fspec remove-dependency AUTH-001 --blocks DASH-001
fspec remove-dependency AUTH-001 --blocked-by API-001
```

**Validate dependencies:**
```bash
# Check for circular dependencies
fspec validate-dependencies

# Returns errors if cycles detected
```

### Priority Management

**Reorder work in state:**
```bash
# Move to top (highest priority)
fspec prioritize AUTH-003 --position=top

# Move to bottom (lowest priority)
fspec prioritize AUTH-003 --position=bottom

# Move to specific position (0-indexed)
fspec prioritize AUTH-003 --position=2

# Move before another work unit
fspec prioritize AUTH-003 --before=AUTH-001

# Move after another work unit
fspec prioritize AUTH-003 --after=AUTH-002
```

### Querying Work Units

**List work units:**
```bash
# All work units
fspec list-work-units

# By status
fspec list-work-units --status=backlog
fspec list-work-units --status=implementing

# By epic
fspec list-work-units --epic=epic-user-management

# By prefix
fspec list-work-units --prefix=AUTH

# Complex queries
fspec list-work-units \
  --status=backlog \
  --epic=epic-user-management \
  --points-gte=5 \
  --has-questions

# Output formats
fspec list-work-units --output=json
fspec list-work-units --output=table
fspec list-work-units --output=ids  # Just IDs for piping
```

**Query work units (advanced):**
```bash
# Get raw JSON for all work units
fspec query work-units --output=json

# Filter by criteria
fspec query work-units \
  --status=blocked \
  --output=json

# Get specific work unit
fspec query work-unit AUTH-001 --output=json

# Show blockers
fspec query work-unit AUTH-001 --show-blockers --output=json

# Show what this blocks
fspec query work-unit AUTH-001 --show-blocks --output=json

# Show children
fspec query work-unit AUTH-001 --show-children --output=json

# Show coverage (which features/scenarios)
fspec query work-unit AUTH-001 --show-coverage --output=json
```

**Query metrics:**
```bash
# Overall metrics
fspec query metrics --output=json
# Returns: counts by state, points by state, velocity, etc.

# Estimate accuracy
fspec query metrics --type=estimate-accuracy --output=json
# Returns: avg tokens per story point

# Cycle time
fspec query metrics --type=cycle-time --output=json
# Returns: avg time in each state

# Velocity
fspec query metrics --type=velocity --output=json
# Returns: points completed over time

# Bottlenecks
fspec query metrics --type=bottlenecks --output=json
# Returns: where work is getting stuck
```

### Epic Management

**Create epic:**
```bash
fspec create-epic epic-user-management \
  --name="User Management" \
  --description="Complete user lifecycle"
```

**Update epic:**
```bash
fspec update-epic epic-user-management \
  --name="Updated name" \
  --description="Updated description"
```

**List epics:**
```bash
fspec list-epics

# With work unit counts
fspec list-epics --show-counts

# Output as JSON
fspec list-epics --output=json
```

**Delete epic:**
```bash
# Delete epic (doesn't delete work units, just unassigns them)
fspec delete-epic epic-user-management
```

### Prefix Management

**Create prefix:**
```bash
fspec create-prefix AUTH \
  --name="Authentication" \
  --description="Auth features" \
  --color="#FF6B6B"
```

**List prefixes:**
```bash
fspec list-prefixes

# With work unit counts
fspec list-prefixes --show-counts
```

**Delete prefix:**
```bash
# Cannot delete if work units exist with this prefix
fspec delete-prefix AUTH
```

### Validation Commands

**Validate work units:**
```bash
# Validate all work units
fspec validate-work-units

# Checks:
# - JSON schema compliance
# - No circular dependencies
# - Parent/child consistency
# - Blocked work in correct state
# - Story points in Fibonacci sequence
```

**Validate coverage:**
```bash
# Check all work units have feature coverage
fspec validate-coverage

# Warns if:
# - Work unit in 'testing' state has no scenarios tagged
# - Work unit marked 'done' but no scenarios exist
```

### Reporting Commands

**Kanban board view:**
```bash
# ASCII kanban board
fspec board

# Output:
┌─────────────┬─────────────┬─────────────┬─────────────┬─────────────┬─────────────┐
│  backlog    │ specifying  │   testing   │implementing │ validating  │    done     │
│   (3)       │    (2)      │    (1)      │    (2)      │    (1)      │   (15)      │
├─────────────┼─────────────┼─────────────┼─────────────┼─────────────┼─────────────┤
│ AUTH-005    │ AUTH-001    │ AUTH-002    │ DASH-001    │ AUTH-003    │ AUTH-000    │
│ [8 pts]     │ [8 pts]     │ [3 pts]     │ [5 pts]     │ [5 pts]     │ [3 pts]     │
│             │             │             │             │             │             │
│ DASH-003    │ SEC-001     │             │ API-003     │             │ API-001     │
│ [5 pts]     │ [5 pts]     │             │ [8 pts]     │             │ [5 pts]     │
│             │             │             │             │             │             │
│ API-010     │             │             │             │             │ ...         │
│ [3 pts]     │             │             │             │             │             │
└─────────────┴─────────────┴─────────────┴─────────────┴─────────────┴─────────────┘

Total: 16 points in progress | 45 points completed
```

**Burnup chart (ASCII):**
```bash
fspec burnup --period=30days

# Shows points completed over time
```

**Blocked items report:**
```bash
fspec blocked

# Shows all blocked work with reasons
```

**Summary report:**
```bash
fspec summary

# Output:
Project Summary
===============
Work Units: 24 total
  - Backlog: 3 (16 points)
  - In Progress: 6 (34 points)
  - Done: 15 (45 points)
  - Blocked: 0

Velocity: 15 points/week (last 2 weeks)

Top Blockers:
  - API-001: Blocking 3 work units (18 points)
  - SEC-001: Blocking 1 work unit (5 points)

Longest in state:
  - AUTH-002: 4 days in testing
  - DASH-001: 3 days in implementing
```

---

## 12. Integration with CAGE

### CAGE Responsibilities

**Workflow orchestration:**
1. Query available work: `fspec query work-units --status=backlog --output=json`
2. Check dependencies: Ensure `blockedBy` items are done
3. Pull work: `fspec update-work-unit AUTH-001 --status=specifying`
4. Execute ACDD workflow (spec → test → code)
5. Record metrics: `fspec update-work-unit AUTH-001 --actual-tokens=125000`
6. Handle failures: Move back to appropriate state
7. Reorder priorities: `fspec prioritize AUTH-003 --position=top`

**State transition validation:**
```bash
# CAGE attempts transition
fspec update-work-unit AUTH-001 --status=testing

# fspec validates:
# - Are there Gherkin scenarios tagged with @AUTH-001?
# - Is estimate assigned?
# - Are all questions answered?
# - Are dependencies met?

# If validation fails, returns error and suggested fix
```

**Example mapping facilitation:**
```bash
# CAGE facilitates example mapping for work in 'specifying' state
# 1. Check for questions
questions=$(fspec query work-unit AUTH-001 --output=json | jq '.questions')

# 2. If questions exist, notify humans or wait
if [ ! -z "$questions" ]; then
  # Notify mentioned people
  # Move to blocked if needed
  fspec update-work-unit AUTH-001 --status=blocked --blocked-reason="Waiting for answers"
fi

# 3. Once questions answered, generate scenarios
fspec generate-scenarios AUTH-001

# 4. Transition to testing
fspec update-work-unit AUTH-001 --status=testing
```

**Metrics collection:**
```bash
# CAGE tracks tokens during work
tokens_used=0

# During specifying
tokens_used=$((tokens_used + 15000))

# During testing
tokens_used=$((tokens_used + 35000))

# During implementing (with retries)
for attempt in 1 2 3; do
  # Attempt implementation
  tokens_used=$((tokens_used + 20000))

  # Check if tests pass
  if tests_pass; then
    break
  fi
done

# Write back metrics
fspec update-work-unit AUTH-001 \
  --actual-tokens=$tokens_used \
  --iterations=$attempt \
  --failed-validations=2
```

### CAGE Hooks Integration

**Pre-transition hooks:**
```bash
# In CAGE config: ~/.cage/hooks/pre-work-unit-transition.sh
#!/bin/bash
work_unit=$1
from_state=$2
to_state=$3

# Example: Enforce ACDD - can't go to implementing without tests
if [ "$to_state" == "implementing" ]; then
  # Check if tests exist and fail
  if ! tests_exist_and_fail "$work_unit"; then
    echo "ERROR: No failing tests found. Write tests first (ACDD)."
    exit 1
  fi
fi

exit 0
```

**Post-transition hooks:**
```bash
# In CAGE config: ~/.cage/hooks/post-work-unit-transition.sh
#!/bin/bash
work_unit=$1
from_state=$2
to_state=$3

# Example: When work goes to 'done', check if it unblocks anything
if [ "$to_state" == "done" ]; then
  # Find work units blocked by this one
  blocked=$(fspec query work-unit "$work_unit" --show-blocks --output=json)

  # Notify or auto-transition blocked work
  # ...
fi

exit 0
```

### CAGE Decision Making

**What to work on next:**
```bash
#!/bin/bash
# CAGE script: decide-next-work.sh

# Get all backlog work
backlog=$(fspec query work-units --status=backlog --output=json)

# Filter out blocked work
available=$(echo "$backlog" | jq '[.[] | select(.blockedBy | length == 0)]')

# Sort by:
# 1. What blocks the most other work (maximize unblocking)
# 2. Highest priority (position in backlog array)
# 3. Smallest estimate (quick wins)

next_work=$(echo "$available" | jq -r '
  sort_by(
    -.blocks | length,
    .priority,
    .estimate
  ) | .[0].id
')

echo "Next work unit: $next_work"

# Pull it
fspec update-work-unit "$next_work" --status=specifying
```

**Handling failures:**
```bash
#!/bin/bash
# CAGE script: handle-validation-failure.sh

work_unit=$1
failure_type=$2

case "$failure_type" in
  "test_failure")
    # Move back to implementing
    fspec update-work-unit "$work_unit" --status=implementing
    ;;
  "spec_error")
    # Move back to specifying
    fspec update-work-unit "$work_unit" --status=specifying
    ;;
  "build_error")
    # Move back to implementing
    fspec update-work-unit "$work_unit" --status=implementing
    ;;
  "unknown")
    # Move to blocked for investigation
    fspec update-work-unit "$work_unit" \
      --status=blocked \
      --blocked-reason="Unknown validation failure: $3"
    ;;
esac
```

### JSON API for CAGE

All commands support `--output=json` for programmatic access:

```bash
# Get work to do
work=$(fspec query work-units --status=backlog --output=json | jq -r '.[0].id')

# Get work details
details=$(fspec query work-unit "$work" --output=json)

# Extract fields
title=$(echo "$details" | jq -r '.title')
estimate=$(echo "$details" | jq -r '.estimate // 0')
blockers=$(echo "$details" | jq -r '.blockedBy | length')

# Make decisions
if [ $blockers -eq 0 ]; then
  fspec update-work-unit "$work" --status=specifying
fi
```

---

## 13. Query & Reporting

### Query Language

**Basic queries:**
```bash
# All work units
fspec query work-units --output=json

# Filter by status
fspec query work-units --status=backlog --output=json

# Filter by epic
fspec query work-units --epic=epic-user-management --output=json

# Filter by prefix
fspec query work-units --prefix=AUTH --output=json

# Filter by points
fspec query work-units --points-gte=5 --output=json
fspec query work-units --points-lte=3 --output=json
fspec query work-units --points-eq=5 --output=json

# Filter by tags
fspec query work-units --tag=@security --output=json

# Filter by dependencies
fspec query work-units --has-blockers --output=json
fspec query work-units --is-blocking --output=json

# Filter by questions
fspec query work-units --has-questions --output=json

# Combine filters (AND logic)
fspec query work-units \
  --status=backlog \
  --epic=epic-user-management \
  --points-gte=5 \
  --output=json
```

### Metric Queries

**Overall metrics:**
```bash
fspec query metrics --output=json

# Returns:
{
  "byState": {
    "backlog": { "count": 3, "points": 16 },
    "specifying": { "count": 2, "points": 13 },
    "testing": { "count": 1, "points": 3 },
    "implementing": { "count": 2, "points": 13 },
    "validating": { "count": 1, "points": 5 },
    "done": { "count": 15, "points": 45 },
    "blocked": { "count": 0, "points": 0 }
  },
  "byEpic": {
    "epic-user-management": { "count": 8, "points": 34, "completed": 3 },
    "epic-security-hardening": { "count": 5, "points": 21, "completed": 2 }
  },
  "byPrefix": {
    "AUTH": { "count": 6, "points": 28, "completed": 2 },
    "DASH": { "count": 4, "points": 18, "completed": 1 }
  },
  "velocity": {
    "lastWeek": 15,
    "last2Weeks": 28,
    "last4Weeks": 52
  }
}
```

**Estimate accuracy:**
```bash
fspec query metrics --type=estimate-accuracy --output=json

# Returns:
{
  "1-point": {
    "samples": 12,
    "avgTokens": 25000,
    "minTokens": 18000,
    "maxTokens": 35000,
    "stdDev": 4500
  },
  "2-point": {
    "samples": 8,
    "avgTokens": 45000,
    "minTokens": 38000,
    "maxTokens": 55000,
    "stdDev": 6000
  },
  "3-point": {
    "samples": 15,
    "avgTokens": 75000,
    "minTokens": 60000,
    "maxTokens": 95000,
    "stdDev": 9000
  }
}
```

**Cycle time analysis:**
```bash
fspec query metrics --type=cycle-time --output=json

# Returns:
{
  "avgCycleTime": "4.5 hours",
  "byState": {
    "backlog": { "avgHours": 2.5, "minHours": 0.5, "maxHours": 8.0 },
    "specifying": { "avgHours": 1.2, "minHours": 0.3, "maxHours": 3.5 },
    "testing": { "avgHours": 0.8, "minHours": 0.2, "maxHours": 2.0 },
    "implementing": { "avgHours": 1.5, "minHours": 0.5, "maxHours": 4.0 },
    "validating": { "avgHours": 3.2, "minHours": 0.5, "maxHours": 8.0 }
  },
  "bottleneck": "validating"
}
```

**Blocked items:**
```bash
fspec query work-units --status=blocked --output=json

# Or specialized command
fspec blocked --output=json

# Returns:
[
  {
    "id": "DASH-002",
    "title": "User profile widget",
    "blockedBy": ["AUTH-001"],
    "blockedSince": "2025-01-14T10:00:00Z",
    "blockedDuration": "1.5 days"
  }
]
```

### Dependency Graph Queries

**Get dependency graph:**
```bash
fspec query dependencies --output=json

# Returns adjacency list:
{
  "AUTH-001": {
    "blocks": ["DASH-001", "DASH-002"],
    "blockedBy": ["API-001"],
    "dependsOn": ["SEC-001"],
    "relatesTo": ["PERF-003"]
  },
  "API-001": {
    "blocks": ["AUTH-001", "DASH-003"],
    "blockedBy": [],
    "dependsOn": [],
    "relatesTo": []
  }
}

# Or as DOT format for visualization
fspec query dependencies --output=dot > graph.dot
graphviz graph.dot -o graph.png
```

**Critical path:**
```bash
fspec query critical-path --output=json

# Returns work units on critical path (longest dependency chain)
[
  {
    "id": "API-001",
    "depth": 0,
    "blocks": 3,
    "totalPointsBlocked": 24
  },
  {
    "id": "AUTH-001",
    "depth": 1,
    "blocks": 2,
    "totalPointsBlocked": 13
  }
]
```

### Coverage Queries

**Check feature coverage:**
```bash
fspec query coverage --output=json

# Returns:
{
  "AUTH-001": {
    "features": ["spec/features/authentication.feature"],
    "scenarios": [
      "Login with OAuth",
      "Handle token refresh"
    ],
    "scenarioCount": 2
  },
  "AUTH-002": {
    "features": [],
    "scenarios": [],
    "scenarioCount": 0,
    "warning": "No scenarios tagged with @AUTH-002"
  }
}
```

**Find untagged scenarios:**
```bash
fspec query untagged-scenarios --output=json

# Returns scenarios not tagged with any work unit ID
[
  {
    "feature": "spec/features/dashboard.feature",
    "scenario": "Display user metrics",
    "line": 25
  }
]
```

### Export for External Tools

**Export to CSV:**
```bash
fspec export --format=csv --output=work-units.csv

# Columns: id, title, status, epic, estimate, actualTokens, ...
```

**Export to GitHub Issues (example):**
```bash
# Get all backlog items
fspec query work-units --status=backlog --output=json | \
  jq -r '.[] | "Title: \(.title)\nBody: \(.description)\nLabels: \(.epic)"' | \
  # Pipe to gh CLI or API
```

**Export to Mermaid gantt chart:**
```bash
fspec export --format=mermaid-gantt --output=gantt.mmd

# Generates Mermaid gantt syntax based on dependencies
```

---

## 14. Example Workflows

### Workflow 1: Creating and Completing a Work Unit

**Step 1: Create work unit**
```bash
$ fspec create-work-unit AUTH "Implement OAuth 2.0 login"

Created work unit: AUTH-001
Title: Implement OAuth 2.0 login
Status: backlog

$ fspec list-work-units --status=backlog
Backlog (1):
  AUTH-001: Implement OAuth 2.0 login
```

**Step 2: CAGE pulls work and starts example mapping**
```bash
$ fspec update-work-unit AUTH-001 --status=specifying

$ fspec add-rule AUTH-001 "Users must authenticate before accessing protected resources"
$ fspec add-rule AUTH-001 "OAuth tokens expire after 1 hour"

$ fspec add-example AUTH-001 "User logs in with Google account"
$ fspec add-example AUTH-001 "User logs in with expired token"

$ fspec add-question AUTH-001 "@bob: Should we support GitHub Enterprise?"

$ fspec query work-unit AUTH-001 --output=json
{
  "id": "AUTH-001",
  "status": "specifying",
  "rules": [...],
  "examples": [...],
  "questions": ["@bob: Should we support GitHub Enterprise?"]
}
```

**Step 3: Human answers question**
```bash
$ fspec answer-question AUTH-001 0 "No GitHub Enterprise, only GitHub.com"

$ fspec add-assumption AUTH-001 "Only GitHub.com supported, not Enterprise"
```

**Step 4: Generate scenarios and estimate**
```bash
$ fspec generate-scenarios AUTH-001
Generated scenarios in spec/features/authentication.feature

$ fspec update-work-unit AUTH-001 --estimate=5
```

**Step 5: Transition to testing**
```bash
$ fspec update-work-unit AUTH-001 --status=testing

# CAGE writes tests...

$ fspec update-work-unit AUTH-001 --status=implementing
```

**Step 6: Implement and validate**
```bash
# CAGE writes code...

$ fspec update-work-unit AUTH-001 --status=validating

# CAGE runs validations...

$ fspec update-work-unit AUTH-001 \
  --status=done \
  --actual-tokens=95000 \
  --iterations=2 \
  --failed-validations=1
```

**Step 7: Check metrics**
```bash
$ fspec query work-unit AUTH-001 --output=json
{
  "id": "AUTH-001",
  "status": "done",
  "estimate": 5,
  "actualTokens": 95000,
  "iterations": 2,
  "stateHistory": [...]
}

$ fspec query metrics --type=estimate-accuracy
5-point tasks: avg 95000 tokens (based on 1 sample)
```

### Workflow 2: Managing Dependencies

**Step 1: Create foundation work**
```bash
$ fspec create-work-unit API "User authentication endpoint"
Created: API-001

$ fspec update-work-unit API-001 --status=implementing
```

**Step 2: Create dependent work**
```bash
$ fspec create-work-unit AUTH "OAuth login flow"
Created: AUTH-001

$ fspec add-dependency AUTH-001 --blocked-by API-001

$ fspec query work-unit AUTH-001 --show-blockers --output=json
{
  "id": "AUTH-001",
  "blockedBy": ["API-001"],
  "blockers": {
    "API-001": {
      "status": "implementing",
      "estimate": 5
    }
  }
}
```

**Step 3: CAGE tries to pull blocked work**
```bash
$ fspec update-work-unit AUTH-001 --status=specifying

ERROR: Cannot transition AUTH-001 to specifying
Blocked by: API-001 (status: implementing)
```

**Step 4: Complete blocker**
```bash
$ fspec update-work-unit API-001 --status=done

$ fspec query work-unit AUTH-001 --show-blockers
No blockers. Ready to start.

$ fspec update-work-unit AUTH-001 --status=specifying
Success.
```

### Workflow 3: Breaking Down Large Work

**Step 1: Create parent work unit**
```bash
$ fspec create-work-unit AUTH "Complete OAuth 2.0 implementation"
Created: AUTH-001

$ fspec update-work-unit AUTH-001 --estimate=13
WARNING: 13-point work unit. Consider breaking down.
```

**Step 2: Break into children**
```bash
$ fspec create-work-unit AUTH "OAuth provider integration" --parent=AUTH-001
Created: AUTH-002

$ fspec create-work-unit AUTH "Token storage and refresh" --parent=AUTH-001
Created: AUTH-003

$ fspec create-work-unit AUTH "Login UI components" --parent=AUTH-001
Created: AUTH-004

$ fspec update-work-unit AUTH-002 --estimate=3
$ fspec update-work-unit AUTH-003 --estimate=5
$ fspec update-work-unit AUTH-004 --estimate=3

$ fspec query work-unit AUTH-001 --show-children
AUTH-001: Complete OAuth 2.0 implementation (13 points)
  Children:
    AUTH-002: OAuth provider integration (3 points) [backlog]
    AUTH-003: Token storage and refresh (5 points) [backlog]
    AUTH-004: Login UI components (3 points) [backlog]

  Total child points: 11
```

**Step 3: Work on children**
```bash
$ fspec update-work-unit AUTH-002 --status=specifying
# ... complete AUTH-002 ...
$ fspec update-work-unit AUTH-002 --status=done

$ fspec update-work-unit AUTH-003 --status=specifying
# ... complete AUTH-003 ...
```

**Step 4: Try to complete parent (fails)**
```bash
$ fspec update-work-unit AUTH-001 --status=done

ERROR: Cannot mark AUTH-001 as done
Incomplete children:
  AUTH-004: Login UI components [backlog]
```

**Step 5: Complete all children**
```bash
$ fspec update-work-unit AUTH-004 --status=done

$ fspec update-work-unit AUTH-001 --status=done
Success. All children complete.
```

### Workflow 4: Using Example Mapping

**Step 1: Start discovery**
```bash
$ fspec create-work-unit AUTH "Password reset flow"
Created: AUTH-005

$ fspec update-work-unit AUTH-005 --status=specifying
```

**Step 2: Example mapping session**
```bash
# Add rules discovered
$ fspec add-rule AUTH-005 "Password reset links expire after 1 hour"
$ fspec add-rule AUTH-005 "Users can only have one active reset link"
$ fspec add-rule AUTH-005 "Reset requires email verification"

# Add examples (will become scenarios)
$ fspec add-example AUTH-005 "User requests password reset"
$ fspec add-example AUTH-005 "User clicks reset link and sets new password"
$ fspec add-example AUTH-005 "User tries to use expired reset link"
$ fspec add-example AUTH-005 "User requests multiple resets (only last one valid)"

# Add questions for clarification
$ fspec add-question AUTH-005 "@product: How many reset attempts before lockout?"
$ fspec add-question AUTH-005 "@security: Should we send notification when password changed?"

# Add assumptions
$ fspec add-assumption AUTH-005 "User has access to email"
$ fspec add-assumption AUTH-005 "Email service is reliable"
```

**Step 3: Check questions**
```bash
$ fspec query work-unit AUTH-005
AUTH-005: Password reset flow
Status: specifying
Questions (2):
  1. @product: How many reset attempts before lockout?
  2. @security: Should we send notification when password changed?
```

**Step 4: Block until answered**
```bash
$ fspec update-work-unit AUTH-005 --status=blocked --blocked-reason="Waiting for question answers"

# Later, humans answer...
$ fspec answer-question AUTH-005 0 "3 attempts within 24 hours" --add-to=rules
$ fspec answer-question AUTH-005 1 "Yes, send email notification" --add-to=rules

$ fspec update-work-unit AUTH-005 --status=specifying
```

**Step 5: Generate scenarios**
```bash
$ fspec generate-scenarios AUTH-005
Generated 4 scenarios in spec/features/password-reset.feature:
  - User requests password reset
  - User clicks reset link and sets new password
  - User tries to use expired reset link
  - User requests multiple resets (only last one valid)

All scenarios tagged with @AUTH-005
```

**Step 6: Estimate and continue**
```bash
$ fspec update-work-unit AUTH-005 --estimate=5

$ fspec update-work-unit AUTH-005 --status=testing
```

---

## 15. Open Questions

### Questions to Resolve

1. **State transitions enforcement:**
   - How strictly should fspec enforce state transitions?
   - Should CAGE be able to override validations?
   - Example: Jump from `backlog` to `implementing` in emergencies?

2. **Metrics granularity:**
   - Should we track token usage per state (specifying, testing, implementing)?
   - Or just total tokens for the entire work unit?
   - How much overhead is acceptable?

3. **Example mapping storage:**
   - Should rules/examples/questions be in work-units.json?
   - Or separate file (spec/example-mapping.json)?
   - Or both (keep JSON, also write to markdown for human readability)?

4. **Scenario generation:**
   - Should `fspec generate-scenarios` be automatic during state transitions?
   - Or manual command only?
   - How much AI vs template-based generation?

5. **Epic vs Prefix:**
   - Is the distinction clear enough?
   - Do we need both, or could epics have prefixes?
   - Example: `epic-auth` → use `AUTH` prefix?

6. **Blocked state:**
   - Is `blocked` a separate state, or a flag on any state?
   - Can work be `implementing + blocked` simultaneously?
   - How does this affect queries and reporting?

7. **Work unit deletion:**
   - Should deleted work units be archived (soft delete)?
   - Or permanently removed (hard delete)?
   - How long to keep `done` work units in active file?

8. **Version control:**
   - Should work-units.json be one big file or split by state?
   - Merge conflict potential with CAGE updating frequently?
   - Should we use Git locking or conflict resolution strategy?

9. **Notifications:**
   - Should fspec handle notifications (@bob in questions)?
   - Or leave that to CAGE?
   - Integration with email/Slack/etc?

10. **Historical data:**
    - How long to keep state history?
    - When to archive old `done` work units?
    - Separate archive file (spec/work-units-archive.json)?

11. **Multi-agent coordination:**
    - If multiple AI agents work simultaneously, how to handle conflicts?
    - Pessimistic locking (claim work units)?
    - Optimistic locking (detect conflicts on update)?
    - First-come-first-serve based on file timestamps?

12. **Validation hooks:**
    - Should fspec have built-in validation hooks?
    - Or rely on CAGE to validate before state transitions?
    - Example: Check that tests exist before moving to `implementing`?

### Design Trade-offs to Consider

**Complexity vs Power:**
- Simple model (few fields, basic states) vs comprehensive (all metadata)
- Current design leans toward comprehensive - is that right?

**Performance:**
- Single JSON file vs multiple files
- File size concerns as work units grow into hundreds/thousands
- Query performance on large datasets

**Human vs Machine:**
- How much should be optimized for human readability?
- CLI output formatting vs JSON for machines
- Documentation burden

**Flexibility vs Convention:**
- Strict workflow vs allow customization
- Fixed states vs configurable states
- Enforced ACDD vs optional

---

## Next Steps

1. **Validate this design** with real usage scenarios
2. **Prototype core commands** (create, update, query)
3. **Define JSON schemas** for validation
4. **Build CAGE integration** proof of concept
5. **Test with actual ACDD workflow** on fspec itself
6. **Iterate based on learnings**

---

## Conclusion

This project management system transforms fspec from a Gherkin specification tool into a complete AI-native project management platform. By organizing work around tags, enforcing ACDD workflow through states, and providing rich querying capabilities, it enables AI agents like those orchestrated by CAGE to manage their own work effectively.

The key innovations are:

1. **Tag-based work units** - Work organized by tags, not files
2. **ACDD-aligned workflow** - States mirror spec → test → code progression
3. **Example mapping integration** - Structured discovery phase
4. **AI-centric metrics** - Track tokens and iterations, not hours
5. **Dependency management** - Prevent working on blocked items
6. **Progressive enrichment** - Work units gain detail as they flow
7. **CLI-first with JSON** - Everything queryable and automatable
8. **CAGE orchestration** - fspec stores, CAGE manages

This system enables AI agents to work like human teams, but optimized for how AI actually works.

---

**Document Status:** DRAFT - Brainstorming
**Last Updated:** 2025-01-15
**Next Review:** After initial CAGE integration prototype
