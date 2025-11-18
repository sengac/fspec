# Removal Plan: compile-research Command

## Reason for Removal

The `compile-research` command contains hardcoded semantic logic that generates a static authentication flowchart diagram based on simple keyword matching. This violates the principle of keeping fspec domain-agnostic and avoiding hardcoded business logic.

### Problematic Code

```typescript
// Lines 64-90 in src/commands/compile-research.ts
if (
  researchContent.toLowerCase().includes('flow') ||
  researchContent.toLowerCase().includes('login') ||
  researchContent.toLowerCase().includes('auth')
) {
  // Always generates the SAME authentication flowchart
  mermaidDiagram = `\`\`\`mermaid
flowchart TB
    Start[Start] --> Login[Login Process]
    Login --> Auth[Authentication]
    Auth --> Success[Success]
    Auth --> Failure[Failure]
\`\`\``;
}
```

**Issues:**
- Hardcoded keywords: "flow", "login", "auth"
- Static diagram template (always authentication flow)
- Not contextually aware
- Would generate wrong diagram for "checkout flow", "payment flow", etc.

## Files to Remove

### 1. Command Implementation
- **File**: `src/commands/compile-research.ts`
- **Action**: Delete entire file

### 2. Help Documentation
- **File**: `src/commands/compile-research-help.ts`
- **Action**: Delete entire file

### 3. CLI Registration
- **File**: `src/index.ts`
- **Action**: Remove import statement
  ```typescript
  // REMOVE THIS:
  import { registerCompileResearchCommand } from './commands/compile-research';
  ```
- **Action**: Remove registration call
  ```typescript
  // REMOVE THIS:
  registerCompileResearchCommand(program);
  ```

### 4. Help Text
- **File**: `src/help.ts`
- **Line**: ~1256-1262
- **Action**: Remove compile-research section from RESEARCH TOOLS
  ```typescript
  // REMOVE THIS SECTION:
  console.log('  ' + chalk.cyan('fspec compile-research <work-unit-id>'));
  console.log(
    '    Description: Compile all research attachments into markdown for AI analysis'
  );
  console.log('    Examples:');
  console.log('      fspec compile-research AUTH-001');
  console.log('');
  ```
- **Action**: Remove reference in Notes section
  ```typescript
  // REMOVE THIS LINE:
  console.log(
    '    - Use compile-research to consolidate findings into single document'
  );
  ```

## Search for Additional References

### Test Files
Search for test files that may reference compile-research:
```bash
grep -r "compile-research" src/commands/__tests__/
grep -r "compileResearch" src/commands/__tests__/
```

### Documentation
Search for any documentation references:
```bash
grep -r "compile-research" spec/
grep -r "compile-research" docs/ 2>/dev/null
grep -r "compile-research" *.md
```

### Bootstrap/Help Output
Check if bootstrap command includes compile-research:
```bash
grep -r "compile-research" src/utils/slashCommandSections/
```

## Validation After Removal

1. **Run validation script**:
   ```bash
   node scripts/validate-command-help.js
   ```
   Expected: Should show 152/152 commands (one less than current 153)

2. **Run tests**:
   ```bash
   npm test
   ```
   Expected: All tests should pass

3. **Check help output**:
   ```bash
   fspec help discovery
   ```
   Expected: No mention of compile-research

4. **Verify command doesn't exist**:
   ```bash
   fspec compile-research --help
   ```
   Expected: Error - unknown command

## Alternative Solution

Instead of compile-research, AI agents can:
1. Read individual research attachments directly
2. Use their own analysis capabilities (ULTRATHINK, etc.)
3. Generate contextually relevant diagrams themselves
4. Combine research findings as needed during Example Mapping

The hardcoded diagram generation should not be fspec's responsibility.
