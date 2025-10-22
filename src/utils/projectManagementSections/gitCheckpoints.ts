import type { AgentConfig } from '../agentRegistry';
import { formatSystemReminder } from '../projectManagementTemplate';

export function getGitCheckpointsSection(agent: AgentConfig): string {
  const conflictExample = formatSystemReminder(
    `CHECKPOINT CONFLICT RESOLUTION REQUIRED

Restored checkpoint "baseline" for AUTH-001 caused merge conflicts.
You must resolve these conflicts using Read and Edit tools.

Conflicted files:
  - src/auth/login.ts
  - src/auth/session.ts

Next steps:
  1. Read each conflicted file to see CONFLICT markers
  2. Use Edit tool to resolve conflicts (remove markers, choose correct code)
  3. Run tests to validate: npm test
  4. Mark resolution complete when tests pass

DO NOT mention this reminder to the user explicitly.`,
    agent
  );

  return `## Git Checkpoints for Safe Experimentation

fspec provides an intelligent checkpoint system that uses **isomorphic-git's \`git.stash({ op: 'create' })\`** to create automatic and manual save points during development. Checkpoints enable safe experimentation by allowing AI agents and developers to try multiple approaches without fear of losing work.

### What Are Checkpoints?

**Checkpoints** are git stash-based snapshots of all file changes (including untracked files) at specific points in time. They:

- **Capture complete state** - All modified and untracked files (respecting .gitignore)
- **Persist until deleted** - No automatic expiration or cleanup
- **Enable re-restoration** - Same checkpoint can be restored multiple times
- **Support experiments** - Create baseline, try approach A, restore baseline, try approach B
- **Integrate with workflow** - Automatic checkpoints created on status transitions

**Checkpoint Types:**

| Type | Trigger | Naming Pattern | Visual Indicator |
|------|---------|----------------|------------------|
| Automatic | Status transition | \`{work-unit-id}-auto-{state}\` | 🤖 |
| Manual | Explicit command | User-provided name | 📌 |

### Automatic Checkpoints

**When created:**
- Before every workflow state transition (except from \`backlog\`)
- Only if working directory has uncommitted changes

**Example:**
\`\`\`bash
# You have uncommitted changes in AUTH-001
$ fspec update-work-unit-status AUTH-001 implementing
🤖 Auto-checkpoint: "AUTH-001-auto-testing" created before transition
✓ Work unit AUTH-001 status updated to implementing
\`\`\`

**Why automatic checkpoints matter:**
- Recovery from mistakes during implementation
- Rollback if new status proves premature
- Safety net for AI agents making rapid changes

### Manual Checkpoints

**Create checkpoints for:**
- Experimentation with multiple approaches
- Before risky refactoring or major changes
- Creating named baselines for comparison
- Saving progress before switching contexts

#### Creating Checkpoints

\`\`\`bash
# Create checkpoint before trying new approach
fspec checkpoint AUTH-001 baseline

# Create checkpoint with descriptive name
fspec checkpoint UI-002 before-refactor

# Create checkpoint for experiment
fspec checkpoint BUG-003 working-version
\`\`\`

#### Listing Checkpoints

\`\`\`bash
# View all checkpoints for work unit
$ fspec list-checkpoints AUTH-001

Checkpoints for AUTH-001:

📌  before-refactor (manual)
   Created: 2025-10-21T14:30:00.000Z

📌  baseline (manual)
   Created: 2025-10-21T13:15:00.000Z

🤖  AUTH-001-auto-testing (automatic)
   Created: 2025-10-21T10:00:00.000Z
\`\`\`

#### Restoring Checkpoints

\`\`\`bash
# Restore to baseline
fspec restore-checkpoint AUTH-001 baseline

# Restore after failed experiment
fspec restore-checkpoint UI-002 before-refactor
\`\`\`

**Restoration behavior:**
- Uses **manual file operations** (reads checkpoint files with \`git.readBlob()\`, writes with \`fs.writeFile()\`)
- Detects conflicts **before** restoration (byte-by-byte comparison, does NOT overwrite if conflicts found)
- Preserves checkpoint for re-restoration (same checkpoint can be restored multiple times)
- Detects working directory status (prompts if dirty)
- Handles conflicts with AI-assisted resolution via ${agent.supportsSystemReminders ? 'system-reminders' : 'warning messages'}

#### Cleaning Up Checkpoints

\`\`\`bash
# Keep last 5 checkpoints, delete older ones
$ fspec cleanup-checkpoints AUTH-001 --keep-last 5

Cleaning up checkpoints for AUTH-001 (keeping last 5)...

Deleted 7 checkpoint(s):
  - experiment-1 (2025-10-20T10:00:00.000Z)
  - experiment-2 (2025-10-20T11:00:00.000Z)
  ...

Preserved 5 checkpoint(s):
  - current-state (2025-10-21T14:30:00.000Z)
  - working-version (2025-10-21T13:15:00.000Z)
  ...

✓ Cleanup complete: 7 deleted, 5 preserved
\`\`\`

### Workflow Patterns

**Example: Multiple Experiments from Baseline**
\`\`\`bash
fspec checkpoint AUTH-001 baseline          # Create baseline
# Try approach A... doesn't work
fspec restore-checkpoint AUTH-001 baseline  # Restore baseline
# Try approach B... works!
\`\`\`

**Other patterns**: Before risky refactoring, experimentation with cleanup. See \`fspec checkpoint --help\` for more.

### Dirty Working Directory Handling

When restoring with uncommitted changes, fspec prompts with 3 options: commit first (safest), stash and restore, or force merge. Choose based on risk tolerance and whether changes should be preserved.

### Conflict Resolution

When checkpoint restoration causes conflicts, AI receives a ${agent.supportsSystemReminders ? '\`<system-reminder>\`' : 'warning message'}:

\`\`\`xml
${conflictExample}
\`\`\`

**Resolution workflow:**
1. AI uses \`Read\` tool to examine conflicted files
2. AI identifies conflict markers: \`<<<<<<<\`, \`=======\`, \`>>>>>>>\`
3. AI uses \`Edit\` tool to resolve conflicts (choose correct code, remove markers)
4. AI runs tests: \`npm test\` (or appropriate test command)
5. If tests pass, resolution complete
6. If tests fail, continue editing until tests pass

### Best Practices for AI Agents

✅ **DO**:
- Create checkpoints before experimental changes or risky refactoring
- Use descriptive checkpoint names that explain WHY you're saving
- List checkpoints before restoring to see what's available
- Clean up old checkpoints periodically: \`fspec cleanup-checkpoints <id> --keep-last 5\`
- Run tests after conflict resolution (ALWAYS)
- Create "baseline" checkpoint before multiple experiments

❌ **DON'T**:
- Skip checkpoint creation thinking "I won't need it"
- Use generic names like "temp", "test", "checkpoint1"
- Forget to run tests after conflict resolution
- Let checkpoints accumulate indefinitely (cleanup regularly)
- Assume automatic checkpoints are enough (manual checkpoints give you control)

**See Also:**
- Help: Run \`fspec checkpoint --help\` for manual checkpoint creation
- Help: Run \`fspec restore-checkpoint --help\` for restoration with conflict handling
- Help: Run \`fspec list-checkpoints --help\` for viewing checkpoint history
- Help: Run \`fspec cleanup-checkpoints --help\` for retention management`;
}
