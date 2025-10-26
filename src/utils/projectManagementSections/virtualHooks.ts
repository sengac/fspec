import type { AgentConfig } from '../agentRegistry';
import { formatSystemReminder } from '../projectManagementTemplate';

export function getVirtualHooksSection(agent: AgentConfig): string {
  const blockingHookExample = formatSystemReminder(
    `BLOCKING HOOK FAILURE: Virtual hook 'eslint' for AUTH-001 failed.

Stderr:
  /path/to/file.ts:42:3 - error TS2304: Cannot find name 'foo'.

This is a BLOCKING hook. Fix the errors before proceeding.`,
    agent
  );

  return `## Virtual Hooks: Work Unit-Scoped Quality Gates

Virtual hooks are ephemeral, work unit-specific hooks that allow AI agents to attach temporary quality checks to individual work units. Unlike global hooks (configured in \`spec/fspec-hooks.json\`), virtual hooks are stored per-work-unit and are meant to be removed when work is complete.

### What Are Virtual Hooks?

**Virtual hooks** are lifecycle hooks scoped to a single work unit. They enable:

- **Ephemeral quality gates** - Attach linting, testing, security scans to specific stories
- **Work-unit-specific automation** - Different checks for different work units
- **Temporary enforcement** - Remove hooks when work reaches "done" status
- **AI-driven workflow** - AI adds/removes hooks based on work unit context

**Key Differences from Global Hooks:**

| Feature | Global Hooks | Virtual Hooks |
|---------|--------------|---------------|
| Scope | Project-wide | Single work unit |
| Storage | \`spec/fspec-hooks.json\` | \`spec/work-units.json\` (per work unit) |
| Lifespan | Permanent | Ephemeral (removed when done) |
| Configuration | Manual file editing or \`fspec add-hook\` | CLI only (\`fspec add-virtual-hook\`) |
| Script Generation | Manual script creation | Auto-generated for git context |
| Use Case | Project standards | Work unit-specific checks |

### Virtual Hook Configuration

Virtual hooks are stored in \`spec/work-units.json\` under \`workUnit.virtualHooks\`:

\`\`\`json
{
  "workUnits": {
    "AUTH-001": {
      "id": "AUTH-001",
      "title": "User Login",
      "status": "implementing",
      "virtualHooks": [
        {
          "name": "eslint",
          "event": "post-implementing",
          "command": "<quality-check-commands>",
          "blocking": true,
          "gitContext": false
        },
        {
          "name": "prettier",
          "event": "post-implementing",
          "command": "prettier --check .",
          "blocking": false,
          "gitContext": false
        },
        {
          "name": "eslint-changed",
          "event": "pre-validating",
          "command": "eslint",
          "blocking": true,
          "gitContext": true
        }
      ]
    }
  }
}
\`\`\`

### Virtual Hook Commands

#### Adding Virtual Hooks

\`\`\`bash
# Basic virtual hook (simple command)
fspec add-virtual-hook AUTH-001 post-implementing "<quality-check-commands>" --blocking

# Git context hook (processes staged/unstaged files)
fspec add-virtual-hook AUTH-001 pre-validating "eslint" --git-context --blocking

# Non-blocking hook (runs but doesn't prevent workflow transition)
fspec add-virtual-hook AUTH-001 post-implementing "prettier --check ."
\`\`\`

#### Managing Virtual Hooks

\`\`\`bash
# List virtual hooks for a work unit
fspec list-virtual-hooks AUTH-001

# Remove specific hook by name
fspec remove-virtual-hook AUTH-001 eslint

# Clear all virtual hooks from work unit
fspec clear-virtual-hooks AUTH-001

# Copy hooks from one work unit to another
fspec copy-virtual-hooks --from AUTH-001 --to AUTH-002

# Copy specific hook only
fspec copy-virtual-hooks --from AUTH-001 --to AUTH-002 --hook-name eslint
\`\`\`

### Hook Execution Order

When a command triggers hooks, execution order is:

1. **Virtual hooks** (work unit-scoped) - Execute FIRST
2. **Global hooks** (project-wide) - Execute SECOND

Within each category, hooks execute in the order they were added (array order).

**Example**:
\`\`\`bash
# Work unit AUTH-001 has virtual hook: "<quality-check-commands>"
# Global hooks have: "fspec validate"

# When moving AUTH-001 to validating:
fspec update-work-unit-status AUTH-001 validating

# Execution order:
# 1. AUTH-001 virtual hook: <quality-check-commands>
# 2. Global hook: fspec validate
\`\`\`

### Git Context Hooks

When \`--git-context\` is specified, fspec generates a script file in \`spec/hooks/.virtual/\` that:

1. **Reads JSON context from stdin** - Receives git status (staged/unstaged files)
2. **Extracts changed files** - Parses \`stagedFiles\` and \`unstagedFiles\` arrays
3. **Passes files to command** - Runs command with changed files only

**Generated Script Example** (\`spec/hooks/.virtual/AUTH-001-eslint.sh\`):

\`\`\`bash
#!/bin/bash
set -e

# Read context JSON from stdin
CONTEXT=$(cat)

# Extract staged and unstaged files from context
STAGED_FILES=$(echo "$CONTEXT" | jq -r '.stagedFiles[]? // empty' 2>/dev/null | tr '\\n' ' ')
UNSTAGED_FILES=$(echo "$CONTEXT" | jq -r '.unstagedFiles[]? // empty' 2>/dev/null | tr '\\n' ' ')

# Combine all changed files
ALL_FILES="$STAGED_FILES $UNSTAGED_FILES"

# Exit if no files to process
if [ -z "$ALL_FILES" ]; then
  echo "No changed files to process"
  exit 0
fi

# Run command with changed files
eslint $ALL_FILES
\`\`\`

**Why Git Context?**

- **Efficiency** - Only lint/format changed files, not entire codebase
- **Relevance** - Quality checks focus on work-in-progress
- **Speed** - Faster feedback for AI agents

### Blocking vs Non-Blocking Hooks

**Blocking Hooks** (\`--blocking\` flag):
- Failure **prevents** workflow transition (for pre-hooks)
- Failure **sets exit code to 1** (for post-hooks)
- Stderr wrapped in ${agent.supportsSystemReminders ? '<system-reminder> tags' : agent.category === 'ide' || agent.category === 'extension' ? '**⚠️ IMPORTANT:** blocks' : '**IMPORTANT:** blocks'} for AI visibility
- Use for critical quality gates (linting, type checking, tests)

**Non-Blocking Hooks** (default):
- Failure logged but doesn't prevent progression
- Useful for notifications, metrics, optional checks

**Example**:
\`\`\`bash
# Blocking - prevents validating if lint fails
fspec add-virtual-hook AUTH-001 pre-validating "<quality-check-commands>" --blocking

# Non-blocking - logs but doesn't prevent progression
fspec add-virtual-hook AUTH-001 post-implementing "notify-script"
\`\`\`

### Common Virtual Hook Patterns

**Quality gates**:
\`\`\`bash
fspec add-virtual-hook AUTH-001 post-implementing "<test-command>" --blocking
fspec add-virtual-hook AUTH-001 pre-validating "eslint" --git-context --blocking
\`\`\`

**Other patterns**: Multiple stacked checks, copy hooks between work units. See \`fspec add-virtual-hook --help\` for more examples.

### Cleanup After Completion

When a work unit reaches "done" status, AI agents should ask whether to keep or remove virtual hooks:

\`\`\`bash
# AI asks user after work unit marked done:
# "AUTH-001 is now complete. Keep or remove virtual hooks?"

# User chooses "remove":
fspec clear-virtual-hooks AUTH-001
✓ Cleared 3 virtual hook(s) from AUTH-001
\`\`\`

**Why Remove?**
- Completed work doesn't need ephemeral checks
- Reduces noise in work-units.json
- Cleans up generated script files in \`.virtual/\`

### Script File Management

Git context hooks generate script files automatically:

- **Location**: \`spec/hooks/.virtual/<work-unit-id>-<hook-name>.sh\`
- **Generation**: Automatic when using \`--git-context\`
- **Cleanup**: Automatic when removing hooks with \`remove-virtual-hook\` or \`clear-virtual-hooks\`
- **Lifecycle**: Scripts created on hook add, deleted on hook remove

**Important**: Do NOT manually edit generated script files. They are regenerated on every hook modification.

### System-Reminders for Blocking Hook Failures

When a blocking virtual hook fails, stderr is wrapped in ${agent.supportsSystemReminders ? '<system-reminder> tags' : 'a warning block'}:

\`\`\`xml
${blockingHookExample}
\`\`\`

This makes failures **highly visible** to AI agents in {{AGENT_NAME}}, ensuring they address issues before continuing.

### Virtual Hooks vs Global Hooks: When to Use Each

**Use Virtual Hooks When:**
- ✅ Quality check applies to ONE work unit only
- ✅ Hook is temporary (remove when work done)
- ✅ Different work units need different checks
- ✅ Experimenting with new quality gates

**Use Global Hooks When:**
- ✅ Quality check applies to ALL work units
- ✅ Hook is permanent (project standard)
- ✅ Enforcing team-wide practices
- ✅ Pre-commit, pre-push, CI/CD integration

**Example Decision Tree:**

\`\`\`
Question: Should this be a virtual or global hook?

Is this check needed for ALL work units?
  → Yes: Use global hook (spec/fspec-hooks.json)
  → No: Continue

Is this check permanent (project standard)?
  → Yes: Use global hook
  → No: Continue

Is this check specific to ONE work unit or story?
  → Yes: Use virtual hook
  → No: Reconsider if you need a hook
\`\`\`

### Best Practices for Virtual Hooks

✅ **DO**:
- Add virtual hooks during specifying/testing phases (plan quality gates early)
- Use \`--blocking\` for critical checks (linting, type checking, tests)
- Use \`--git-context\` for file-specific commands (eslint, prettier)
- Remove virtual hooks when work reaches "done"
- Copy hooks to related work units using \`copy-virtual-hooks\`

❌ **DON'T**:
- Skip removal when work is complete (causes clutter)
- Use virtual hooks for permanent project standards (use global hooks)
- Manually edit generated script files (they're auto-generated)
- Forget to use \`--help\` for comprehensive command documentation

### Troubleshooting Virtual Hooks

**Common Issues:**

1. **Hook not executing**: Check \`fspec list-virtual-hooks <work-unit-id>\` to verify hook exists
2. **Command not found**: Ensure command exists in PATH or use full path
3. **Git context failing**: Verify \`jq\` is installed (required for JSON parsing)
4. **Script permission denied**: Generated scripts are automatically made executable (0o755)

**Debugging:**
\`\`\`bash
# List hooks for work unit
fspec list-virtual-hooks AUTH-001

# Check generated script
cat spec/hooks/.virtual/AUTH-001-eslint.sh

# Manually test git context script
echo '{"stagedFiles":["src/auth.ts"],"unstagedFiles":[]}' | \\
  spec/hooks/.virtual/AUTH-001-eslint.sh
\`\`\`

**See Also:**
- Global Hooks: See "Lifecycle Hooks for Workflow Automation" section above
- Help: Run \`fspec add-virtual-hook --help\` for comprehensive usage guide
- Examples: Check \`src/commands/*-virtual-hook-help.ts\` for detailed patterns`;
}
