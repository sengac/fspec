# Critical Review: Incomplete Implementation of INIT-009 and INIT-010

**Date:** 2025-10-22
**Reviewer:** Claude (AI Code Review)
**Scope:** Agent switching and remove-init-files features
**Status:** 🚨 CRITICAL - NOT READY FOR COMMIT

---

## Executive Summary

The recently staged changes for INIT-009 (remove-init-files) and INIT-010 (agent switching) are **incomplete implementations**. While the features have:
- ✅ Complete Gherkin specifications
- ✅ Passing tests
- ✅ Command registration
- ✅ Help documentation

They **fail to implement** core acceptance criteria because:
- ❌ Interactive prompts are missing
- ❌ Agent detection logic not integrated into action handlers
- ❌ Tests bypass incomplete implementations by calling internal functions directly

**Recommendation:** DO NOT COMMIT. Either complete the implementation or revert the changes.

---

## Issue #1: Missing Interactive Prompt in `remove-init-files`

### Feature Requirement
```gherkin
When I run 'fspec remove-init-files'
When the interactive prompt asks 'Keep spec/fspec-config.json?'
When I select 'No'
```

### Current Implementation
**File:** `src/commands/remove-init-files.ts:96-97`

```typescript
// TODO: Add interactive prompt for keepConfig
// For now, hardcode to false (remove everything)
const options: RemoveOptions = { keepConfig: false };
```

### Problems
1. **No interactive prompt component exists**
   - Feature requires Ink/React prompt similar to `AgentSelector`
   - User should be asked: "Keep spec/fspec-config.json? (Yes/No)"
   - Current code hardcodes `keepConfig: false` (always removes config)

2. **Acceptance criteria violated**
   - Scenario "Keep config and remove only agent files" cannot work
   - User has no choice - config is always removed

3. **Tests bypass the issue**
   - Tests call `removeInitFiles(testDir, { keepConfig: true/false })` directly
   - Tests never exercise the action handler with hardcoded `false`
   - Tests pass but feature is broken for real users

### What's Missing
1. Create `src/components/KeepConfigPrompt.tsx`:
   ```typescript
   interface KeepConfigPromptProps {
     onSubmit: (keepConfig: boolean) => void;
   }

   export function KeepConfigPrompt({ onSubmit }: KeepConfigPromptProps) {
     // Interactive Yes/No prompt
   }
   ```

2. Update action handler:
   ```typescript
   .action(async () => {
     const cwd = process.cwd();

     // Show interactive prompt
     const keepConfig = await new Promise<boolean>(resolve => {
       const { waitUntilExit } = render(
         React.createElement(KeepConfigPrompt, {
           onSubmit: (keep) => resolve(keep)
         })
       );
       void waitUntilExit();
     });

     await removeInitFiles(cwd, { keepConfig });
   });
   ```

3. Update success message to show what was removed:
   ```typescript
   console.log(chalk.green('✓ Successfully removed fspec init files'));
   console.log('  - Removed spec/CLAUDE.md');
   console.log('  - Removed .claude/commands/fspec.md');
   if (!keepConfig) {
     console.log('  - Removed spec/fspec-config.json');
   }
   ```

---

## Issue #2: Missing Agent Switching Logic in `init`

### Feature Requirement
```gherkin
Given I have fspec initialized with Claude agent
When I run 'fspec init --agent=cursor'
Then the prompt should ask 'Switch from Claude to Cursor?'
When I select 'Switch to Cursor'
Then Claude files should be removed
Then Cursor files should be installed
```

### Current Implementation
**File:** `src/commands/init.ts:205-271` (action handler)

```typescript
// Line 47-50: Checks shouldSwitch but never sets it
if (options?.shouldSwitch === false) {
  throw new Error('Agent switch cancelled by user');
}
```

### Problems
1. **No agent detection before installation**
   - Action handler doesn't check for existing agent
   - Directly proceeds to `installAgents()` without comparison
   - `shouldSwitch` is checked but never set

2. **No interactive prompt for switching**
   - Feature requires: "Switch from Claude to Cursor?"
   - No prompt component exists
   - No logic to show prompt when agents differ

3. **Tests bypass the issue**
   - Tests call `installAgents(testDir, ['cursor'], { shouldSwitch: true })` directly
   - Tests mock the `shouldSwitch` option that action handler never sets
   - Real CLI invocation would fail silently

### What's Missing
1. Add agent detection at start of action handler:
   ```typescript
   .action(async (options: { agent: string[] }) => {
     const cwd = process.cwd();
     let agentIds: string[];

     // Detect existing agent
     const existingAgent = await detectInstalledAgent(cwd);

     // ... rest of logic
   });
   ```

2. Create comparison and prompt logic:
   ```typescript
   // After agent selection (CLI or interactive)
   if (existingAgent && existingAgent !== agentIds[0]) {
     // Show switch prompt
     const shouldSwitch = await promptAgentSwitch(
       existingAgent,
       agentIds[0]
     );

     if (!shouldSwitch) {
       console.log(chalk.yellow('Agent switch cancelled'));
       process.exit(0);
     }
   }
   ```

3. Create `src/components/AgentSwitchPrompt.tsx`:
   ```typescript
   interface AgentSwitchPromptProps {
     fromAgent: string;
     toAgent: string;
     onSubmit: (shouldSwitch: boolean) => void;
   }

   export function AgentSwitchPrompt({
     fromAgent,
     toAgent,
     onSubmit
   }: AgentSwitchPromptProps) {
     // Prompt: "Switch from Claude to Cursor?"
     // Options: "Switch to Cursor" | "Cancel"
   }
   ```

4. Pass `shouldSwitch` to `installAgents()`:
   ```typescript
   await installAgents(cwd, agentIds, { shouldSwitch });
   ```

5. Helper function:
   ```typescript
   async function detectInstalledAgent(cwd: string): Promise<string | null> {
     // Try reading spec/fspec-config.json first
     const configPath = join(cwd, 'spec', 'fspec-config.json');
     if (existsSync(configPath)) {
       const config = JSON.parse(await readFile(configPath, 'utf-8'));
       if (config.agent) return config.agent;
     }

     // Fall back to file detection
     const detected = await detectAgents(cwd);
     return detected.length > 0 ? detected[0] : null;
   }
   ```

---

## Issue #3: Duplicate Agent Config Write

### Problem
**File:** `src/commands/init.ts`

Agent config is written **twice**:
```typescript
// Line 64-66: Written in installAgents()
if (agentIds.length > 0) {
  writeAgentConfig(cwd, agentIds[0]);
}

// Line 248-250: Written AGAIN in action handler
if (agentIds.length > 0) {
  writeAgentConfig(cwd, agentIds[0]);
}
```

### Issue
- Violates DRY principle
- Redundant filesystem operation
- Code smell indicating rushed implementation

### Fix
Remove one of the calls (preferably keep it in `installAgents()` since that's the logical place).

---

## Issue #4: Misleading Help Documentation

### Problem
**File:** `src/commands/init-help.ts:46`

```typescript
'Auto-detects existing agent installations and prompts to switch when different agent requested',
```

**Reality:** This functionality does not exist in the code.

### Also Misleading
**File:** `src/commands/remove-init-files-help.ts:17`

```typescript
output:
  '✓ Successfully removed fspec init files\n' +
  '  - Removed spec/CLAUDE.md\n' +
  '  - Removed .claude/commands/fspec.md\n' +
  '  - Removed spec/fspec-config.json',
```

**Reality:** Actual output is just: `"✓ Successfully removed fspec init files"`

### Fix
Either:
1. Implement the described functionality, OR
2. Update help docs to match actual behavior

---

## Issue #5: Test Coverage Gap

### Problem
Tests pass by bypassing incomplete action handlers.

**Evidence:**

**remove-init-files.test.ts:52**
```typescript
await removeInitFiles(testDir, { keepConfig: false });
```
- Calls internal function directly
- Bypasses action handler with hardcoded `keepConfig: false`
- Doesn't test actual CLI behavior

**init-agent-switching.test.ts:50**
```typescript
await installAgents(testDir, ['cursor'], { shouldSwitch: true });
```
- Calls internal function directly
- Mocks `shouldSwitch` option that action handler never sets
- Doesn't test actual CLI invocation

### Issue
Tests give false confidence that features work, when they don't.

### Fix
Add E2E tests that:
1. Spawn actual CLI process
2. Simulate user input to prompts
3. Verify output matches expectations

Example:
```typescript
import { spawn } from 'child_process';

describe('E2E: remove-init-files', () => {
  it('should prompt for keepConfig and remove based on user choice', async () => {
    const proc = spawn('./dist/index.js', ['remove-init-files'], {
      cwd: testDir
    });

    // Simulate user pressing 'n' for No
    proc.stdin.write('n\n');

    // Verify output
    const output = await waitForOutput(proc);
    expect(output).toContain('spec/fspec-config.json should be removed');
  });
});
```

---

## What IS Properly Wired ✅

1. **Command Registration**
   - Both commands imported in `src/index.ts:76,225`
   - Properly registered with Commander.js

2. **Help System**
   - Documented in `src/help.ts:1101-1109` (remove-init-files)
   - Documented in `src/commands/init-help.ts` and `remove-init-files-help.ts`

3. **Feature Files**
   - Complete Gherkin specs with Background, Scenarios, and Example Mapping
   - `spec/features/remove-initialization-files.feature`
   - `spec/features/agent-switching-prompt-in-fspec-init.feature`

4. **Coverage Files**
   - `.feature.coverage` files created (though incomplete)

5. **Core Logic**
   - `removeAgentFiles()` works correctly
   - `removeOtherAgentFiles()` works correctly
   - `detectInstalledAgent()` exists and works

6. **Documentation Updates**
   - README.md updated
   - docs/getting-started.md updated
   - docs/user-guide.md updated

---

## Required Work to Complete Features

### For `remove-init-files` (INIT-009)
- [ ] Create `src/components/KeepConfigPrompt.tsx`
- [ ] Add prompt logic to action handler
- [ ] Update success message to show deleted files
- [ ] Add E2E test for CLI invocation
- [ ] Update help docs to match actual behavior

**Estimated:** 2-3 story points

### For `init` agent switching (INIT-010)
- [ ] Add `detectInstalledAgent()` call in action handler
- [ ] Create `src/components/AgentSwitchPrompt.tsx`
- [ ] Add comparison and prompt logic
- [ ] Remove duplicate `writeAgentConfig()` call
- [ ] Add E2E tests for CLI invocation
- [ ] Fix misleading help documentation

**Estimated:** 3-5 story points

---

## Recommendations

### Option 1: Complete Before Committing ⭐ RECOMMENDED
1. Implement missing interactive prompts
2. Wire up agent detection in action handlers
3. Add E2E tests
4. Update help docs
5. Then commit complete features

**Pros:**
- Features work as specified
- No technical debt
- Tests reflect reality

**Cons:**
- Delays commit
- Requires additional work now

### Option 2: Revert and Track Separately
1. Unstage all changes
2. Create BUG work unit: "Complete INIT-009 implementation"
3. Create BUG work unit: "Complete INIT-010 implementation"
4. Track as future work

**Pros:**
- Clean commit history
- Proper ACDD workflow
- Clear separation of concerns

**Cons:**
- Loses work already done
- Requires re-specification

### Option 3: Commit with Warnings (NOT RECOMMENDED)
1. Add TODO comments to incomplete code
2. Update help docs to say "Coming soon"
3. Commit as-is

**Pros:**
- Fastest path forward

**Cons:**
- Creates technical debt
- Misleading to users
- Violates ACDD principles
- Tests give false confidence

---

## Impact Analysis

### User Impact
If committed as-is:
- Users running `fspec remove-init-files` will **always** have config deleted (no choice)
- Users running `fspec init --agent=cursor` when Claude is installed will **not** be prompted (silent switch)
- Help docs promise features that don't exist

### Technical Debt
- TODO comments in production code
- Tests that don't test actual CLI behavior
- Incomplete acceptance criteria implementation
- Misleading documentation

### ACDD Violation
This violates core ACDD principles:
1. ❌ Tests pass but features incomplete (should fail until implemented)
2. ❌ Acceptance criteria not met
3. ❌ Feature files describe unimplemented behavior

---

## Conclusion

**DO NOT COMMIT THIS CODE WITHOUT COMPLETING THE IMPLEMENTATION.**

The staged changes represent incomplete work that:
- Violates acceptance criteria
- Misleads users with help documentation
- Passes tests through implementation bypassing
- Creates technical debt

**Recommended Action:** Choose Option 1 (complete before committing) or Option 2 (revert and track).

---

## Appendix: File Inventory

### Modified Files
- ✅ `README.md` - Documentation updated
- ✅ `docs/getting-started.md` - Documentation updated
- ✅ `docs/user-guide.md` - Documentation updated
- ⚠️ `src/commands/init.ts` - Missing detection/prompt logic
- ⚠️ `src/commands/init-help.ts` - Misleading documentation
- ⚠️ `src/commands/remove-init-files.ts` - Missing interactive prompt
- ⚠️ `src/commands/remove-init-files-help.ts` - Misleading output example
- ✅ `src/help.ts` - Command registration in help
- ✅ `src/index.ts` - Command registration

### New Files
- ✅ `spec/features/agent-switching-prompt-in-fspec-init.feature`
- ✅ `spec/features/agent-switching-prompt-in-fspec-init.feature.coverage`
- ✅ `spec/features/remove-initialization-files.feature`
- ✅ `spec/features/remove-initialization-files.feature.coverage`
- ⚠️ `src/commands/__tests__/init-agent-switching.test.ts` - Bypasses action handler
- ⚠️ `src/commands/__tests__/remove-init-files.test.ts` - Bypasses action handler

### Work Units Modified
- ✅ `spec/work-units.json` - INIT-009 and INIT-010 tracked

**Total Files:** 16
**Properly Implemented:** 8 (50%)
**Incomplete:** 8 (50%)
