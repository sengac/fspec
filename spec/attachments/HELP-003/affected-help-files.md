# Help Files Affected by BUG-023

## Bug Description
The COMMON PATTERNS section displays `[object Object]` instead of formatted pattern information.

## Root Cause
Type mismatch between:
- **Interface** (`src/utils/help-formatter.ts:32`): `commonPatterns?: string[];`
- **Actual Usage**: Objects with `{ pattern, example, description }`
- **Formatter** (`src/utils/help-formatter.ts:110`): Displays as string → `[object Object]`

## Affected Files (20+ help files)

All files using object-style `commonPatterns`:

1. src/commands/add-background-help.ts
2. src/commands/add-dependencies-help.ts
3. src/commands/add-diagram-help.ts
4. src/commands/add-hook-help.ts
5. src/commands/add-virtual-hook-help.ts
6. src/commands/audit-coverage-help.ts
7. src/commands/clear-virtual-hooks-help.ts
8. src/commands/copy-virtual-hooks-help.ts
9. src/commands/delete-features-by-tag-help.ts
10. src/commands/delete-scenarios-by-tag-help.ts
11. src/commands/delete-work-unit-help.ts
12. src/commands/dependencies-help.ts
13. src/commands/link-coverage-help.ts
14. src/commands/list-virtual-hooks-help.ts
15. src/commands/prioritize-work-unit-help.ts
16. src/commands/query-bottlenecks-help.ts
17. src/commands/query-orphans-help.ts
18. src/commands/remove-dependency-help.ts
19. src/commands/remove-virtual-hook-help.ts
20. src/commands/show-coverage-help.ts

## Verification Commands

Test these commands to see `[object Object]` output:

```bash
fspec add-virtual-hook --help | grep -A 10 "COMMON PATTERNS"
fspec add-hook --help | grep -A 10 "COMMON PATTERNS"
fspec copy-virtual-hooks --help | grep -A 10 "COMMON PATTERNS"
fspec list-virtual-hooks --help | grep -A 10 "COMMON PATTERNS"
fspec link-coverage --help | grep -A 10 "COMMON PATTERNS"
```

## Fix Strategy

1. Update `src/utils/help-formatter.ts`:
   - Change interface to accept objects: `commonPatterns?: Array<{ pattern: string; example: string; description: string }> | string[];`
   - Update formatter to handle both formats (backward compatibility)

2. After fix, regenerate all help outputs to verify correct display

## Expected Output After Fix

```
COMMON PATTERNS
  • Linting Before Implementation
    Example: fspec add-virtual-hook AUTH-001 pre-implementing "npm run lint" --blocking
    Ensures code is clean before starting implementation. Prevents messy code from being committed.

  • Type Checking Before Validation
    Example: fspec add-virtual-hook AUTH-001 pre-validating "npm run typecheck" --blocking
    Catches type errors before moving to validation phase. Strict quality gate.
```
