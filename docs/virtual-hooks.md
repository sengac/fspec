# Virtual Hooks: Work Unit-Scoped Quality Gates

## Overview

**Virtual hooks** are ephemeral, work unit-specific lifecycle hooks that let AI agents attach temporary quality gates to individual work units. Unlike global hooks (configured in `spec/fspec-hooks.json`), virtual hooks are:

- **Work unit-scoped** - Apply to one work unit only
- **Temporary** - Removed when work reaches "done"
- **Auto-generated** - fspec creates scripts automatically
- **Git context-aware** - Can pass changed files to lint/format commands
- **Blocking or non-blocking** - Enforce or advise

**Perfect for:** Story-specific linting, test requirements, security scans, formatting checks.

---

## Key Differences from Global Hooks

| Feature | Global Hooks | Virtual Hooks |
|---------|--------------|---------------|
| Scope | Project-wide | Single work unit |
| Storage | `spec/fspec-hooks.json` | `spec/work-units.json` |
| Lifespan | Permanent | Ephemeral |
| Configuration | Manual or `fspec add-hook` | CLI only (`fspec add-virtual-hook`) |
| Script Generation | Manual | Auto-generated for git context |
| Use Case | Project standards | Work unit-specific checks |

---

## How It Works

### Basic Virtual Hook

**AI runs:**
```bash
fspec add-virtual-hook AUTH-001 post-implementing "npm test" --blocking
```

**What this does:**
1. Adds hook to `spec/work-units.json` for AUTH-001
2. Hook runs AFTER status changes to "implementing"
3. If tests fail, prevents progression (blocking)
4. Stderr wrapped in `<system-reminder>` tags for AI visibility

**Example conversation:**
```
You: "Make sure we run tests after implementing this feature"

AI: I'll add a virtual hook to enforce testing...
    fspec add-virtual-hook AUTH-001 post-implementing "npm test" --blocking
    ✓ Added blocking post-implementing hook: npm test

    This hook will run automatically when AUTH-001 moves to implementing.
    If tests fail, the transition will be blocked.
```

### Git Context Hook

**AI runs:**
```bash
fspec add-virtual-hook AUTH-001 pre-validating "eslint" --git-context --blocking
```

**What this does:**
1. Adds hook to work unit
2. **Auto-generates script** in `spec/hooks/.virtual/AUTH-001-eslint.sh`
3. Script reads git status (staged/unstaged files) from stdin
4. Passes only changed files to eslint
5. Runs BEFORE status changes to "validating"

**Generated script example:**
```bash
#!/bin/bash
set -e

# Read context JSON from stdin
CONTEXT=$(cat)

# Extract staged and unstaged files
STAGED_FILES=$(echo "$CONTEXT" | jq -r '.stagedFiles[]? // empty')
UNSTAGED_FILES=$(echo "$CONTEXT" | jq -r '.unstagedFiles[]? // empty')

# Combine all changed files
ALL_FILES="$STAGED_FILES $UNSTAGED_FILES"

# Exit if no files to process
if [ -z "$ALL_FILES" ]; then
  echo "No changed files to process"
  exit 0
fi

# Run command with changed files
eslint $ALL_FILES
```

**Why git context?**
- **Efficiency** - Only lint changed files, not entire codebase
- **Relevance** - Quality checks focus on work-in-progress
- **Speed** - Faster feedback for AI agents

---

## Common Patterns

### Pattern 1: Tests After Implementation

```bash
fspec add-virtual-hook AUTH-001 post-implementing "npm test" --blocking
```

**Use case:** Ensure tests pass before moving to validation.

**When it runs:** After AUTH-001 moves to implementing.

**What happens if it fails:** Status change blocked, AI must fix tests.

### Pattern 2: Lint Changed Files Only

```bash
fspec add-virtual-hook AUTH-001 pre-validating "eslint" --git-context --blocking
```

**Use case:** Lint only the files changed in this work unit.

**When it runs:** Before AUTH-001 moves to validating.

**What happens if it fails:** Linting errors block validation, AI must fix.

### Pattern 3: Multiple Quality Gates

```bash
fspec add-virtual-hook AUTH-001 post-implementing "npm run lint" --blocking
fspec add-virtual-hook AUTH-001 post-implementing "npm run typecheck" --blocking
fspec add-virtual-hook AUTH-001 post-implementing "npm test" --blocking
```

**Use case:** Stack multiple checks at the same workflow event.

**Execution order:** All post-implementing hooks run in array order.

**What happens if any fails:** First failure blocks progression.

### Pattern 4: Non-Blocking Notifications

```bash
fspec add-virtual-hook AUTH-001 post-implementing "npm run notify"
```

**Use case:** Send Slack notification when implementing completes.

**When it runs:** After AUTH-001 moves to implementing.

**What happens if it fails:** Logged but doesn't block progression.

### Pattern 5: Copy Hooks Between Work Units

```bash
# Set up template
fspec add-virtual-hook TEMPLATE-001 post-implementing "npm run lint" --blocking
fspec add-virtual-hook TEMPLATE-001 pre-validating "npm run typecheck" --blocking

# Copy to actual work units
fspec copy-virtual-hooks --from TEMPLATE-001 --to AUTH-001
fspec copy-virtual-hooks --from TEMPLATE-001 --to AUTH-002
```

**Use case:** Apply same quality gates to related work units.

---

## Managing Virtual Hooks

### List Hooks for Work Unit

**AI runs:**
```bash
fspec list-virtual-hooks AUTH-001
```

**Output:**
```
Virtual hooks for AUTH-001:

🪝  eslint (pre-validating, blocking, git context)
   Command: eslint
   Script: spec/hooks/.virtual/AUTH-001-eslint.sh

🪝  npm test (post-implementing, blocking)
   Command: npm test

🪝  notify (post-implementing, non-blocking)
   Command: npm run notify
```

### Remove Specific Hook

**AI runs:**
```bash
fspec remove-virtual-hook AUTH-001 eslint
```

**What this does:**
1. Removes hook from work unit
2. Deletes generated script file (if git context)
3. Confirms removal

### Clear All Hooks

**AI runs:**
```bash
fspec clear-virtual-hooks AUTH-001
```

**Output:**
```
✓ Cleared 3 virtual hook(s) from AUTH-001
✓ Deleted 1 generated script(s)
```

### Copy Hooks

**AI runs:**
```bash
# Copy all hooks
fspec copy-virtual-hooks --from AUTH-001 --to AUTH-002

# Copy specific hook only
fspec copy-virtual-hooks --from AUTH-001 --to AUTH-002 --hook-name eslint
```

---

## Hook Execution Order

When a command triggers hooks:

1. **Virtual hooks** (work unit-scoped) - Execute FIRST
2. **Global hooks** (project-wide) - Execute SECOND

Within each category, hooks execute in array order.

**Example:**
```
Work unit AUTH-001 has virtual hook: "npm run lint"
Global hooks have: "fspec validate"

When moving AUTH-001 to validating:
  1. AUTH-001 virtual hook: npm run lint
  2. Global hook: fspec validate
```

---

## Blocking vs Non-Blocking

### Blocking Hooks (`--blocking` flag)

**Behavior:**
- Pre-hook failure → Prevents workflow transition
- Post-hook failure → Sets exit code to 1
- Stderr wrapped in `<system-reminder>` tags

**Use for:** Critical quality gates (linting, tests, type checking)

**Example failure:**
```xml
<system-reminder>
BLOCKING HOOK FAILURE: Virtual hook 'eslint' for AUTH-001 failed.

Stderr:
  /path/to/file.ts:42:3 - error TS2304: Cannot find name 'foo'.

This is a BLOCKING hook. Fix the errors before proceeding.
</system-reminder>
```

### Non-Blocking Hooks (default)

**Behavior:**
- Failure logged but doesn't prevent progression
- Useful for notifications, metrics, optional checks

**Use for:** Nice-to-have checks, alerting, data collection

---

## Cleanup After Completion

When work unit reaches "done", AI should ask about cleanup:

```
You: /fspec Move AUTH-001 to done

AI: AUTH-001 is now complete!

    I notice AUTH-001 has 3 virtual hooks:
    - eslint (pre-validating, blocking)
    - npm test (post-implementing, blocking)
    - notify (post-implementing, non-blocking)

    Should I remove these hooks now that work is done? (y/n)

You: y

AI: fspec clear-virtual-hooks AUTH-001
    ✓ Cleared 3 virtual hook(s) from AUTH-001
    ✓ Deleted 1 generated script(s)

    Work unit AUTH-001 is now clean!
```

**Why remove?**
- Completed work doesn't need ephemeral checks
- Reduces noise in work-units.json
- Cleans up generated script files

---

## Script File Management

Git context hooks generate scripts automatically:

- **Location:** `spec/hooks/.virtual/<work-unit-id>-<hook-name>.sh`
- **Generation:** Automatic when using `--git-context`
- **Cleanup:** Automatic when removing hooks
- **Lifecycle:** Created on add, deleted on remove

**Important:** Do NOT manually edit generated scripts. They're regenerated on every modification.

---

## Best Practices

✅ **DO:**
- Add virtual hooks during specifying/testing phases
- Use `--blocking` for critical checks
- Use `--git-context` for file-specific commands
- Remove hooks when work reaches "done"
- Copy hooks to related work units
- Use descriptive hook names

❌ **DON'T:**
- Skip removal when work complete (causes clutter)
- Use virtual hooks for permanent project standards
- Manually edit generated scripts
- Forget to test hooks before relying on them

---

## Use Cases

### Security Scanning

```bash
fspec add-virtual-hook AUTH-001 pre-validating "npm audit" --blocking
```

Ensures no vulnerabilities in dependencies before validation.

### Format Changed Files

```bash
fspec add-virtual-hook UI-002 pre-validating "prettier" --git-context --blocking
```

Formats only changed files before validation.

### Custom Test Subset

```bash
fspec add-virtual-hook API-003 post-implementing "npm test -- --grep 'API'" --blocking
```

Runs only API-related tests for this work unit.

### Performance Budget

```bash
fspec add-virtual-hook PERF-001 pre-validating "./scripts/check-bundle-size.sh" --blocking
```

Custom script enforces bundle size limits.

### Database Migrations

```bash
fspec add-virtual-hook DB-001 post-implementing "npm run migrate:test" --blocking
```

Runs migrations in test environment before validation.

---

## Troubleshooting

**Hook not executing:**
- Check `fspec list-virtual-hooks <work-unit-id>` to verify it exists
- Ensure command is in PATH or use full path
- Check hook event matches workflow transition

**Git context failing:**
- Verify `jq` is installed (required for JSON parsing)
- Check that files actually changed (git status)
- Look at generated script: `cat spec/hooks/.virtual/<script>.sh`

**Permission denied:**
- Generated scripts are automatically executable (0o755)
- If manually creating scripts, run `chmod +x <script>.sh`

**Debugging:**
```bash
# List hooks
fspec list-virtual-hooks AUTH-001

# Check generated script
cat spec/hooks/.virtual/AUTH-001-eslint.sh

# Manually test git context script
echo '{"stagedFiles":["src/auth.ts"],"unstagedFiles":[]}' | \
  spec/hooks/.virtual/AUTH-001-eslint.sh
```

---

## Commands Reference

```bash
# Add virtual hook
fspec add-virtual-hook <work-unit-id> <event> <command> [--blocking] [--git-context]

# List virtual hooks
fspec list-virtual-hooks <work-unit-id>

# Remove specific hook
fspec remove-virtual-hook <work-unit-id> <hook-name>

# Clear all hooks
fspec clear-virtual-hooks <work-unit-id>

# Copy hooks
fspec copy-virtual-hooks --from <source-id> --to <target-id> [--hook-name <name>]
```

**Get detailed help:**
```bash
fspec add-virtual-hook --help
fspec list-virtual-hooks --help
fspec remove-virtual-hook --help
fspec clear-virtual-hooks --help
fspec copy-virtual-hooks --help
```

---

## Virtual Hooks vs Global Hooks

**Use Virtual Hooks When:**
- ✅ Quality check applies to ONE work unit only
- ✅ Hook is temporary (remove when done)
- ✅ Different work units need different checks
- ✅ Experimenting with new quality gates

**Use Global Hooks When:**
- ✅ Quality check applies to ALL work units
- ✅ Hook is permanent (project standard)
- ✅ Enforcing team-wide practices
- ✅ Pre-commit, pre-push, CI/CD integration

**Decision tree:**
```
Is this check needed for ALL work units?
  → Yes: Use global hook
  → No: Continue

Is this check permanent (project standard)?
  → Yes: Use global hook
  → No: Continue

Is this check specific to ONE work unit?
  → Yes: Use virtual hook
```

---

**Temporary quality gates for work units. Permanent standards for projects.**
