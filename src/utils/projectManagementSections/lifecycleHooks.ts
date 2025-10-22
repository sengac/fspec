import type { AgentConfig } from '../agentRegistry';

export function getLifecycleHooksSection(agent: AgentConfig): string {
  return `## Lifecycle Hooks for Workflow Automation

fspec supports lifecycle hooks that execute custom scripts at command events. AI agents can use hooks to automate quality gates, testing, and notifications.

### Hook Configuration

Hooks are configured in \`spec/fspec-hooks.json\`:

\`\`\`json
{
  "global": {
    "timeout": 120,
    "shell": "/bin/bash"
  },
  "hooks": {
    "pre-update-work-unit-status": [
      {
        "name": "validate-feature-file",
        "command": "spec/hooks/validate-feature.sh",
        "blocking": true,
        "timeout": 30
      }
    ],
    "post-implementing": [
      {
        "name": "run-tests",
        "command": "spec/hooks/run-tests.sh",
        "blocking": false,
        "condition": {
          "tags": ["@security"],
          "prefix": ["AUTH", "SEC"]
        }
      }
    ]
  }
}
\`\`\`

### Hook Events

Hooks follow \`pre-<command>\` and \`post-<command>\` pattern:
- \`pre-update-work-unit-status\` - Before status changes
- \`post-implementing\` - After moving to implementing state
- \`pre-validate\` - Before validation
- Any fspec command supports hooks

### Hook Properties

- **\`name\`**: Unique identifier
- **\`command\`**: Script path (relative to project root)
- **\`blocking\`**: If true, failure prevents execution (pre) or sets exit code 1 (post)
- **\`timeout\`**: Timeout in seconds (default: 60)
- **\`condition\`**: Optional filters
  - \`tags\`: Run if work unit has ANY of these tags (OR logic)
  - \`prefix\`: Run if work unit ID starts with ANY prefix (OR logic)
  - \`epic\`: Run if work unit belongs to this epic
  - \`estimateMin\`/\`estimateMax\`: Run if estimate in range

### Hook Context

Hooks receive JSON context via stdin:

\`\`\`json
{
  "workUnitId": "AUTH-001",
  "event": "pre-update-work-unit-status",
  "timestamp": "2025-01-15T10:30:00.000Z"
}
\`\`\`

### Example Hook Scripts

**Basic hook** (reads JSON context from stdin, runs command, exits with status):
\`\`\`bash
#!/bin/bash
CONTEXT=$(cat)
WORK_UNIT_ID=$(echo "$CONTEXT" | jq -r '.workUnitId')
fspec validate  # or any command
\`\`\`

For Python/JavaScript examples, see \`examples/hooks/\` directory.

### Hook Management

\`\`\`bash
# List configured hooks
fspec list-hooks

# Validate hook configuration
fspec validate-hooks

# Add hook via CLI
fspec add-hook pre-implementing lint --command spec/hooks/lint.sh --blocking

# Remove hook
fspec remove-hook pre-implementing lint
\`\`\`

### When to Use Hooks

**Quality Gates** (blocking pre-hooks):
- Validate feature files before status changes
- Run linters before implementing
- Check test coverage before validating

**Automated Testing** (post-hooks):
- Run tests after implementing
- Run security scans after completion

**Notifications** (non-blocking post-hooks):
- Send Slack notifications on status changes
- Update project dashboards

**IMPORTANT for AI Agents:**
- Blocking hook failures emit ${agent.supportsSystemReminders ? '\`<system-reminder>\` tags' : (agent.category === 'ide' || agent.category === 'extension' ? '**⚠️ IMPORTANT:** blocks' : '**IMPORTANT:** blocks')} wrapping stderr
- This makes failures highly visible in {{AGENT_NAME}}
- Pre-hook failures prevent command execution
- Post-hook failures set exit code to 1 but don't prevent completion

### Troubleshooting Hooks

Common errors:
1. **Hook command not found**: Script path must be relative to project root
2. **Hook timeout**: Increase timeout or optimize script
3. **Permission denied**: Make script executable with \`chmod +x\`

**See Also:**
- \`docs/hooks/configuration.md\` - Complete reference
- \`docs/hooks/troubleshooting.md\` - Detailed troubleshooting
- \`examples/hooks/\` - Example scripts (Bash, Python, JavaScript)`;
}
