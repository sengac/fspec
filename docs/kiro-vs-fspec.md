# Kiro vs fspec: The Ultimate Spec-Driven Development Showdown

> **TL;DR:** Both automate spec-to-code workflows, but Kiro is AWS's proprietary AI IDE while fspec is an open-source CLI that enforces TRUE test-driven development with ANY AI tool. If you value freedom, TDD discipline, and iterative requirements discovery, fspec wins.

---

## Table of Contents

1. [What Are These Tools?](#what-are-these-tools)
2. [Quick Comparison Table](#quick-comparison-table)
3. [The Philosophy Difference](#the-philosophy-difference)
4. [Deep Dive: 7 Key Differentiators](#deep-dive-7-key-differentiators)
5. [The Killer Feature: Iterative Requirements Discovery](#the-killer-feature-iterative-requirements-discovery)
6. [When to Use Each](#when-to-use-each)
7. [The Bottom Line](#the-bottom-line)

---

## What Are These Tools?

### Kiro (Amazon's AI IDE)

**Released:** July 14, 2025 (Preview)
**What it is:** An integrated development environment built on VS Code that uses AI agents (Claude Sonnet 4) to generate code from specifications.

**The Pitch:** "Write specs in plain English, we'll generate production-ready code with autonomous AI agents."

### fspec (Open Source CLI)

**Released:** 2025
**What it is:** A command-line tool that enforces Acceptance Criteria Driven Development (ACDD) and Test-Driven Development (TDD) workflows with ANY AI coding assistant.

**The Pitch:** "AI can't cheat - we enforce proper TDD discipline so your specs, tests, and code stay aligned."

---

## Quick Comparison Table

| Feature | Kiro | fspec |
|---------|------|-------|
| **Type** | Proprietary AI IDE | Open Source CLI Tool |
| **Platform** | AWS Cloud / VS Code Fork | Local, Any Editor |
| **License** | Commercial (likely paid) | MIT (Free Forever) |
| **AI Integration** | Built-in (Claude Sonnet 4) | Bring Your Own (Claude Code, Cursor, Copilot, etc.) |
| **Interface** | GUI-based IDE | Terminal + TUI |
| **Code Generation** | ✅ Yes (autonomous agents) | ✅ Yes (via AI assistant) |
| **Test Generation** | ✅ Yes (auto-generated) | ✅ Yes (enforced BEFORE code) |
| **TDD Enforcement** | ❌ No | ✅ **YES** (temporal validation) |
| **Spec Format** | EARS notation, custom | **Gherkin/Cucumber (BDD standard)** |
| **Requirements Discovery** | One-directional (spec → code) | **Iterative (spec ⇄ test ⇄ code)** |
| **Tool Lock-in** | AWS IDE only | Works with VS Code, Vim, Cursor, etc. |
| **Workflow Enforcement** | Soft (AI-guided) | **Hard (blocking validation)** |
| **Temporal Validation** | ❌ No | ✅ **Prevents retroactive work** |
| **Reverse Engineering** | ❌ No | ✅ **Reverse ACDD** (specs from code) |
| **Coverage Tracking** | Spec-to-code traceability | **Spec → Test → Impl (3-tier linking)** |
| **Git Integration** | Basic | **Deep** (auto-checkpoints, hooks) |
| **Project Management** | Limited | **Full Kanban** (work units, epics) |
| **Ecosystem** | Closed (AWS only) | Open (Cucumber, BDD tools) |

---

## The Philosophy Difference

### Kiro's Philosophy: **"AI Does It For You"**

```
Human Intent → AI Generates Specs → AI Generates Code → Done
```

**Strengths:**
- Fast prototyping
- Minimal manual work
- Visual, user-friendly

**Risks:**
- AI might hallucinate requirements
- No guarantee of proper TDD
- Specs might drift from implementation
- Vendor lock-in

### fspec's Philosophy: **"AI Can't Cheat The Workflow"**

```
Specs (Gherkin) → Tests (RED) → Code (GREEN) → Refactor
         ↑           ↓
         └── Discover Missing Requirements ──┘
```

**Strengths:**
- Enforces proper TDD discipline
- Prevents AI from skipping steps
- Iterative requirements discovery
- Tool-agnostic freedom

**Trade-offs:**
- More CLI learning curve
- Requires discipline (but that's the point!)

---

## Deep Dive: 7 Key Differentiators

### 1. **TDD Enforcement: The Game Changer**

#### Kiro: Test-Aware, Not Test-Driven

- Generates tests automatically
- No enforcement that tests come BEFORE code
- AI can write code first, tests later (fake TDD)
- No RED phase validation

**Problem:** AI agents might generate "passing tests" that don't actually test anything meaningful.

#### fspec: TRUE Test-Driven Development

```bash
# 1. Write specs
fspec add-scenario auth "Login with valid credentials"

# 2. Move to testing (auto-checkpoint created)
fspec update-work-unit-status AUTH-001 testing

# 3. Write FAILING tests (RED phase - REQUIRED)
npm test  # Must fail or you can't proceed

# 4. Move to implementing (temporal validation kicks in)
fspec update-work-unit-status AUTH-001 implementing
# ⚠️  BLOCKED if tests were created AFTER this timestamp

# 5. Write minimal code to pass tests (GREEN phase)
npm test  # Must pass

# 6. Refactor while keeping tests green
```

**The Power:** fspec uses **file modification timestamps** to BLOCK you if tests were created after entering implementing phase. This prevents AI from:
- Writing code first, tests later
- Theater commits (pretending to do TDD)
- Retroactive specification

**Kiro can't do this.** It has no temporal validation.

---

### 2. **Open vs Closed: Freedom vs Lock-In**

#### Kiro: AWS Ecosystem Lock-In

- **Platform:** AWS-hosted IDE only
- **AI Model:** Claude Sonnet 4 (locked to Anthropic via AWS)
- **Pricing:** Likely commercial (AWS services aren't free)
- **Customization:** Limited to what AWS allows
- **Data:** Hosted on AWS (compliance considerations)

**Analogy:** Buying an iPhone - beautiful, integrated, but you're stuck in Apple's ecosystem.

#### fspec: Tool-Agnostic Freedom

- **Platform:** Runs anywhere (macOS, Linux, Windows)
- **Editor:** VS Code, Vim, Neovim, Cursor, Zed, etc.
- **AI:** Claude Code, Cursor, GitHub Copilot, Windsurf, any future tool
- **Pricing:** MIT licensed - **free forever**
- **Customization:** Open source - modify anything
- **Data:** Local-first - your code never leaves your machine

**Analogy:** Android - use any phone, any app, customize everything.

**Practical Benefit:** If Claude Code releases a killer feature tomorrow, you can use it with fspec. If Kiro's AI gets worse or AWS raises prices, you're stuck.

---

### 3. **Iterative Requirements Discovery: The Hidden Superpower**

This is where fspec shines in a way Kiro fundamentally can't.

#### The Problem with One-Directional Workflows (Kiro)

```
Prompt → Specs → Design Doc → Code
```

**What happens when:**
- Tests reveal edge cases you didn't think of?
- Implementation exposes unclear requirements?
- You realize specs were incomplete?

**In Kiro:** You manually update specs, hope AI regenerates correctly.

#### The fspec Solution: Bidirectional Discovery

```bash
# Start with what you THINK you need
fspec add-rule AUTH-001 "Users must be authenticated"
fspec generate-scenarios AUTH-001

# Move to testing
fspec update-work-unit-status AUTH-001 testing

# Write test - reveals gap
it('should handle expired tokens', () => {
  // Wait... the spec doesn't say what to do with expired tokens!
})

# Move BACKWARD to specifying
fspec update-work-unit-status AUTH-001 specifying

# Add discovered requirement
fspec add-rule AUTH-001 "Expired tokens must return 401 Unauthorized"
fspec add-scenario auth "Login with expired token returns 401"

# Continue TDD cycle
fspec update-work-unit-status AUTH-001 testing
```

**The Magic:** fspec allows **backward movement through Kanban states**:
- `testing → specifying` - Tests revealed incomplete specs
- `implementing → testing` - Need more test coverage
- `validating → implementing` - Quality checks failed

**Why This Matters:**
- ✅ Requirements emerge through TDD
- ✅ Specs evolve based on implementation learnings
- ✅ Edge cases discovered iteratively
- ✅ No pretending specs were perfect upfront

**Kiro's approach:** Assumes specs are complete upfront (waterfall mindset).
**fspec's approach:** Specs and tests co-evolve (agile mindset).

---

### 4. **Specification Format: Proprietary vs Standard**

#### Kiro: EARS Notation (Custom)

- **Format:** Amazon's EARS (Easy Approach to Requirements Syntax)
- **Ecosystem:** Kiro-specific, limited tooling
- **Portability:** Locked to Kiro
- **Learning Curve:** Learn Kiro's way

#### fspec: Gherkin/Cucumber (BDD Standard)

- **Format:** Gherkin - industry standard since 2008
- **Ecosystem:** Cucumber, SpecFlow, Behave, Behat, pytest-bdd, etc.
- **Portability:** Works with 100+ BDD tools across 30+ languages
- **Learning Curve:** Industry-standard skill (transferable)

**Example:**

```gherkin
# fspec (Gherkin) - readable by anyone
Scenario: Login with valid credentials
  Given I am on the login page
  When I enter valid username and password
  And I click the login button
  Then I should be redirected to the dashboard
  And I should see a welcome message

# Kiro (EARS) - Amazon's custom format
WHEN the user is on the login page
IF the user enters valid username and password
THEN the system shall redirect to dashboard
AND the system shall display welcome message
```

**Benefit:** Gherkin specs written in fspec can be used by:
- QA teams (Cucumber test automation)
- Documentation tools (living docs)
- Reporting tools (test reports)
- Other BDD frameworks

**Kiro specs:** Only work in Kiro.

---

### 5. **Coverage Tracking: Compliance & Auditability**

Both tools offer traceability, but fspec's is more granular.

#### Kiro: Spec-to-Code Traceability

- Links specifications to generated code
- Useful for compliance (knowing what code implements what requirement)

#### fspec: Three-Tier Traceability (Spec → Test → Implementation)

```json
{
  "scenario": "Login with valid credentials",
  "testMapping": {
    "file": "src/__tests__/auth.test.ts",
    "lines": "45-62"
  },
  "implementationMapping": {
    "file": "src/auth/login.ts",
    "lines": "10-24"
  }
}
```

**Why This Matters:**

1. **Compliance:** Know exactly which tests validate which acceptance criteria
2. **Gap Detection:** See which scenarios have no tests (coverage holes)
3. **Refactoring Safety:** Understand impact of changing code
4. **Code Review:** Reviewers can trace from requirement → test → code

**Commands:**

```bash
# Show coverage status
fspec show-coverage user-authentication
# ✅ Login with valid credentials (FULLY COVERED)
#    Test: src/__tests__/auth.test.ts:45-62
#    Impl: src/auth/login.ts:10-24
# ⚠️  Password reset (PARTIALLY COVERED)
#    Test: src/__tests__/auth.test.ts:80-95
#    Impl: ⚠️  No implementation mappings

# Audit coverage (verify files still exist)
fspec audit-coverage user-authentication
# ✅ All files found (4/4)
```

---

### 6. **AI Agent Control: Autonomous vs Enforced**

#### Kiro: Autonomous AI Agents

- **Agent Hooks:** Auto-update tests when files change
- **Background Automation:** AI works while you code
- **Convenience:** Less manual work

**Risk:** AI can make changes you didn't explicitly approve.

#### fspec: Human-in-the-Loop with Hard Validation

- **Lifecycle Hooks:** Run scripts at specific workflow events
- **Blocking Validation:** Can prevent state transitions if checks fail
- **Virtual Hooks:** Work unit-specific quality gates

**Example:**

```bash
# Add blocking quality check before implementing
fspec add-virtual-hook AUTH-001 \
  --event pre-implementing \
  --command "npm run lint && npm run type-check" \
  --blocking

# Try to move to implementing with lint errors
fspec update-work-unit-status AUTH-001 implementing
# ❌ BLOCKED: Linting failed (12 errors)
# Fix errors before proceeding
```

**Philosophy:**
- **Kiro:** "Trust the AI to make smart changes"
- **fspec:** "Validate everything, block if quality gates fail"

---

### 7. **Reverse ACDD: Legacy Code Support**

What if you have existing code without specs?

#### Kiro: Not Designed for Reverse Engineering

- Built for greenfield projects (new code)
- No documented reverse engineering workflow

#### fspec: Reverse ACDD Built-In

```bash
# Start reverse ACDD
fspec reverse

# AI analyzes codebase and discovers:
# - User-facing interactions (routes, CLI commands, API endpoints)
# - Business logic patterns
# - Existing test coverage
# - Missing specifications

# Generates:
# ✅ Feature files with inferred acceptance criteria
# ✅ Skeleton test files (structure, not implementation)
# ✅ Coverage mappings linking existing code to specs
# ✅ User story maps

# Result: Legacy code now has living documentation
```

**Use Cases:**
- Inherited legacy codebase
- Transitioning to BDD
- Documentation for compliance
- Onboarding new developers

---

## The Killer Feature: Iterative Requirements Discovery

Let's walk through a real scenario that shows fspec's unique advantage.

### Scenario: Building an Authentication System

#### The Kiro Way (One-Directional)

```
1. Prompt: "Build authentication with email/password"
2. Kiro generates:
   - User stories
   - Technical design
   - Code (login, signup, session management)
   - Tests

3. You realize: "Wait, what about 2FA? Social login? Rate limiting?"
4. Manually update specs, regenerate code, hope it's correct
```

**Problem:** Requirements were incomplete upfront. You're forced to regenerate or manually fix.

#### The fspec Way (Iterative Discovery)

```bash
# Iteration 1: Start simple
fspec add-rule AUTH-001 "Users can login with email and password"
fspec generate-scenarios AUTH-001
fspec update-work-unit-status AUTH-001 testing

# Write tests - discover edge case
it('should prevent login after 5 failed attempts', () => {
  // Hmm, spec doesn't mention rate limiting...
})

# Move backward, add discovered requirement
fspec update-work-unit-status AUTH-001 specifying
fspec add-rule AUTH-001 "Account locked after 5 failed login attempts"
fspec add-scenario auth "Account lockout after failed attempts"

# Iteration 2: Implement, discover another gap
fspec update-work-unit-status AUTH-001 testing
fspec update-work-unit-status AUTH-001 implementing
# While implementing, realize: "We need password reset!"

# Move backward again
fspec update-work-unit-status AUTH-001 specifying
fspec add-scenario auth "User requests password reset"

# Iteration 3: Tests reveal session management gap
# Move backward, add session timeout scenarios

# Final result: Complete specs that evolved through TDD
```

**Why This Works:**

1. **Tests expose gaps** - Writing tests forces you to think through edge cases
2. **Implementation reveals complexity** - Actually building it shows what specs missed
3. **No fake completeness** - You don't pretend specs were perfect upfront
4. **Organic discovery** - Requirements emerge naturally through the process

**Kiro can't do this** because:
- No backward movement through workflow
- No temporal validation
- No TDD enforcement

---

## When to Use Each

### Choose Kiro If You:

✅ **Work in AWS ecosystem** - Already using AWS services
✅ **Want turnkey solution** - Don't want to configure anything
✅ **Prefer GUI over CLI** - Like visual workflows, diagrams
✅ **Have enterprise budget** - Can afford commercial pricing
✅ **Need team collaboration** - Built-in multi-user features
✅ **Trust autonomous AI** - Comfortable with AI making decisions
✅ **Greenfield projects only** - Building new code from scratch

### Choose fspec If You:

✅ **Value open source** - Want to inspect/modify/contribute
✅ **Want tool freedom** - Use any editor, any AI assistant
✅ **Need TDD discipline** - Want enforced test-driven development
✅ **Prefer local-first** - Don't want cloud dependencies
✅ **Have legacy code** - Need reverse engineering (Reverse ACDD)
✅ **Want iterative discovery** - Requirements evolve through TDD
✅ **Need BDD standards** - Gherkin/Cucumber ecosystem compatibility
✅ **Command-line power user** - Prefer terminal workflows
✅ **Budget-conscious** - Free forever vs commercial pricing
✅ **Want hard validation** - Blocking quality gates, temporal checks

---

## The Bottom Line

### What They Have in Common

- ✅ Spec-to-code automation
- ✅ AI-powered code generation
- ✅ Traceability (spec → implementation)
- ✅ Test generation
- ✅ Reduced manual work

### The Critical Differences

| Kiro | fspec |
|------|-------|
| Proprietary AI IDE | Open source CLI |
| AWS lock-in | Tool-agnostic |
| Test generation | **TDD enforcement** |
| One-directional workflow | **Iterative discovery** |
| EARS notation | Gherkin/BDD standard |
| Autonomous AI agents | **Human-validated workflows** |
| Greenfield only | **Reverse ACDD for legacy** |
| Soft guidance | **Hard blocking validation** |
| Commercial (likely) | **Free (MIT)** |

### The Honest Take

**Kiro is impressive** - AWS built a polished, integrated AI IDE that makes spec-driven development accessible to teams who want everything in one box.

**But fspec is fundamentally different** - it's not just an AI IDE, it's a **workflow enforcement system** that:

1. **Prevents AI from cheating** (temporal validation)
2. **Enforces TRUE TDD** (tests before code, always)
3. **Enables iterative discovery** (backward movement through states)
4. **Works with any tool** (VS Code, Vim, Cursor, etc.)
5. **Speaks BDD standard** (Gherkin, not proprietary format)
6. **Costs nothing** (MIT license, no vendor lock-in)

### The Real Question

Do you want an AI that does everything FOR you (Kiro)?

Or do you want a system that ensures AI does things THE RIGHT WAY (fspec)?

---

## Call to Action

### Try fspec Today

```bash
# Install
npm install -g @sengac/fspec

# Initialize your project
fspec init

# Start your first work unit
fspec create-story AUTH "User Authentication"
fspec update-work-unit-status AUTH-001 specifying
fspec add-rule AUTH-001 "Users can login with email/password"
fspec generate-scenarios AUTH-001

# Experience enforced TDD
fspec update-work-unit-status AUTH-001 testing
# Write tests, verify RED phase
fspec update-work-unit-status AUTH-001 implementing
# Try to implement without tests - get BLOCKED 🚫
```

### Learn More

- **GitHub:** [github.com/sengac/fspec](https://github.com/sengac/fspec)
- **Docs:** [docs.fspec.dev](https://docs.fspec.dev)
- **Community:** [discord.gg/fspec](https://discord.gg/fspec)

---

## Appendix: Feature Comparison Matrix

### Workflow & Project Management

| Feature | Kiro | fspec |
|---------|------|-------|
| Kanban board | ❌ | ✅ Full (backlog → done) |
| Work unit management | Limited | ✅ Stories, bugs, tasks |
| Epic grouping | ❌ | ✅ Yes |
| State transitions | Linear | ✅ Bidirectional |
| Temporal validation | ❌ | ✅ Prevents retroactive work |
| Auto-checkpoints | ❌ | ✅ Git stash on transitions |
| Backward movement | ❌ | ✅ testing → specifying allowed |

### Requirements & Specifications

| Feature | Kiro | fspec |
|---------|------|-------|
| Spec format | EARS (proprietary) | Gherkin (BDD standard) |
| Example Mapping | ❌ | ✅ Built-in |
| Scenario generation | ✅ AI-powered | ✅ From example map |
| Architecture notes | ✅ Design docs | ✅ In feature files |
| Tag management | ❌ | ✅ Registry + validation |
| Reverse engineering | ❌ | ✅ Reverse ACDD |

### Testing & TDD

| Feature | Kiro | fspec |
|---------|------|-------|
| Test generation | ✅ Auto-generated | ✅ Skeleton + @step comments |
| TDD enforcement | ❌ | ✅ **Temporal validation** |
| RED phase validation | ❌ | ✅ Tests must fail first |
| Test-first blocking | ❌ | ✅ Can't implement without tests |
| @step comments | ❌ | ✅ **Mandatory** for all steps |
| Coverage tracking | Spec → Code | ✅ Spec → Test → Impl |

### Quality & Validation

| Feature | Kiro | fspec |
|---------|------|-------|
| Gherkin validation | N/A | ✅ Official parser |
| Tag validation | ❌ | ✅ Registry enforcement |
| Prefill detection | ❌ | ✅ Blocks placeholders |
| Lifecycle hooks | Agent hooks (auto) | ✅ Pre/post command hooks |
| Virtual hooks | ❌ | ✅ Work unit-scoped |
| Blocking validation | ❌ | ✅ Quality gates |
| Estimation validation | ❌ | ✅ Requires complete specs |

### Integration & Ecosystem

| Feature | Kiro | fspec |
|---------|------|-------|
| Editor support | VS Code fork only | **Any editor** |
| AI support | Claude Sonnet 4 | **Any AI assistant** |
| Git integration | Basic | ✅ Deep (hooks, checkpoints) |
| MCP support | ✅ Yes | ✅ Yes |
| BDD ecosystem | ❌ | ✅ Cucumber, Behave, etc. |
| Platform support | AWS Cloud | ✅ macOS, Linux, Windows |

### Research & Discovery

| Feature | Kiro | fspec |
|---------|------|-------|
| AST code analysis | ❌ | ✅ Tree-sitter based |
| Perplexity integration | ❌ | ✅ Research tool |
| Jira integration | ❌ | ✅ Research tool |
| Confluence integration | ❌ | ✅ Research tool |
| Stakeholder comms | ❌ | ✅ Teams/Slack/Discord |
| Auto-attach research | ❌ | ✅ With AI extraction |

### Cost & Licensing

| Feature | Kiro | fspec |
|---------|------|-------|
| License | Commercial (likely) | **MIT (Open Source)** |
| Pricing | AWS pricing model | **Free forever** |
| Vendor lock-in | AWS only | None |
| Self-hosted | ❌ | ✅ Local-first |
| Enterprise support | AWS | Community + paid support |

---

## Final Thought

**Kiro is what happens when Amazon builds an AI IDE.**

**fspec is what happens when developers who love TDD, BDD, and open source build a workflow enforcement system.**

Both are impressive. But only one gives you freedom, enforces discipline, and costs nothing.

**Choose wisely.**
