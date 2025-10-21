# {{AGENT_NAME}} Development Guidelines for fspec

This document provides guidelines for AI assistants (particularly {{AGENT_NAME}}) working on the **fspec codebase**. This is about DEVELOPING fspec itself, not using it.

**For using fspec commands and ACDD workflow**: See [spec/{{AGENT_NAME}}.md](spec/{{AGENT_NAME}}.md)

---

## Project Overview

**fspec** is a standardized CLI tool for AI agents to manage Gherkin-based feature specifications and project work units using Acceptance Criteria Driven Development (ACDD).

- **Repository**: https://github.com/sengac/fspec
- **License**: MIT

For complete project context:
- **Project foundation**: [spec/FOUNDATION.md](spec/FOUNDATION.md)
- **Using fspec workflow**: [spec/{{AGENT_NAME}}.md](spec/{{AGENT_NAME}}.md)

---

<system-reminder>
CRITICAL: Follow ACDD workflow order.
DO NOT skip phases.
ALWAYS use fspec commands for all operations.
</system-reminder>

## MANDATORY CODING STANDARDS - ZERO TOLERANCE

**ALL CODE MUST PASS QUALITY CHECKS BEFORE COMMITTING**

### CRITICAL DO NOT VIOLATIONS - CODE WILL BE REJECTED

#### TypeScript Violations:

- ❌ **NEVER** use `any` type - use proper types always
- ❌ **NEVER** use `as unknown as` - use proper type guards or generics
- ❌ **NEVER** use `require()` - only ES6 `import`/`export`
- ❌ **NEVER** use CommonJS syntax (`module.exports`, `__dirname`, `__filename`)
- ❌ **NEVER** use file extensions in TypeScript imports
- ❌ **NEVER** use `var` - only `const`/`let`
- ❌ **NEVER** use `==` or `!=` - only `===` and `!==`

---

## Development Methodology: Acceptance Criteria Driven Development (ACDD)

Before proceeding, ultrathink your next steps and deeply consider the implications.

This project uses **Acceptance Criteria Driven Development** where:

1. **Specifications come first** - We define acceptance criteria in Gherkin format
2. **Tests come second** - We write tests that map to scenarios BEFORE any code
3. **Code comes last** - We implement just enough code to make tests pass

### CRITICAL RULES:

- **NEVER write production code without a failing test first**
- **Each Gherkin scenario must have corresponding tests**
- **Tests must map 1:1 to scenarios in feature files**

---

## Commands and Workflow

All fspec commands are available via the CLI. Use them frequently:

```bash
fspec validate          # Validate Gherkin syntax
fspec format           # Format feature files
fspec board            # View Kanban board
fspec help specs       # Gherkin management
fspec help work        # Kanban workflow
```

For the complete command reference, run:
```bash
fspec --help
```

Slash commands are available at {{SLASH_COMMAND_PATH}}.

---

## Best Practices

1. **Quality over Speed**: Take a moment to reflect before implementing
2. **Ask Before Major Changes**: Propose refactoring before implementing
3. **Maintain Specifications**: Update feature files as code evolves
4. **Cross-Platform**: Consider Windows path/shell differences
5. **No Shortcuts**: Fix issues properly

---

## When You Get Stuck

1. Check existing patterns in the codebase
2. Refer to `spec/FOUNDATION.md` for project goals
3. Refer to `spec/{{AGENT_NAME}}.md` for fspec usage
4. Run tests to verify changes
5. Check feature files for acceptance criteria

---

## Contributing

When contributing to fspec:

1. Follow ACDD: Feature file → Tests → Implementation
2. Ensure all tests pass
3. Update relevant documentation
4. Follow established patterns
5. Keep commits focused and descriptive

Remember: The goal is to create a CLI tool that helps AI agents manage Gherkin specifications using ACDD. Every line of code should contribute to this goal.
