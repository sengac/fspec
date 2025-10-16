# Getting Started with fspec

## Overview

**Important:** You don't run fspec commands directly. Instead, you give high-level requests to your AI agent (like Claude Code), and the agent uses fspec to manage the workflow for you.

## Installation

### 1. Install fspec

```bash
git clone https://github.com/sengac/fspec.git
cd fspec
npm install && npm run build && npm run install:local
```

### 2. Initialize in Your Project

```bash
cd /path/to/your/project
fspec init
```

This creates:
- `.claude/commands/fspec.md` - Forward ACDD command for building new features
- `.claude/commands/rspec.md` - Reverse ACDD command for existing codebases
- `spec/CLAUDE.md` - Specification management guidelines

**Works with any AI agent:** While designed for Claude Code, you can use the generated `fspec.md` and `rspec.md` command files with other AI agents by mapping them to your agent's command system. If using another agent, rename `spec/CLAUDE.md` to `spec/AGENTS.md` for clarity.

## Using fspec with Your AI Agent

### For New Features (Forward ACDD)

Simply ask your AI agent to build something using `/fspec`:

```
/fspec Build user authentication feature
/fspec Add password reset functionality
/fspec Implement API rate limiting
```

The AI agent will:
1. **Create a work unit** and move it through Kanban states
2. **Do Example Mapping** - Ask you clarifying questions about requirements
3. **Generate validated Gherkin specs** from your answers
4. **Write tests first** that map to the scenarios
5. **Implement code** to make those tests pass
6. **Enforce ACDD workflow** - can't skip phases

### For Existing Code (Reverse ACDD)

Ask your AI agent to analyze existing code using `/rspec`:

```
/rspec Analyze the entire codebase
/rspec Document the payment processing system
/rspec Create specs for src/api/routes.ts
```

The AI agent will:
1. **Analyze the code** to identify user-facing interactions
2. **Create feature files** with inferred acceptance criteria
3. **Group into epics** by business domain
4. **Generate test skeletons** (structure only, not implemented)
5. **Link existing code** to scenarios for traceability

## Example Interaction

Here's what a typical interaction looks like:

```
You: /fspec Build user login feature

Claude: I'll use fspec to manage this work through ACDD workflow.
        Let me start with discovery...

        I'm creating work unit AUTH-001 for "User Login" and moving
        it to the specifying phase. Now let me ask some clarifying
        questions:

        1. Should we support OAuth in addition to email/password?
        2. What are your password requirements (length, complexity)?
        3. Do you want "remember me" functionality?

You: 1. Yes, support Google OAuth
     2. At least 8 characters, must include a number
     3. Yes, remember for 30 days

Claude: Perfect! Based on your answers, I'm adding these rules and
        examples to the work unit:

        Rules:
        - Password must be 8+ characters with at least one number
        - Support email/password and Google OAuth
        - Remember me stores session for 30 days

        Now I'll generate validated Gherkin scenarios from this...

        ✓ Created spec/features/user-authentication.feature

        Moving to testing phase. I'll write tests that map to these
        scenarios BEFORE any implementation code...
```

## What Happens Behind the Scenes

When you use `/fspec` or `/rspec`, the AI agent is running fspec commands on your behalf:

```bash
# Creating work units
fspec create-work-unit AUTH "User login feature"

# Managing workflow states
fspec update-work-unit-status AUTH-001 specifying

# Example mapping (discovery)
fspec add-rule AUTH-001 "Password must be 8+ characters"
fspec add-example AUTH-001 "User logs in with valid email"
fspec add-question AUTH-001 "@human: Should we support OAuth?"

# Generating specs from discovery
fspec generate-scenarios AUTH-001

# Validating Gherkin syntax
fspec validate

# Tracking coverage
fspec link-coverage user-authentication --scenario "Login" --test-file ...
```

**You focus on requirements, the AI handles workflow enforcement.**

## Key Concepts

### Kanban Workflow (7 States)

Work units flow through enforced states:
- `backlog` → `specifying` → `testing` → `implementing` → `validating` → `done`
- Plus `blocked` for work that can't proceed

**The AI agent cannot skip phases.** This ensures ACDD discipline.

### Example Mapping (Discovery)

Before writing code, the AI uses structured discovery:
- **Rules** (blue cards) - Business rules governing the feature
- **Examples** (green cards) - Concrete examples illustrating behavior
- **Questions** (red cards) - Clarifying questions for you to answer
- **Attachments** - Diagrams, mockups, or documents

### Coverage Tracking

fspec tracks scenario-to-test-to-implementation mappings:
- Which scenarios have test coverage
- Line ranges in test files
- Which implementation files are tested
- Coverage statistics

## Getting Help

If you want to see what fspec commands are available (for reference):

```bash
fspec help              # Overview
fspec help work         # Kanban workflow commands
fspec help discovery    # Example mapping commands
fspec help specs        # Gherkin feature file commands
fspec help hooks        # Lifecycle hooks
```

But remember: **You don't run these yourself** - your AI agent uses them.

## Next Steps

- [Installation Guide](./installation.md) - Detailed setup instructions
- [Reverse ACDD](./reverse-acdd.md) - Document existing codebases
- [Project Management](./project-management.md) - Understand the Kanban workflow
- [Coverage Tracking](./coverage-tracking.md) - Learn about traceability

## Troubleshooting

**Q: The AI agent isn't using fspec commands**

A: Make sure you ran `fspec init` in your project directory, and that `.claude/commands/fspec.md` exists. Use `/fspec` (with slash) to invoke the command.

**Q: Can I use fspec without an AI agent?**

A: Yes, but that's not the intended use case. You can run commands directly for debugging or manual workflows, but fspec is designed to be driven by AI agents.

**Q: Does this work with AI agents other than Claude Code?**

A: Yes! Take the generated `fspec.md` and `rspec.md` files and map them to your AI agent's command system.
