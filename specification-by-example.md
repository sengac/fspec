# Specification by Example and fspec

**fspec is a CLI implementation of the Specification by Example methodology for AI agents.**

This document explains the relationship between Gojko Adzic's "Specification by Example" book and the fspec tool, showing how fspec operationalizes SBE principles through enforced workflow, persistent state, and AI-first design.

---

## Table of Contents

1. [What is Specification by Example?](#what-is-specification-by-example)
2. [The SBE Process Pattern](#the-sbe-process-pattern)
3. [How fspec Implements SBE](#how-fspec-implements-sbe)
4. [How fspec Extends SBE](#how-fspec-extends-sbe)
5. [Historical Impact & Importance](#historical-impact--importance)
6. [Why SBE Matters for AI Agents](#why-sbe-matters-for-ai-agents)

---

## What is Specification by Example?

**Specification by Example (SBE)** is a collaborative approach to defining software requirements and tests, where teams use realistic examples to illustrate intended behavior and create executable specifications.

### Core Principles

1. **Concrete Examples**: Use real scenarios instead of abstract requirements
2. **Collaborative Discovery**: Business, development, and testing work together
3. **Living Documentation**: Specifications evolve with the codebase
4. **Executable Specifications**: Examples become automated tests
5. **Just-in-Time**: Specify only when ready to implement

### The Book

**"Specification by Example: How Successful Teams Deliver the Right Software"**
*By Gojko Adzic (2011)*

Based on research from 50+ teams across industries, this book codified practices from:
- Behavior-Driven Development (BDD)
- Acceptance Test-Driven Development (ATDD)
- Example-Driven Development
- Executable Specifications

---

## The SBE Process Pattern

![Figure 2.1 from Specification by Example](./docs/images/sbe-process-pattern.jpg)

The book identifies **9 key process patterns** that successful teams follow:

### 1. Deriving Scope from Goals
Start with business goals, derive scope into user stories/features

### 2. Illustrating Using Examples
Collaborative discovery using concrete examples before writing specs

### 3. Specifying Collaboratively
Team works together to refine examples into structured specifications

### 4. Refining the Specification
Transform examples into executable format (Given-When-Then)

### 5. Automating Validation Without Changing Specifications
Tests execute specifications without modifying them

### 6. Executable Specification
Specifications become automated tests that verify behavior

### 7. Validating Frequently
Run specifications continuously to catch regressions

### 8. Living Documentation
Specifications evolve with codebase, always up-to-date

### 9. Evolving a Documentation System
Documentation system that grows with the project

### Just-in-Time Principle

> "Successful teams don't implement the entire sequence at one time or for all the specifications shown in figure 2.1—especially not before development starts. Instead, teams derive the scope from goals once a team is ready for more work, for example, at the beginning of a project phase or a milestone. They provide specifications only when the team is ready to start implementing an item, such as at the start of the relevant iteration. Don't mistake the sequence in figure 2.1 for big Waterfall specifications."

---

## How fspec Implements SBE

Here's how each stage of the SBE cycle maps to fspec features:

### 1. Deriving Scope from Goals → **Work Units & Epics**

**SBE Process**: Start with business goals, derive scope into user stories/features

**fspec Implementation**:
- **Foundation Discovery**: `fspec discover-foundation` captures WHY (vision, problem space) and WHAT (capabilities, personas)
- **Work Units**: Created with `fspec create-work-unit` to represent stories/bugs/tasks
- **Epics**: High-level business initiatives grouping related work
- **Kanban Backlog**: Visual prioritization of scope

```bash
# Derive scope from business goals
fspec create-work-unit AUTH "User Login" --epic=user-management
fspec create-epic user-management "User Management Features"
```

**Artifacts**:
- `spec/foundation.json` - Project vision, capabilities, personas
- `spec/work-units.json` - Backlog of work derived from goals
- `spec/epics.json` - High-level business initiatives

---

### 2. Illustrating Using Examples → **Example Mapping**

**SBE Process**: Collaborative discovery using concrete examples before writing specs

**fspec Implementation**: This is where fspec **truly shines** with its Example Mapping commands:

- **Rules (Blue Cards)**: `fspec add-rule` - Business rules governing the feature
- **Examples (Green Cards)**: `fspec add-example` - Concrete scenarios illustrating rules
- **Questions (Red Cards)**: `fspec add-question` - Uncertainties needing answers
- **User Story**: `fspec set-user-story --role --action --benefit`

```bash
# Step 0: Set user story (avoids placeholders in generated scenarios)
fspec set-user-story AUTH-001 \
  --role "registered user" \
  --action "log in with email and password" \
  --benefit "I can access my account securely"

# Step 1: Capture business rules (Blue Cards)
fspec add-rule AUTH-001 "Password must be 8+ characters"
fspec add-rule AUTH-001 "Maximum 5 login attempts before account lock"
fspec add-rule AUTH-001 "Session expires after 30 minutes of inactivity"

# Step 2: Gather concrete examples (Green Cards)
fspec add-example AUTH-001 "User enters valid email/password, receives session token"
fspec add-example AUTH-001 "User enters wrong password 3 times, sees warning message"
fspec add-example AUTH-001 "User enters wrong password 5 times, account locked for 15 minutes"

# Step 3: Ask clarifying questions (Red Cards)
fspec add-question AUTH-001 "@human: Should we support OAuth in Phase 1?"
fspec add-question AUTH-001 "@human: What happens if user closes browser without logging out?"

# Step 4: Answer questions collaboratively
fspec answer-question AUTH-001 0 --answer "No, defer OAuth to Phase 2" --add-to assumption
fspec answer-question AUTH-001 1 --answer "Session persists until timeout" --add-to rule

# Step 5: View the complete example map
fspec show-work-unit AUTH-001
```

**This is exactly the Example Mapping conversation pattern from SBE!**

**Artifacts**:
- Rules, examples, questions stored in `spec/work-units.json` under each work unit
- Interactive conversation between AI agent and human developer

---

### 3. Specifying Collaboratively → **Generate Scenarios**

**SBE Process**: Team refines examples into structured specifications

**fspec Implementation**:
- **Auto-generate scenarios**: `fspec generate-scenarios <work-unit-id>` transforms Example Mapping data into Gherkin
- **Manual refinement**: `fspec add-scenario`, `fspec update-step`, etc.
- **Feature files**: Living documentation in `spec/features/*.feature`

```bash
# Transform examples into Gherkin scenarios
fspec generate-scenarios AUTH-001
# Creates: spec/features/user-authentication.feature

# Manual refinement if needed
fspec add-scenario user-authentication "Login with expired session"
fspec add-step user-authentication "Login with expired session" \
  given "I have a session that expired 35 minutes ago"
```

**Artifacts**:
- `spec/features/*.feature` - Gherkin feature files (living documentation)
- `spec/features/*.feature.coverage` - Coverage tracking files (auto-created)

---

### 4. Refining the Specification → **Gherkin Feature Files**

**SBE Process**: Structured specifications using Given-When-Then format

**fspec Implementation**:
- **Gherkin syntax**: Official Cucumber parser via `@cucumber/gherkin`
- **Validation**: `fspec validate` ensures correct syntax
- **Formatting**: `fspec format` provides consistent structure
- **Tags**: Organize and filter scenarios

```bash
# Validate Gherkin syntax
fspec validate spec/features/user-authentication.feature

# Format for consistency
fspec format spec/features/user-authentication.feature

# Add tags for organization
fspec add-tag-to-feature spec/features/user-authentication.feature @critical
fspec add-tag-to-feature spec/features/user-authentication.feature @security
```

**Example Feature File**:
```gherkin
@critical @security @authentication
Feature: User Authentication

  Background: User Story
    As a registered user
    I want to log in with email and password
    So that I can access my account securely

  Scenario: Login with valid credentials
    Given I am on the login page
    And I have a registered account
    When I enter my valid email and password
    Then I should be redirected to my dashboard
    And I should receive a session token

  Scenario: Login with invalid password triggers warning
    Given I am on the login page
    And I have attempted login 2 times with wrong password
    When I enter my email with wrong password again
    Then I should see "2 more attempts remaining" warning
    And I should not receive a session token
```

**Artifacts**:
- Validated, formatted Gherkin feature files
- Tags registered in `spec/tags.json`

---

### 5. Automating Validation Without Changing Specifications → **Coverage Tracking**

**SBE Process**: Tests execute specifications without modifying them

**fspec Implementation**:
- **Coverage files**: `*.feature.coverage` link scenarios to tests and implementation
- **Test-first workflow**: ACDD enforces writing tests BEFORE code
- **Traceability**: `fspec link-coverage` maintains scenario → test → implementation mappings

```bash
# After writing test (testing phase)
fspec link-coverage user-authentication \
  --scenario "Login with valid credentials" \
  --test-file src/__tests__/auth.test.ts \
  --test-lines 45-62

# After implementing code (implementing phase)
fspec link-coverage user-authentication \
  --scenario "Login with valid credentials" \
  --test-file src/__tests__/auth.test.ts \
  --impl-file src/auth/login.ts \
  --impl-lines 10-24
```

**The specifications (Gherkin) remain unchanged while tests execute them.**

**Artifacts**:
- `*.feature.coverage` files track scenario-to-test-to-implementation mappings
- Tests in `src/__tests__/*.test.ts`
- Implementation in `src/**/*.ts`

---

### 6. Executable Specification → **Test-Driven Development**

**SBE Process**: Specifications become executable tests

**fspec Implementation**:
- **ACDD workflow**: Forces `testing` phase BEFORE `implementing` phase
- **Red-Green-Refactor**: Tests must fail (red), then pass (green)
- **Temporal validation**: Prevents retroactive test creation (FEAT-011)

```bash
# Move to testing phase
fspec update-work-unit-status AUTH-001 testing

# Write tests (MUST fail - red phase)
# Create: src/__tests__/auth.test.ts
npm test  # Tests FAIL

# Move to implementing phase
fspec update-work-unit-status AUTH-001 implementing

# Write minimal code to make tests pass
# Edit: src/auth/login.ts
npm test  # Tests PASS (green phase)
```

**The Gherkin scenarios directly inform test structure and assertions.**

**Test File Example**:
```typescript
/**
 * Feature: spec/features/user-authentication.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

describe('Feature: User Authentication', () => {
  describe('Scenario: Login with valid credentials', () => {
    it('should redirect to dashboard and provide session token', async () => {
      // Given I am on the login page
      const loginPage = await navigateToLogin();

      // And I have a registered account
      const user = await createTestUser();

      // When I enter my valid email and password
      await loginPage.enterCredentials(user.email, user.password);
      await loginPage.submit();

      // Then I should be redirected to my dashboard
      expect(await getCurrentUrl()).toBe('/dashboard');

      // And I should receive a session token
      expect(await getSessionToken()).toBeDefined();
    });
  });
});
```

---

### 7. Validating Frequently → **Lifecycle Hooks**

**SBE Process**: Run specifications continuously to catch regressions

**fspec Implementation**:
- **Global hooks**: `spec/fspec-hooks.json` runs quality gates automatically
- **Virtual hooks**: Work unit-scoped hooks for temporary checks
- **Pre/post hooks**: Execute at command lifecycle events

```bash
# Add global hook to run tests before implementing
fspec add-hook pre-implementing test \
  --command "npm test" \
  --blocking

# Add virtual hook for work unit-specific checks
fspec add-virtual-hook AUTH-001 pre-validating "eslint" \
  --git-context \
  --blocking

# Hooks run automatically during workflow transitions
fspec update-work-unit-status AUTH-001 implementing
# → pre-implementing hook runs npm test
```

**Artifacts**:
- `spec/fspec-hooks.json` - Global lifecycle hooks
- `spec/hooks/.virtual/*.sh` - Auto-generated git context scripts
- Virtual hooks stored in work unit metadata

---

### 8. Living Documentation → **Validated Gherkin + Coverage**

**SBE Process**: Specifications evolve with codebase, always up-to-date

**fspec Implementation**:
- **Format on save**: `fspec format` keeps specs consistent
- **Tag validation**: `fspec validate-tags` ensures tag registry compliance
- **Coverage audit**: `fspec audit-coverage` verifies test/impl links are valid
- **Auto-generated docs**: `FOUNDATION.md` and `TAGS.md` generated from JSON

```bash
# Validate everything stays in sync
fspec check  # Runs validate + validate-tags + other checks

# Audit coverage to ensure links are valid
fspec audit-coverage user-authentication

# Show coverage to see traceability
fspec show-coverage user-authentication
```

**Example Coverage Report**:
```
Coverage Report: user-authentication

✅ Login with valid credentials (FULLY COVERED)
   Test: src/__tests__/auth.test.ts:45-62
   Implementation: src/auth/login.ts:10-24

⚠️  Login with expired session (PARTIALLY COVERED)
   Test: src/__tests__/auth.test.ts:64-78
   Implementation: (not linked)

❌ Account lock after failed attempts (NOT COVERED)
   No test mapping found

Coverage: 67% (2/3 scenarios)
```

**Feature files + coverage files = living documentation system.**

---

### 9. Evolving a Documentation System → **JSON-Backed Architecture**

**SBE Process**: Documentation system that grows with the project

**fspec Implementation**:
- **JSON storage**: `foundation.json`, `tags.json`, `work-units.json` are queryable
- **Markdown generation**: Human-readable docs auto-generated from JSON
- **Version control**: All specs and metadata tracked in git
- **Persistent state**: AI agents have queryable project state across sessions

```bash
# Query system for insights
fspec query-metrics --format=json
fspec query-work-units --status=implementing
fspec query-dependency-stats

# Generate human-readable documentation
fspec generate-foundation-md  # Creates FOUNDATION.md from foundation.json
fspec generate-tags-md         # Creates TAGS.md from tags.json

# Export for external tools
fspec export-work-units --format=csv --output=backlog.csv
fspec export-dependencies --format=mermaid --output=deps.mmd
```

**Artifacts**:
- `spec/foundation.json` → `spec/FOUNDATION.md`
- `spec/tags.json` → `spec/TAGS.md`
- `spec/work-units.json` (queryable state)
- `spec/epics.json`, `spec/prefixes.json`

---

## Just-in-Time Workflow Enforcement

The book emphasizes **just-in-time specification** - don't specify everything upfront.

**fspec enforces this through ACDD workflow states:**

```mermaid
stateDiagram-v2
    [*] --> backlog: Create work unit
    backlog --> specifying: Ready to specify
    specifying --> testing: Specs complete
    testing --> implementing: Tests written
    implementing --> validating: Tests pass
    validating --> done: Quality checks pass

    note right of backlog
        Work exists but NOT specified
        (Just-in-time: wait until ready)
    end note

    note right of specifying
        Specify ONLY when ready
        Example Mapping happens here
    end note

    note right of testing
        Write tests when ready to implement
        (Not before specs are done)
    end note
```

**The workflow prevents:**
- ❌ Specifying all features upfront (Big Design Up Front)
- ❌ Writing tests before specifications are clear
- ❌ Implementing before tests exist
- ❌ Skipping validation before marking done

**The workflow enforces:**
- ✅ Just-in-time specification (move to specifying when ready)
- ✅ Example Mapping before scenarios (collaborative discovery)
- ✅ Tests before code (TDD discipline)
- ✅ Quality gates before completion (validation phase)

---

## How fspec Extends SBE

While SBE is primarily a methodology, fspec adds **enforcement and tooling** for AI agents:

### 1. Workflow Enforcement

**Problem**: SBE relies on team discipline to follow the process

**fspec Solution**:
- **State machine**: Work units can only move through valid transitions
- **Temporal validation**: Prevents retroactive completion (FEAT-011)
- **Prefill detection**: Blocks workflow if feature files have placeholders
- **Estimation validation**: Can't estimate without completed feature file

```bash
# This will FAIL if feature file was created before entering specifying state
fspec update-work-unit-status AUTH-001 testing
# Error: ACDD temporal ordering violation detected!
```

### 2. AI Agent Support

**Problem**: SBE book assumes human team collaboration

**fspec Solution**:
- **System-reminders**: Wrapped in `<system-reminder>` tags, visible to AI but not users
- **Guided workflows**: Step-by-step prompts for discovery, estimation, etc.
- **Persistent state**: AI can query project state across sessions
- **Queryable artifacts**: JSON format enables programmatic access

```bash
# AI receives system-reminder when estimate is missing
fspec show-work-unit AUTH-001
# <system-reminder>
# Work unit AUTH-001 has no estimate.
# After generating scenarios, estimate based on complexity.
# Run: fspec update-work-unit-estimate AUTH-001 <points>
# </system-reminder>
```

### 3. Dependency Management

**Problem**: SBE doesn't address work sequencing or blockers

**fspec Solution**:
- **Dependencies**: Work units can depend on, block, or relate to each other
- **Dependency graphs**: Visualize work relationships
- **Auto-advance**: Automatically move work when dependencies are met
- **Blocked state**: Explicit state for work that can't proceed

```bash
# Create dependency relationships
fspec add-dependency UI-001 --blocked-by=API-001
fspec add-dependency DASH-001 --depends-on=AUTH-001

# Visualize dependencies
fspec export-dependencies --format=mermaid
```

### 4. Checkpoint System

**Problem**: SBE doesn't provide rollback mechanisms

**fspec Solution**:
- **Git-based checkpoints**: Automatic checkpoints on state transitions
- **Manual checkpoints**: Create named save points for experiments
- **Safe restoration**: Restore with conflict detection
- **AI-guided resolution**: System-reminders for conflict resolution

```bash
# Create checkpoint before experiment
fspec checkpoint AUTH-001 baseline

# Try approach A... doesn't work

# Restore checkpoint
fspec restore-checkpoint AUTH-001 baseline

# Try approach B... works!
```

### 5. Reverse ACDD

**Problem**: SBE assumes greenfield projects, not legacy codebases

**fspec Solution**:
- **Reverse engineering**: Apply SBE to existing codebases without specs
- **Gap detection**: Identify missing features, tests, or coverage
- **Strategic planning**: AI-guided approach selection
- **Incremental adoption**: Fill gaps one feature at a time

```bash
# Analyze existing codebase and detect gaps
fspec reverse

# Choose strategy (Spec Gap, Test Gap, Coverage Mapping, Full Reverse)
fspec reverse --strategy=A

# Continue step-by-step through workflow
fspec reverse --continue
```

### 6. Virtual Hooks

**Problem**: SBE doesn't provide work unit-specific quality gates

**fspec Solution**:
- **Ephemeral hooks**: Temporary quality checks scoped to single work units
- **Git context scripts**: Auto-generated hooks that process changed files
- **Cleanup on completion**: Remove hooks when work is done

```bash
# Add work unit-specific linting
fspec add-virtual-hook AUTH-001 pre-validating "eslint" \
  --git-context \
  --blocking

# Cleared automatically when work reaches done
fspec clear-virtual-hooks AUTH-001
```

### 7. Coverage Tracking

**Problem**: SBE doesn't define how to link specs, tests, and implementation

**fspec Solution**:
- **Three-tier linking**: Scenario → Test (file + lines) → Implementation (file + lines)
- **Gap detection**: Identify uncovered scenarios or untested code
- **Audit capabilities**: Verify all links are valid
- **Project-wide reporting**: See coverage across all features

```bash
# Show coverage gaps
fspec show-coverage

# Audit to find broken links
fspec audit-coverage user-authentication --fix
```

---

## Historical Impact & Importance

### Publication Context (2011)

When published in 2011, "Specification by Example" synthesized several emerging practices:
- **Behavior-Driven Development (BDD)** - Dan North's work
- **Acceptance Test-Driven Development (ATDD)** - Ward Cunningham, Kent Beck
- **Example-Driven Development** - Various practitioners
- **Executable Specifications** - Fit/FitNesse community

Adzic didn't invent these concepts, but he **codified them into a coherent methodology** backed by research from 50+ teams across different industries.

### Research-Backed Methodology

Unlike many programming books based on opinion, SBE is grounded in:
- **Case studies from real teams**: Google, Sabre, RainStor, uSwitch, etc.
- **Quantifiable benefits**: 50-90% reduction in rework, 30-50% faster delivery
- **Cross-industry validation**: Finance, healthcare, e-commerce, embedded systems

This empirical approach gave it credibility beyond "yet another agile practice."

### Influenced Tool Ecosystems

The book directly or indirectly influenced:
- **Cucumber** (Ruby, JS, Java) - Gherkin became the de facto DSL
- **SpecFlow** (.NET) - BDD framework explicitly following SBE
- **Concordion** (Java) - Living documentation approach
- **Behave** (Python) - Gherkin-based testing
- **Gauge** (Multi-language) - Markdown-based specifications

The "Given-When-Then" format is now **ubiquitous** in testing frameworks.

### Modern Adoption

SBE principles are now considered **best practice** in organizations like:
- **ThoughtWorks** (Technology Radar endorses BDD/SBE)
- **Spotify, Netflix, Amazon** (documentation-as-code cultures)
- **Government Digital Service (GDS)** in UK - mandated for public projects
- **Financial institutions** (regulatory compliance through executable specs)

### Influence Score: **8.5/10** in Modern Software Development

**Why Not 10/10?**
- Not as widely read as "Clean Code" or "The Pragmatic Programmer"
- Primarily influential in **Agile/BDD circles**, less in other paradigms
- Some teams found the approach too heavyweight for simple projects

**Why Not Lower?**
- **Directly shaped modern testing practices** (Given-When-Then everywhere)
- **Core text for BDD/ATDD practitioners** (referenced constantly)
- **Influenced tool design** for a decade (2011-2025)
- **Still relevant** - principles hold up in 2025 with AI coding agents

### Comparable Books in Influence

| Book | Author | Impact Area | Influence |
|------|--------|-------------|-----------|
| **Specification by Example** | Gojko Adzic | Requirements, BDD, Testing | 8.5/10 |
| The Pragmatic Programmer | Hunt & Thomas | General best practices | 9.5/10 |
| Clean Code | Robert Martin | Code quality | 9/10 |
| Domain-Driven Design | Eric Evans | Architecture | 8/10 |
| Continuous Delivery | Humble & Farley | DevOps, CI/CD | 9/10 |
| Test-Driven Development | Kent Beck | Testing discipline | 8.5/10 |

It's in the **top tier** for requirements engineering and testing methodology.

---

## Why SBE Matters for AI Agents

### The Evolution Timeline

1. **2011 (SBE book)**: Human teams need better collaboration on requirements
2. **2015-2020**: BDD/ATDD tools mature but require discipline
3. **2023-2025**: AI coding agents emerge but lack structure
4. **fspec's role**: Bring SBE rigor to AI-human collaboration

### The "AI Gap" That fspec Addresses

The SBE book was written for **human teams collaborating**, not AI agents. Key gaps:

| SBE Assumption | AI Reality | fspec Solution |
|----------------|------------|----------------|
| Team discipline enforces workflow | AI may skip steps | State machine + temporal validation |
| Humans remember context | AI loses context between sessions | Persistent queryable state (JSON) |
| Shared understanding implicit | AI needs explicit guidance | System-reminders + guided workflows |
| Process followed voluntarily | AI needs enforcement | Workflow blocking + prefill detection |
| Documentation kept in sync manually | AI needs automation | Auto-generate markdown from JSON |

### Why AI Agents Need SBE

**The bottleneck shifts from "writing code" to "knowing what to build":**

| Without SBE | With SBE (via fspec) |
|-------------|----------------------|
| AI jumps straight to code | AI starts with Example Mapping |
| No shared understanding | Collaborative discovery with human |
| Vague requirements | Concrete examples |
| Tests written after code (if at all) | Tests written before code (enforced) |
| Documentation gets stale | Living documentation stays current |
| Context lost between sessions | Persistent queryable state |
| AI guesses at edge cases | AI explores scenarios through examples |

### fspec Makes SBE Enforceable for AI

**The book described the methodology. fspec enforces it.**

- **Persistent state**: Work units, examples, rules, questions all queryable
- **Workflow validation**: Can't skip phases or cheat the sequence
- **Temporal enforcement**: Can't create tests/code then backdate specifications
- **System-reminders**: Guide AI through proper SBE workflow
- **Coverage tracking**: Maintain traceability from scenario to implementation
- **Quality gates**: Hooks ensure specifications stay aligned with code

---

## Summary: The Complete Mapping

| SBE Stage | SBE Practice | fspec Feature | fspec Command |
|-----------|--------------|---------------|---------------|
| 1. Derive scope | Business goals → user stories | Foundation + Work units + Epics | `fspec discover-foundation`<br/>`fspec create-work-unit` |
| 2. Illustrate | Concrete examples | Example Mapping | `fspec add-rule`<br/>`fspec add-example`<br/>`fspec add-question` |
| 3. Specify | Transform to Given-When-Then | Generate scenarios | `fspec generate-scenarios` |
| 4. Refine | Structure specifications | Gherkin feature files | `fspec validate`<br/>`fspec format` |
| 5. Automate | Tests execute specs | Coverage tracking | `fspec link-coverage` |
| 6. Execute | Specs become tests | Test-driven workflow | `fspec update-work-unit-status testing` |
| 7. Validate | Run continuously | Lifecycle hooks | `fspec add-hook`<br/>`fspec add-virtual-hook` |
| 8. Live docs | Evolve with code | Validated Gherkin | `fspec check`<br/>`fspec audit-coverage` |
| 9. Evolve | Growing documentation | JSON-backed architecture | `fspec query-*`<br/>`fspec generate-*-md` |

---

## Key Takeaway

**In essence**: If Gojko Adzic's book describes the "what" and "why" of Specification by Example, **fspec is the "how" for AI coding agents.**

In a world where AI can write code faster than ever, **the bottleneck shifts to knowing WHAT to build.** SBE/Example Mapping solves that. fspec makes it real.

---

## References

- **Book**: "Specification by Example: How Successful Teams Deliver the Right Software" by Gojko Adzic (2011)
- **Example Mapping**: Introduced by Matt Wynne (Cucumber Ltd) as a lightweight SBE practice
- **fspec Documentation**:
  - `spec/FOUNDATION.md` - Project foundation and architecture
  - `spec/CLAUDE.md` - AI agent workflow guide
  - `spec/TAGS.md` - Tag registry
  - `README.md` - Getting started guide

---

**Last Updated**: 2025-10-23
**Version**: 1.0
**Author**: Generated by Claude Code analyzing fspec project and SBE methodology
