# Git Checkpoints: Safe Experimentation

## Overview

**Git checkpoints** let AI agents try multiple approaches safely by creating save points in git stash. When approach A doesn't work, restore the baseline and try approach B without losing work.

**Key features:**
- Automatic checkpoints before workflow transitions
- Manual checkpoints for experimentation
- Re-restorable (same checkpoint multiple times)
- Conflict resolution assistance
- No expiration (persist until explicitly deleted)

---

## How It Works

Checkpoints use git stash under the hood to save:
- All modified files (staged and unstaged)
- All untracked files (respecting .gitignore)
- Complete working directory state

**Stash format:** `fspec-checkpoint:{workUnitId}:{checkpointName}:{timestamp}`

---

## Automatic Checkpoints

**When created:**
- Before every workflow state transition (except from backlog)
- Only if working directory has uncommitted changes

**Naming pattern:** `{work-unit-id}-auto-{previous-state}`

**Example:**
```
You: /fspec Move AUTH-001 to implementing

AI: "I'll update AUTH-001 status to implementing..."
    🤖 Auto-checkpoint: "AUTH-001-auto-testing" created before transition
    ✓ Work unit AUTH-001 status updated to implementing
```

**Why automatic checkpoints matter:**
- Recovery from mistakes during implementation
- Rollback if new status proves premature
- Safety net for rapid AI changes

---

## Manual Checkpoints

### Creating Checkpoints

**AI creates manual checkpoints when:**
1. Experimenting with multiple approaches
2. Before risky refactoring
3. Creating baselines for comparison

**Command (AI runs this):**
```bash
fspec checkpoint AUTH-001 baseline
```

**Example conversation:**
```
You: "Let's try a different approach to authentication"

AI: "I'll create a checkpoint before trying the new approach..."
    📌 Manual checkpoint: "baseline" created for AUTH-001

    Now let me try approach A: JWT with Redis...
    [implements approach A]
    [tests fail]

    That didn't work. Let me restore the baseline and try approach B...

    fspec restore-checkpoint AUTH-001 baseline
    ✓ Restored checkpoint "baseline"

    Now trying approach B: Session-based auth...
    [implements approach B]
    [tests pass]

    Success! Approach B works.
```

### Listing Checkpoints

**AI runs:**
```bash
fspec list-checkpoints AUTH-001
```

**Output:**
```
Checkpoints for AUTH-001:

📌  before-refactor (manual)
   Created: 2025-10-21T14:30:00.000Z

📌  baseline (manual)
   Created: 2025-10-21T13:15:00.000Z

🤖  AUTH-001-auto-testing (automatic)
   Created: 2025-10-21T10:00:00.000Z
```

### Restoring Checkpoints

**AI runs:**
```bash
fspec restore-checkpoint AUTH-001 baseline
```

**Restoration behavior:**
- Uses `git stash apply` (preserves checkpoint for re-restoration)
- Detects working directory status (prompts if dirty)
- Handles conflicts with AI-assisted resolution

---

## Common Workflows

### Pattern 1: Multiple Experiments from Baseline

```
Create baseline → Try approach A → Doesn't work →
Restore baseline → Try approach B → Works!
```

**Example:**
```
You: "Try implementing login with both JWT and session-based auth
     to see which works better"

AI: Creating baseline checkpoint...
    📌 Checkpoint "baseline" created

    Trying JWT approach...
    [implements JWT auth]
    [tests fail - CORS issues]

    JWT has CORS complexity. Restoring baseline...
    ✓ Restored "baseline"

    Trying session-based approach...
    [implements session auth]
    [tests pass]

    Session-based auth works! Continuing with this approach.
```

### Pattern 2: Before Risky Refactoring

```
Create checkpoint → Refactor code → Tests fail →
Restore checkpoint → Try smaller refactoring → Tests pass
```

**Example:**
```
You: "Refactor the authentication middleware to use decorators"

AI: This is a significant refactoring. Creating checkpoint...
    📌 Checkpoint "before-decorator-refactor" created

    Refactoring to decorators...
    [major refactoring]
    [tests fail - decorators broke routing]

    Decorator approach broke routing. Restoring...
    ✓ Restored "before-decorator-refactor"

    Let's try a simpler approach - extract helper functions...
    [smaller refactoring]
    [tests pass]

    Helper function extraction works!
```

### Pattern 3: Experimentation with Cleanup

Try multiple UI implementations, keep the winner, cleanup experiments:

```
You: "Try tabs, accordion, and carousel for the dashboard.
     Keep whichever feels best."

AI: Creating baseline...
    📌 Checkpoint "baseline" created

    Trying tabs...
    📌 Checkpoint "experiment-tabs" created
    [implements tabs]
    "Here's the tabs implementation. What do you think?"

You: "Meh, try accordion"

AI: Restoring baseline...
    ✓ Restored "baseline"
    Trying accordion...
    📌 Checkpoint "experiment-accordion" created
    [implements accordion]
    "Here's accordion. Better?"

You: "Not great. Try carousel"

AI: Restoring baseline...
    ✓ Restored "baseline"
    Trying carousel...
    📌 Checkpoint "experiment-carousel" created
    [implements carousel]
    "Here's carousel!"

You: "Perfect! Go with carousel"

AI: Great! Cleaning up old experiments...
    fspec cleanup-checkpoints AUTH-001 --keep-last 2
    ✓ Deleted 2 checkpoint(s): experiment-tabs, experiment-accordion
    ✓ Kept 2 checkpoint(s): experiment-carousel, baseline
```

---

## Dirty Working Directory Handling

When restoring with uncommitted changes, AI gets prompted:

```
AI: "⚠️  Working directory has uncommitted changes

     Choose how to proceed:
     1. Commit changes first [Low risk]
     2. Stash changes and restore [Medium risk]
     3. Force restore with merge [High risk]"
```

**AI guidance:**
- Choose option 1 when changes are intentional
- Choose option 2 when changes are temporary
- Choose option 3 only when understanding merge conflicts

---

## Conflict Resolution

When restoration causes conflicts, AI receives guidance:

```
<system-reminder>
CHECKPOINT CONFLICT RESOLUTION REQUIRED

Restored checkpoint "baseline" for AUTH-001 caused merge conflicts.
You must resolve these conflicts using Read and Edit tools.

Conflicted files:
  - src/auth/login.ts
  - src/auth/session.ts

Next steps:
  1. Read each conflicted file to see CONFLICT markers
  2. Use Edit tool to resolve conflicts
  3. Run tests to validate: npm test
  4. Mark resolution complete when tests pass
</system-reminder>
```

**AI resolution workflow:**
1. Read conflicted files
2. Identify conflict markers (`<<<<<<<`, `=======`, `>>>>>>>`)
3. Edit files to resolve conflicts
4. Run tests
5. Continue if tests pass

---

## Cleanup Commands

### Keep Last N Checkpoints

**AI runs:**
```bash
fspec cleanup-checkpoints AUTH-001 --keep-last 5
```

**Output:**
```
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
```

---

## Best Practices

✅ **DO:**
- Create checkpoints before experimental changes
- Use descriptive names explaining WHY you're saving
- List checkpoints before restoring
- Clean up old checkpoints periodically
- Run tests after conflict resolution
- Create "baseline" checkpoint before multiple experiments

❌ **DON'T:**
- Skip checkpoint creation thinking "I won't need it"
- Use generic names like "temp", "test", "checkpoint1"
- Forget to run tests after conflict resolution
- Let checkpoints accumulate indefinitely
- Assume automatic checkpoints are enough

---

## Commands Reference

```bash
# Manual checkpoint
fspec checkpoint <work-unit-id> <name>

# List checkpoints
fspec list-checkpoints <work-unit-id>

# Restore checkpoint
fspec restore-checkpoint <work-unit-id> <name>

# Cleanup old checkpoints
fspec cleanup-checkpoints <work-unit-id> --keep-last <N>
```

**Get detailed help:**
```bash
fspec checkpoint --help
fspec restore-checkpoint --help
fspec list-checkpoints --help
fspec cleanup-checkpoints --help
```

---

## Integration with ACDD Workflow

Checkpoints work seamlessly with fspec's Kanban workflow:

**Automatic checkpoints:**
```
backlog → specifying (no checkpoint - no changes yet)
specifying → testing (checkpoint created if changes exist)
testing → implementing (checkpoint created)
implementing → validating (checkpoint created)
validating → done (checkpoint created)
```

**Manual checkpoints:**
- Create during any workflow state
- Useful for experimentation within a state
- Enable trying multiple implementations

**Example:**
```
AUTH-001 in implementing state

📌 baseline (manual - before trying different approaches)
🤖 AUTH-001-auto-testing (automatic - before moving to implementing)
📌 experiment-redis (manual - trying Redis for sessions)
📌 experiment-memory (manual - trying in-memory sessions)
```

---

## Why Checkpoints Matter

**Without checkpoints:**
- AI can't safely experiment
- Failed approaches require manual git cleanup
- No easy way to compare multiple implementations
- Risk of losing working code during refactoring

**With checkpoints:**
- ✅ Try multiple approaches risk-free
- ✅ Instant rollback to working state
- ✅ Compare implementations side-by-side
- ✅ Safety net for aggressive refactoring
- ✅ Encourages experimentation and exploration

**Stop fearing AI mistakes. Start experimenting fearlessly.**
