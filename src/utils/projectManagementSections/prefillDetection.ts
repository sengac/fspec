import type { AgentConfig } from '../agentRegistry';
import { formatSystemReminder } from '../projectManagementTemplate';

export function getPrefillDetectionSection(agent: AgentConfig): string {
  const prefillExample = formatSystemReminder(
    `PREFILL DETECTED in generated feature file.

Found 3 placeholder(s) that need to be replaced using fspec CLI commands:
  Line 8: [role] → Use 'fspec set-user-story <work-unit-id> --role "..." --action "..." --benefit "..."'
  Line 9: [action] → Use 'fspec set-user-story <work-unit-id> --role "..." --action "..." --benefit "..."'
  Line 10: [benefit] → Use 'fspec set-user-story <work-unit-id> --role "..." --action "..." --benefit "..."'

DO NOT use Write or Edit tools to replace these placeholders.
ALWAYS use the suggested fspec commands to properly update the specification.`,
    agent
  );

  return `## Prefill Detection and CLI Enforcement

**CRITICAL**: fspec detects placeholder text in generated feature files and emits system-reminders to guide AI agents to use CLI commands instead of directly editing files.

### What is Prefill Detection?

When fspec generates feature files (via \`create-feature\` or \`generate-scenarios\`), the output may contain placeholder text like:
- \`[role]\`, \`[action]\`, \`[benefit]\` in Background sections
- \`[precondition]\`, \`[expected outcome]\` in scenario steps
- \`TODO:\` markers in architecture notes
- Generic tags like \`@critical\`, \`@component\`

**Instead of using Write/Edit tools to replace these placeholders, AI agents MUST use fspec CLI commands.**

### ${agent.supportsSystemReminders ? 'System-Reminders' : 'Warning Messages'} for Placeholder Detection

When prefill is detected, fspec emits a ${agent.supportsSystemReminders ? '`<system-reminder>`' : 'warning message'} that is:
- **Visible to AI** - Agent sees and processes the ${agent.supportsSystemReminders ? 'reminder' : 'message'}
- **Invisible to users** - Stripped from UI output
- **Actionable** - Contains specific CLI commands to fix the issue

**Example ${agent.supportsSystemReminders ? 'system-reminder' : 'warning message'}:**

\`\`\`xml
${prefillExample}
\`\`\`

### Workflow Blocking

**fspec prevents workflow progression when prefill exists in linked feature files.**

If you try to advance a work unit status (e.g., from \`specifying\` to \`testing\`) while the linked feature file contains placeholder text, the command will **fail with exit code 1**:

\`\`\`bash
$ fspec update-work-unit-status WORK-001 testing
Error: Cannot advance work unit status: linked feature file contains prefill placeholders.

Found 3 placeholder(s):
  Line 8: [role]
  Line 9: [action]
  Line 10: [benefit]

Fix these placeholders before advancing:
  fspec set-user-story WORK-001 --role "user role" --action "user action" --benefit "user benefit"
\`\`\`

**This hard error prevents:**
- Advancing to \`testing\` with incomplete specifications
- Moving to \`implementing\` without proper acceptance criteria
- Marking work as \`done\` when feature files have TODO markers

### Setting User Story During Example Mapping

**The proper workflow to avoid prefill in Background sections:**

1. **During Example Mapping**, capture the user story fields:
   \`\`\`bash
   fspec set-user-story WORK-001 \\
     --role "developer using fspec" \\
     --action "validate feature files automatically" \\
     --benefit "I catch syntax errors before committing"
   \`\`\`

2. **Generate scenarios** from the example map:
   \`\`\`bash
   fspec generate-scenarios WORK-001
   \`\`\`

3. **The generated feature file** will have a complete Background section (NO placeholders):
   \`\`\`gherkin
   Background: User Story
     As a developer using fspec
     I want to validate feature files automatically
     So that I catch syntax errors before committing
   \`\`\`

### Fixing Placeholder Steps

For placeholder steps in scenarios (\`[precondition]\`, \`[expected outcome]\`), use:

\`\`\`bash
# Replace a step with proper Given/When/Then text
fspec update-step <feature-name> "<scenario-name>" "[precondition]" \\
  --text "I have a feature file with valid Gherkin syntax"
\`\`\`

### Fixing TODO Architecture Notes

For \`TODO:\` markers in architecture notes:

\`\`\`bash
# Add architecture documentation
fspec add-architecture <feature-name> "Uses @cucumber/gherkin for parsing. Supports all Gherkin keywords."
\`\`\`

### Fixing Generic Tags

For placeholder tags like \`@critical\`, \`@component\`:

\`\`\`bash
# Add proper tags to feature file
fspec add-tag-to-feature spec/features/my-feature.feature @high
fspec add-tag-to-feature spec/features/my-feature.feature @cli
fspec add-tag-to-feature spec/features/my-feature.feature @validation
\`\`\`

### Summary: Prefill Workflow

1. **Create work unit** and move to \`specifying\`
2. **Use Example Mapping** to capture user story, rules, examples
3. **Set user story** using \`fspec set-user-story\` command
4. **Generate scenarios** using \`fspec generate-scenarios\`
5. **Fix any remaining placeholders** using CLI commands (NOT Write/Edit)
6. **Advance status** only after all prefill is removed

**This workflow ensures:**
- ✅ Proper use of fspec CLI commands
- ✅ Complete specifications without placeholders
- ✅ No direct file editing that bypasses validation
- ✅ Clear system-reminders guiding AI agents`;
}
