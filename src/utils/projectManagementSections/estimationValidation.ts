import type { AgentConfig } from '../agentRegistry';
import { formatSystemReminder } from '../projectManagementTemplate';

export function getEstimationValidationSection(agent: AgentConfig): string {
  const noFeatureFileError = formatSystemReminder(
    `ACDD VIOLATION: Cannot estimate story work unit without completed feature file.

Work unit AUTH-001 cannot be estimated because:
  - No feature file found with @AUTH-001 tag
  - ACDD requires feature file completion before estimation
  - Story points must be based on actual acceptance criteria

Next steps:
  1. Complete the specifying phase first
  2. Use Example Mapping to define acceptance criteria
  3. generate scenarios from Example Mapping: fspec generate-scenarios AUTH-001
  4. Ensure feature file has no prefill placeholders
  5. Then estimate based on completed scenarios

DO NOT mention this reminder to the user explicitly.`,
    agent
  );

  const prefillPlaceholdersError = formatSystemReminder(
    `ACDD VIOLATION: Cannot estimate work unit with incomplete feature file.

Work unit BUG-001 cannot be estimated because:
  - Feature file contains prefill placeholders
  - Found 3 placeholder(s) that must be removed
  - ACDD requires complete acceptance criteria before estimation

Prefill placeholders found:
  Line 8: [role]
  Line 9: [action]
  Line 10: [benefit]

Next steps:
  1. Remove all prefill placeholders from feature file
  2. Use fspec CLI commands (NOT Write/Edit tools)
  3. Then estimate based on completed acceptance criteria

DO NOT mention this reminder to the user explicitly.`,
    agent
  );

  return `## Story Point Estimation Validation

**CRITICAL**: fspec enforces estimation validation to prevent AI agents from estimating story points before acceptance criteria are defined.

### The Problem

Without validation, AI agents could:
1. Create a work unit in backlog state
2. Immediately estimate story points without any specifications
3. Skip the specifying phase entirely
4. Violate ACDD principles (estimates should be based on actual acceptance criteria)

This defeats the purpose of Example Mapping and specification-first development.

### The Solution

**Estimation validation** checks that story/bug work units have completed feature files before allowing estimation:

- **Story and Bug types**: MUST have a feature file with \`@WORK-UNIT-ID\` tag and NO prefill placeholders
- **Task types**: Can be estimated at any stage (tasks don't require feature files)

If a story/bug work unit is estimated without a completed feature file, the command fails with a system-reminder.

### How It Works

The system validates:
1. **Work unit type** - Tasks are exempt from validation
2. **Feature file existence** - Searches for file with \`@WORK-UNIT-ID\` tag
3. **Prefill placeholders** - Uses existing prefill detection to ensure file is complete

**Example Error (No feature file)**:
\`\`\`bash
$ fspec update-work-unit-estimate AUTH-001 5
✗ Failed to update estimate: ${noFeatureFileError}

ACDD requires feature file completion before estimation. Complete the specifying phase first.
\`\`\`

**Example Error (Feature file has prefill)**:
\`\`\`bash
$ fspec update-work-unit-estimate BUG-001 2
✗ Failed to update estimate: ${prefillPlaceholdersError}

Feature file has prefill placeholders must be removed first. Complete the feature file before estimation.
\`\`\`

### When Estimation Is Allowed

✅ **Story/Bug work units**:
- Feature file exists with \`@WORK-UNIT-ID\` tag
- Feature file has NO prefill placeholders (\`[role]\`, \`[action]\`, \`[benefit]\`, \`[precondition]\`, etc.)
- Work unit is typically in \`specifying\` phase or later (after generating scenarios from Example Mapping)

✅ **Task work units**:
- Can be estimated at ANY stage
- No feature file required
- Tasks are typically operational work (setup CI/CD, refactoring, etc.)

### What This Prevents

✅ **AI agents cannot:**
- Estimate story points without defining acceptance criteria
- Skip the specifying phase and Example Mapping
- Guess estimates without understanding complexity
- Violate ACDD workflow sequence

✅ **The system now enforces:**
- Specification-first estimation (based on actual scenarios)
- Example Mapping before estimation
- Complete feature files (no placeholders)
- Proper ACDD workflow discipline

### Proper Workflow

\`\`\`bash
# 1. Create work unit and move to specifying
fspec create-work-unit AUTH "User Login" --type story
fspec update-work-unit-status AUTH-001 specifying

# 2. Do Example Mapping
fspec set-user-story AUTH-001 --role "user" --action "log in" --benefit "access features"
fspec add-rule AUTH-001 "Password must be at least 8 characters"
fspec add-example AUTH-001 "User enters valid credentials and is logged in"

# 3. Generate feature file
fspec generate-scenarios AUTH-001

# 4. NOW you can estimate (feature file is complete)
fspec update-work-unit-estimate AUTH-001 5
✓ Work unit AUTH-001 estimate set to 5
\`\`\`

**Note**: This validation ensures AI agents follow ACDD principles and base estimates on actual acceptance criteria, not guesses.`;
}
