# Local Change Protocols (LCP) — Implementation Guide for fspec

## Table of Contents

1. [Origin & Motivation](#1-origin--motivation)
2. [Core Concept](#2-core-concept)
3. [How LCP Fits Into fspec's ACDD](#3-how-lcp-fits-into-fspecs-acdd)
4. [Data Model](#4-data-model)
5. [LCP Comment Format (Language-Agnostic)](#5-lcp-comment-format-language-agnostic)
6. [Risk Zones](#6-risk-zones)
7. [CLI Commands](#7-cli-commands)
8. [Validation & Enforcement](#8-validation--enforcement)
9. [Hook Integration](#9-hook-integration)
10. [System Reminder Integration](#10-system-reminder-integration)
11. [Story Breakdown](#11-story-breakdown)
12. [Academic References](#12-academic-references)
13. [Source Material from PDD Repository](#13-source-material-from-pdd-repository)

---

## 1. Origin & Motivation

### Protocol-Driven Development (PDD)

Local Change Protocols originate from the [Protocol-Driven Development](https://github.com/philspil66/Protocol-Driven-Development-PDD) framework by Phil Spilsbury. PDD's central axiom is:

> **"The AI can be probabilistic. The pipeline cannot."**

PDD introduces explicit engineering protocols that govern how software is specified, generated, and validated. The pipeline follows: `Protocol → AI → Code` rather than `Prompt → AI → Code`.

### The Problem LCP Solves

From the PDD patterns documentation:

> "Some code is dangerous to change. The risk is often undocumented and invisible."

In AI-assisted development, this problem is amplified:
- AI agents can modify any file without understanding its risk profile
- Critical business logic (payments, auth, encryption) can be silently altered
- Design constraints that "live in people's heads" are invisible to AI
- Generated code may compile and pass tests but violate architectural invariants
- Review bottlenecks form when humans can't validate changes at AI speed

### Why fspec Needs This

fspec already has:
- **Stage permissions** — blocks file writes by lifecycle phase
- **Virtual hooks** — per-work-unit quality gates
- **Global hooks** — project-wide pre/post transition checks
- **Watcher sessions** — supervisor agents (Security Reviewer, etc.)
- **@step comments** — mandatory traceability markers in test files

But fspec has **no mechanism to embed governance rules directly at the point of change** — inside the source files themselves. LCP fills this gap by making constraints visible exactly where they matter: in the code that must not be carelessly modified.

---

## 2. Core Concept

A Local Change Protocol is a **structured comment block** embedded in a source code file that declares:

1. **What area of concern this code covers** (scope)
2. **How risky changes to this code are** (risk level)
3. **What must NOT be changed** (constraints)
4. **What conditions require human review** (review triggers)
5. **What verification must pass after changes** (verification commands)

### Characteristics (from PDD)

- **Local to the file** — lives where the risk lives
- **Explicit about risk** — no hidden assumptions
- **Enforceable by tooling** — not just documentation
- **Understandable by humans and AI** — structured, parseable format

### PDD Template

```
LOCAL_CHANGE_PROTOCOL
scope: <area>
risk_level: <low | medium | high>

constraints:
  - do_not_change: <rule>

requires_human_review:
  - <condition>

verification:
  - <tests>
```

---

## 3. How LCP Fits Into fspec's ACDD

### Integration Points

```
Foundation Event Storm
    ↓
Bounded Contexts → Risk Zone Assignment
    ↓
Feature Files → Architecture docstrings reference LCP-protected files
    ↓
Implementation Phase → LCP comments added to high-risk source files
    ↓
Validation Phase → LCP validation checks constraints/verification
    ↓
Hook System → Workflow hooks + Rust pre_tool_use enforce LCP rules on changed files
    ↓
System Reminders → AI agents warned about LCP constraints before editing
```

### Lifecycle

1. **During specifying**: Risk zones are declared for the feature (via architecture notes or feature-level metadata)
2. **During implementing**: LCP blocks are added to source files as structured comments
3. **During validating**: `fspec validate-lcp` scans for LCP blocks, validates structure, and checks that verification commands pass
4. **Ongoing**: Pre-commit hooks and watcher sessions enforce LCP constraints on every change

### Relationship to Existing Systems

| Existing System | LCP Enhancement |
|---|---|
| **Stage permissions** | LCP adds file-level granularity (not just phase-level) |
| **Virtual hooks** | LCP constraints persist after work unit is done; virtual hooks are ephemeral |
| **Global hooks** | LCP-aware hooks only trigger for files with LCP blocks |
| **Watcher sessions** | Security Reviewer watcher can read LCP blocks and flag violations |
| **@step comments** | LCP follows the same "structured comment" pattern, but for governance instead of traceability |
| **Tag system** | Risk zones map to tags (`@risk-high`, `@risk-medium`, `@risk-low`) |
| **Coverage tracking** | LCP-protected files can be tracked in coverage as requiring additional verification |

---

## 4. Data Model

### LCP Registry (`spec/lcp-registry.json`)

Central registry of all LCP-protected files and risk zone assignments:

```json
{
  "version": "1.0.0",
  "riskZones": {
    "high": {
      "description": "LCP required, human review required",
      "paths": [
        "src/payments/**",
        "src/auth/**",
        "src/encryption/**"
      ],
      "tags": ["@risk-high"]
    },
    "medium": {
      "description": "LCP recommended",
      "paths": [
        "src/pricing/**",
        "src/order-flow/**"
      ],
      "tags": ["@risk-medium"]
    },
    "low": {
      "description": "Automated refactors allowed",
      "paths": [
        "src/ui/**",
        "src/logging/**"
      ],
      "tags": ["@risk-low"]
    }
  },
  "protectedFiles": [
    {
      "path": "src/payments/process-payment.ts",
      "scope": "payment processing",
      "riskLevel": "high",
      "constraints": [
        { "type": "do_not_change", "rule": "fee calculation formula" },
        { "type": "do_not_change", "rule": "transaction recording sequence" }
      ],
      "requiresHumanReview": [
        "any modification to this file",
        "changes to the payment amount calculation"
      ],
      "verification": [
        "npm run test:payments",
        "npm run test:integration:payments"
      ],
      "addedBy": "AUTH-001",
      "addedAt": "2026-03-22T00:00:00Z"
    }
  ]
}
```

### LCP Comment Block (In-Source)

The in-source comment block is the **authoritative declaration**. The registry is derived from scanning source files.

```typescript
/**
 * @LCP
 * scope: payment processing
 * risk_level: high
 *
 * constraints:
 *   - do_not_change: fee calculation formula
 *   - do_not_change: transaction recording sequence
 *
 * requires_human_review:
 *   - any modification to this file
 *   - changes to payment amount calculation
 *
 * verification:
 *   - npm run test:payments
 *   - npm run test:integration:payments
 */
```

---

## 5. LCP Comment Format (Language-Agnostic)

Following fspec's existing pattern for `@step` comments, LCP uses language-appropriate comment syntax:

### TypeScript/JavaScript/Java/C/C++
```typescript
/**
 * @LCP
 * scope: authentication
 * risk_level: high
 *
 * constraints:
 *   - do_not_change: password hashing algorithm
 *   - do_not_change: session token generation
 *
 * requires_human_review:
 *   - any modification to auth middleware
 *
 * verification:
 *   - npm run test:auth
 */
```

### Python
```python
# @LCP
# scope: authentication
# risk_level: high
#
# constraints:
#   - do_not_change: password hashing algorithm
#   - do_not_change: session token generation
#
# requires_human_review:
#   - any modification to auth middleware
#
# verification:
#   - pytest tests/test_auth.py
```

### Rust
```rust
/// @LCP
/// scope: cryptographic operations
/// risk_level: high
///
/// constraints:
///   - do_not_change: key derivation function
///   - do_not_change: constant-time comparison
///
/// requires_human_review:
///   - any modification to this module
///
/// verification:
///   - cargo test --package crypto
```

### Ruby
```ruby
# @LCP
# scope: billing
# risk_level: high
#
# constraints:
#   - do_not_change: invoice calculation logic
#
# requires_human_review:
#   - changes affecting monetary values
#
# verification:
#   - bundle exec rspec spec/billing/
```

---

## 6. Risk Zones

### Zone Classification (from PDD)

| Zone | Examples | Rules |
|------|----------|-------|
| **High Risk** | payments, auth, encryption, PII handling | LCP **required**; human review **required** |
| **Medium Risk** | pricing, order flow, data transformation | LCP **recommended** |
| **Low Risk** | UI components, logging, utilities | Automated refactors allowed |

### Zone-to-Tag Mapping

Risk zones integrate with fspec's tag system:

```
@risk-high    → LCP block required in source files
@risk-medium  → LCP block recommended (warning if absent)
@risk-low     → No LCP requirement
```

### Zone Assignment

Zones can be assigned at multiple levels:
1. **Path-based** — glob patterns in `lcp-registry.json` (e.g., `src/payments/**`)
2. **Feature-based** — via tags on feature files (`@risk-high`)
3. **Work-unit-based** — architecture notes on work units
4. **Foundation-based** — derived from bounded contexts in the foundation event storm

---

## 7. CLI Commands

### Core Commands

```bash
# Scan codebase and build/update LCP registry from @LCP comments in source files
fspec scan-lcp

# Show all LCP-protected files with their constraints
fspec show-lcp

# Show LCP for a specific file
fspec show-lcp --file src/payments/process-payment.ts

# Validate all LCP blocks (syntax, structure, verification commands exist)
fspec validate-lcp

# Run verification commands for all LCP-protected files that have changed
fspec verify-lcp
fspec verify-lcp --file src/payments/process-payment.ts

# Check if a file is in a risk zone and whether it has required LCP
fspec check-risk src/payments/new-file.ts
```

### Risk Zone Management

```bash
# Add a path pattern to a risk zone
fspec add-risk-zone high "src/payments/**"
fspec add-risk-zone medium "src/pricing/**"

# Remove a path from a risk zone
fspec remove-risk-zone high "src/payments/test-helpers/**"

# Show all risk zones with their paths
fspec show-risk-zones

# Audit: find high-risk files missing LCP blocks
fspec audit-lcp
fspec audit-lcp --fix  # Generate skeleton LCP blocks for missing files
```

### LCP Block Generation

```bash
# Generate a skeleton LCP block for a file (interactive)
fspec init-lcp src/payments/process-payment.ts

# Generate LCP blocks for all files in a risk zone
fspec init-lcp --zone high

# Generate LCP from work unit architecture notes
fspec init-lcp --from-work-unit AUTH-001
```

---

## 8. Validation & Enforcement

### Validation Layers

Following PDD's principle of layered validation:

#### 1. Structural Validation (`fspec validate-lcp`)
- Every `@LCP` block parses correctly (valid YAML-like structure)
- Required fields present: `scope`, `risk_level`
- `risk_level` is one of: `high`, `medium`, `low`
- At least one constraint for `high` risk files
- Verification commands reference real test scripts

#### 2. Completeness Validation (`fspec audit-lcp`)
- All files in `high` risk zones have `@LCP` blocks
- Warn for `medium` risk zone files without `@LCP` blocks
- Report files with `@LCP` blocks not in any risk zone (orphaned LCPs)

#### 3. Change Validation (hook-based)
- Pre-commit: detect changed files with `@LCP` blocks
- For `high` risk: block commit unless verification passes
- For `requires_human_review`: emit system-reminder warning to AI agents
- Track which constraints were potentially affected by the diff

#### 4. Verification Execution (`fspec verify-lcp`)
- Run verification commands listed in `@LCP` blocks for changed files
- Report pass/fail per file per verification command
- Integrate with fspec's hook system for automatic execution

### Integration with `fspec check`

Add LCP validation to the existing `fspec check` command:

```
$ fspec check
✓ Gherkin syntax valid
✓ Tags validated
✓ Coverage complete
✓ LCP blocks valid (12 files)
✗ LCP audit: 2 high-risk files missing @LCP blocks
  - src/auth/validate-token.ts
  - src/payments/refund.ts
```

---

## 9. Hook Integration

### Global Hook for LCP

```json
{
  "hooks": {
    "pre-implementing": [
      {
        "name": "lcp-scan",
        "command": "fspec scan-lcp",
        "blocking": false,
        "timeout": 30
      }
    ],
    "pre-validating": [
      {
        "name": "lcp-verify",
        "command": "fspec verify-lcp",
        "blocking": true,
        "timeout": 120,
        "condition": {
          "tags": ["@risk-high", "@risk-medium"]
        }
      }
    ]
  }
}
```

### Virtual Hook Auto-Creation

When a work unit touches files in a high-risk zone, automatically suggest virtual hooks:

```
fspec add-virtual-hook AUTH-001 post-implementing "fspec verify-lcp --file src/auth/login.ts" --blocking true
```

---

## 10. System Reminder Integration

When an AI agent is about to edit a file with an `@LCP` block, the system reminder should include:

```xml
<system-reminder>
⚠️ LOCAL CHANGE PROTOCOL ACTIVE

The file you are editing has a Local Change Protocol:

File: src/payments/process-payment.ts
Scope: payment processing
Risk Level: HIGH

CONSTRAINTS (DO NOT VIOLATE):
  - DO NOT CHANGE: fee calculation formula
  - DO NOT CHANGE: transaction recording sequence

REQUIRES HUMAN REVIEW:
  - Any modification to this file
  - Changes to payment amount calculation

VERIFICATION (must pass after changes):
  - npm run test:payments
  - npm run test:integration:payments

You MUST run the verification commands after making changes.
You MUST NOT modify the constrained areas unless explicitly instructed.
</system-reminder>
```

---

## 11. Story Breakdown

### LCP-001: LCP Data Model & Registry (3 points)
- Design the `@LCP` structured comment block grammar (language-agnostic format)
- Define the `lcp-registry.json` schema with two distinct sections:
  - **Risk zone configuration** (manually managed glob patterns per risk level)
  - **Protected files cache** (derived from scanning source files)
- Write JSON Schema (Ajv) for registry validation
- This story produces the **specification only** — no parser implementation

### LCP-002: LCP Parser, Scanner & Display (5 points)
- Implement the `@LCP` comment block parser for all supported languages (TS, JS, Python, Rust, Ruby, Go, Java, C, C++)
- Implement `scan-lcp` command that walks the codebase and builds/updates the protectedFiles section
- Support incremental scanning (only changed files via git diff)
- Detect orphaned registry entries (file deleted but still in registry)
- Implement `show-lcp` command to display all protected files with constraints (text + JSON output, optional `--file` flag)

### LCP-003: Risk Zone Management (3 points)
- Implement `add-risk-zone`, `remove-risk-zone`, `show-risk-zones` commands
- Store risk zone configuration in the riskZones section of `lcp-registry.json`
- Glob-based path matching for zone membership
- Integration with fspec tag system (`@risk-high`, `@risk-medium`, `@risk-low`)

### LCP-004: LCP Validation & Audit (5 points)
- Implement `validate-lcp` command (structural validation of all `@LCP` blocks: syntax, required fields, valid values, constraints for high-risk)
- Implement `audit-lcp` command (completeness: high-risk files without LCP, medium-risk warnings, orphaned LCPs)
- Implement `audit-lcp --fix` to generate **minimal skeleton** LCP blocks for compliance (not the full interactive init — that is LCP-008)
- Integrate into existing `fspec check` pipeline
- Report formatting (text and JSON output)

### LCP-005: LCP Verification Engine (5 points)
- Implement `verify-lcp` command that runs verification commands from `@LCP` blocks
- Detect which files changed using git diff integration
- Only run verification for LCP-protected files that were modified
- **Leverage fspec's existing process spawning infrastructure** (hook execution engine pattern) for command execution and timeout — not a new execution engine
- Report pass/fail per file per verification command
- Support `--file` flag for single file or all changed LCP files

### LCP-006: LCP Workflow Enforcement (5 points) — was 3, increased due to dual-layer scope
- **TypeScript layer:** Add LCP verification as a configurable pre-validating workflow hook in `fspec-hooks.json`; auto-suggest virtual hooks when work unit touches high-risk zones; block implementing→validating if verification fails
- **Rust agent core layer:** Provide a `pre_tool_use` hook script that checks LCP constraints before Write/Edit/ApplyPatch tools modify protected files — this is the **hard enforcement** path that physically prevents AI agents from changing constrained code
- Both layers use the existing hook configuration schema in `fspec-hooks.json`

### LCP-007: System Reminder & Agent Guidance (3 points)
- **TypeScript layer:** When fspec commands produce output about LCP-protected files, append `<system-reminder>` blocks with constraints, risk level, verification commands
- **Rust layer:** Configure `session_start` or `pre_tool_use` hooks to inject LCP context for continuous awareness (not just at command output time)
- Format LCP data for optimal AI consumption: explicit DO NOT CHANGE directives, risk zone context
- Depends on both LCP-002 (parser) and LCP-003 (risk zone context)

### LCP-008: LCP Scaffold & Init (2 points)
- Implement `init-lcp` command for **interactive, context-aware** LCP block generation
- Generate from work unit architecture notes (`--from-work-unit` flag)
- Generate for all files matching a risk zone (`--zone` flag)
- Insert correctly-formatted comment blocks using language-appropriate syntax (detected from file extension)
- Unlike `audit-lcp --fix` (minimal skeletons), `init-lcp` produces meaningful scope, constraints, and verification

---

## 12. Academic References

The LCP concept in PDD builds on three foundational papers, all included as PDF attachments:

### 1. Tony Hoare — "An Axiomatic Basis for Computer Programming" (1969)

**File:** `An Axiomatic basis  for computer programming (1969) Hoare.pdf`

Hoare introduced the formal notion of **preconditions and postconditions** for program correctness — the intellectual foundation of Design by Contract. LCP's `constraints` and `verification` fields directly descend from this work:

- **Preconditions** → LCP constraints (what must be true before changes)
- **Postconditions** → LCP verification (what must pass after changes)
- **Invariants** → LCP `do_not_change` rules (what must never be altered)

> Key insight: "If the assertion P(x) is satisfied before initiation of a program Q(x,y), then the assertion R(y) will be satisfied on its completion."

**Relevance to LCP:** Just as Hoare Logic provides formal verification that program behavior is preserved across execution, LCP provides lightweight verification that critical behavior is preserved across code changes.

### 2. David L. Parnas — "On the Criteria to Be Used in Decomposing Systems into Modules" (1972)

**File:** `On the Criteri To Be Used in Decomposing Systems into Modules (1972) DL Parnes.pdf` *(Note: filename contains typos from PDD source — correct spelling is "Criteria" and "Parnas")*

Parnas introduced **Information Hiding** — the principle that modules should hide design decisions that are likely to change. LCP operationalizes this for AI-assisted development:

- **Module boundaries** → LCP `scope` field defines the protected concern
- **Hidden design decisions** → LCP `do_not_change` constraints make critical decisions explicit
- **Interface stability** → LCP `requires_human_review` triggers protect interfaces from unreviewed changes

> Key insight: "We propose instead that one begins with a list of difficult design decisions or design decisions which are likely to change. Each module is then designed to hide such a decision from the others."

**Relevance to LCP:** In AI-assisted development, the "difficult design decisions" that Parnas warns about are exactly the ones that need LCP protection. AI agents don't know which decisions are load-bearing — LCP makes this explicit.

### 3. Niklaus Wirth — "Program Development by Stepwise Refinement" (1971)

**File:** `Program Development by Stepwise Refinement by Niklaus Wirth (1971).pdf`

Wirth introduced **Stepwise Refinement** — building systems by progressively refining high-level descriptions into implementation. LCP connects to this through fspec's ACDD pipeline:

```
Discovery (high-level) → Specification → Architecture → Implementation (detailed)
                                              ↓
                                    LCP added at implementation
                                    as architectural constraints
                                    become concrete code
```

> Key insight: "In each step, one or several instructions of the given program are decomposed into more detailed instructions. This successive decomposition or refinement of specifications terminates when all instructions are expressed in terms of an underlying computer or programming language."

**Relevance to LCP:** LCP blocks represent the **architectural decisions that survived refinement** — the constraints that were established during specification/architecture but must be preserved in the implementation. They are the bridge between design intent and code-level protection.

---

## 13. Source Material from PDD Repository

### Local Change Protocols (from `/patterns/local-change-protocols.md`)

> **Problem:** Some code is dangerous to change. The risk is often undocumented and invisible.
>
> **Solution:** A Local Change Protocol embeds binding rules directly into high-risk code. It governs how the code may be changed.
>
> **Characteristics:**
> - Local to the file
> - Explicit about risk
> - Enforceable by tooling
> - Understandable by humans and AI

### Zones and Guardrails (from `/patterns/zones-and-guardrails.md`)

> **Problem:** Large repositories hide risk. Not all code deserves the same level of automation.
>
> **Solution:** Zones classify code by risk level and define required protocols.
>
> | Zone | Examples | Rules |
> |------|----------|-------|
> | **High Risk** | payments, auth | LCP required, human review required |
> | **Medium Risk** | pricing, order flow | LBP recommended |
> | **Low Risk** | ui, logging | Automated refactors allowed |

### PDD Core Principles (relevant to LCP)

1. **Deterministic Process** — "The AI can be probabilistic. The pipeline cannot." LCP adds deterministic constraints at the file level.
2. **Constraint Before Generation** — LCP enforces constraints before AI can modify code.
3. **Layered Validation** — LCP adds a file-level validation layer complementing existing phase-level checks.
4. **Protocol Enforcement** — LCP is a protocol that governs how specific files may evolve.

### Design by Contract Integration

From PDD's history documentation:

> "Contracts define explicit guarantees about how a component behaves. They describe what must be true before, during, and after execution."
>
> Contract elements map to LCP:
> - **Preconditions** → constraints (what must hold before making changes)
> - **Postconditions** → verification (what must pass after changes)
> - **Invariants** → do_not_change rules (what must never be altered)

---

## Summary

Local Change Protocols bring PDD's file-level governance concept into fspec's existing ACDD framework. The implementation:

1. **Leverages existing patterns** — structured comments (like `@step`), hook system, tag system, system reminders
2. **Fills a real gap** — no current mechanism for code-level constraint embedding
3. **Is incrementally adoptable** — start with high-risk files, expand as needed
4. **Is AI-native** — designed to be both human-readable and machine-parseable
5. **Builds on classical CS** — Hoare's contracts, Parnas's information hiding, Wirth's stepwise refinement

The 8 stories total approximately **31 story points** and can be implemented incrementally:
- **Layer 1 (Foundation):** LCP-001 (data model) → LCP-002 (parser/scanner) + LCP-003 (risk zones)
- **Layer 2 (Validation):** LCP-004 (validate/audit) + LCP-005 (verification engine)
- **Layer 3 (Enforcement):** LCP-006 (workflow enforcement across TypeScript hooks + Rust pre_tool_use) + LCP-007 (system reminders across both layers)
- **Layer 4 (UX):** LCP-008 (interactive scaffolding)

### Architecture Note: Two-Layer Enforcement

fspec has two completely separate hook systems that coexist in `fspec-hooks.json`:
1. **TypeScript CLI hooks** (`src/commands/hooks/`) — ACDD workflow transitions (pre-update-work-unit-status, post-implementing, etc.)
2. **Rust agent lifecycle hooks** (`codelet/core/src/lifecycle_hooks/`) — runtime agent behavior (pre_tool_use, post_tool_use, session_start, etc.)

LCP enforcement MUST integrate with both:
- **Soft enforcement** (TypeScript) — system reminders guide AI agents, workflow hooks validate at phase transitions
- **Hard enforcement** (Rust) — `pre_tool_use` hooks physically block Write/Edit/ApplyPatch on constrained code
