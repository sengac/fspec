# AST Research: Codebase Patterns for SCHED-002

## 1. LockedFileManager Usage Pattern

**File:** `src/utils/file-manager.ts`

The `LockedFileManager` implements a three-layer file locking architecture:
- **Layer 1:** Inter-process coordination via `proper-lockfile`
- **Layer 2:** In-process readers-writer pattern
- **Layer 3:** Atomic write-replace pattern

### Usage Pattern (from `add-rule.ts`):

```typescript
import { fileManager } from '../utils/file-manager';

// Atomic transaction pattern
await fileManager.transaction(filePath, async fileData => {
  // Modify fileData in place
  Object.assign(fileData, data);
});
```

### Key Methods:
- `fileManager.read(filePath)` - Read-locked file access
- `fileManager.transaction(filePath, callback)` - Write-locked atomic transaction

## 2. Command Implementation Pattern

**Example:** `src/commands/add-rule.ts` (76 lines)

### Structure:
1. **Interface definitions** for options and results
2. **Core async function** that performs the logic
3. **Registration function** that wires to Commander.js

```typescript
interface AddRuleOptions {
  workUnitId: string;
  rule: string;
  cwd?: string;
}

interface AddRuleResult {
  success: boolean;
  ruleCount: number;
}

export async function addRule(options: AddRuleOptions): Promise<AddRuleResult> {
  const cwd = options.cwd || process.cwd();
  const filePath = join(cwd, 'spec/work-units.json');
  
  // Validation, transformation, persistence
  
  return { success: true, ... };
}

export function registerAddRuleCommand(program: Command): void {
  program
    .command('add-rule')
    .description('...')
    .argument('<arg>', 'desc')
    .action(async (...) => {
      try {
        await addRule({ ... });
        output.log('✓ ...');
      } catch (error: any) {
        output.error('✗ ...', error.message);
        process.exit(1);
      }
    });
}
```

## 3. Type Definitions Pattern

**Location:** `src/types/`

Types are defined in separate files. For schedules, we need:
- `src/types/schedule.ts` - Schedule-related interfaces

**Existing pattern from `src/types/index.ts`:**

```typescript
export interface ItemWithId {
  id: number;
  deleted: boolean;
}

export interface RuleItem extends ItemWithId {
  text: string;
  createdAt: string;
}
```

## 4. JSON Schema Validation Pattern

**Example location:** Schema files are referenced via `src/schemas/` or inline with Ajv.

```typescript
import Ajv from 'ajv';

const ajv = new Ajv();
const validate = ajv.compile(schema);
const valid = validate(data);
if (!valid) {
  throw new Error(`Validation error: ${ajv.errorsText(validate.errors)}`);
}
```

## 5. Ensure File Pattern

**Example:** `src/utils/ensure-files.ts`

```typescript
export async function ensureWorkUnitsFile(cwd: string): Promise<WorkUnitsData> {
  const workUnitsFile = join(cwd, 'spec/work-units.json');
  
  if (!existsSync(workUnitsFile)) {
    const defaultData = {
      version: '1.0.0',
      workUnits: {},
      prefixes: {}
    };
    await writeFile(workUnitsFile, JSON.stringify(defaultData, null, 2));
  }
  
  return await fileManager.read(workUnitsFile);
}
```

## 6. Validation Utilities Pattern

**For cron validation:**
```typescript
import cronValidate from 'cron-validate';

function validateCron(expression: string): boolean {
  const result = cronValidate(expression);
  return result.isValid();
}
```

**For timezone validation (built-in):**
```typescript
function getValidTimezones(): string[] {
  return Intl.supportedValuesOf('timeZone');
}

function validateTimezone(tz: string): boolean {
  return getValidTimezones().includes(tz);
}
```

## 7. Directory Structure for Schedule Commands

Based on existing patterns, schedule commands should be in:
- `src/commands/schedule/add.ts`
- `src/commands/schedule/remove.ts`
- `src/commands/schedule/pause.ts`
- `src/commands/schedule/resume.ts`
- `src/commands/schedule/list.ts`

Or follow existing flat structure:
- `src/commands/add-schedule.ts`
- `src/commands/remove-schedule.ts`
- `src/commands/pause-schedule.ts`
- `src/commands/resume-schedule.ts`
- `src/commands/list-schedules.ts`

## 8. Output Pattern

**Use `output` utility for consistent CLI messaging:**

```typescript
import { output } from '../utils/output';

output.log('✓ Success message');
output.error('✗ Error:', error.message);
```

## 9. Files to Create for SCHED-002

1. **Types:** `src/types/schedule.ts`
2. **Schema:** `src/schemas/schedule.schema.json`
3. **Utility:** `src/utils/ensure-schedules-file.ts`
4. **Validators:** `src/utils/validators/cron.ts`, `src/utils/validators/timezone.ts`
5. **Commands:**
   - `src/commands/add-schedule.ts`
   - `src/commands/remove-schedule.ts`
   - `src/commands/pause-schedule.ts`
   - `src/commands/resume-schedule.ts`
   - `src/commands/list-schedules.ts`

## Research Date
2026-03-18
