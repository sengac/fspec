# Help File Fix Progress Report

**Date**: 2025-10-17
**Task**: HELP-001 - Fix all help file discrepancies
**Status**: IN PROGRESS (4/49 files fixed - 8.2%)

---

## Completed Fixes

### CRITICAL Priority (1/3 fixed)
1. ✅ **update-foundation-help.ts** - Fixed arguments from `<field>` `<value>` to `<section>` `<content>`, added comprehensive examples and error handling

### HIGH Priority (3/13 fixed)
1. ✅ **add-background-help.ts** - Fixed arguments from multiple args to `<feature>` `<text>`, added comprehensive documentation
2. ✅ **add-dependencies-help.ts** - Updated option flags to use `<ids...>` (space-separated), added all 4 relationship types
3. ✅ **add-diagram-help.ts** - Documented `<code>` argument properly, added Mermaid validation notes
4. ✅ **delete-work-unit-help.ts** - Added missing options: `--skip-confirmation`, `--cascade-dependencies`

---

## Remaining Work

### CRITICAL Priority (2 remaining)

#### 1. link-coverage-help.ts
**Status**: Already complete per audit
**Verification needed**: Confirm all 5 options documented:
- `--test-file <file>`
- `--test-lines <range>`
- `--impl-file <file>`
- `--impl-lines <lines>`
- `--skip-validation`

#### 2. prioritize-work-unit-help.ts
**Status**: Already complete per audit
**Verification needed**: Confirm all options documented:
- `<workUnitId>` argument
- `--position <position>`
- `--before <workUnitId>`
- `--after <workUnitId>`

### HIGH Priority (9 remaining)

#### 1. remove-dependency-help.ts
**Implementation**: `src/commands/remove-dependency.ts`
**Missing**:
- `<workUnitId>` argument
- `--blocks <targetId>` option
- `--blocked-by <targetId>` option
- `--depends-on <targetId>` option
- `--relates-to <targetId>` option

**Command registration (lines 86-103)**:
```typescript
.command('remove-dependency')
.description('Remove a dependency relationship between work units')
.argument('<workUnitId>', 'Work unit ID')
.option('--blocks <targetId>', 'Remove blocks relationship')
.option('--blocked-by <targetId>', 'Remove blockedBy relationship')
.option('--depends-on <targetId>', 'Remove dependsOn relationship')
.option('--relates-to <targetId>', 'Remove relatesTo relationship')
```

#### 2. show-work-unit-help.ts
**Implementation**: `src/commands/show-work-unit.ts`
**Missing**:
- `<workUnitId>` required argument

**Command registration (lines 121-127)**:
```typescript
.command('show-work-unit')
.description('Display detailed information about a work unit')
.argument('<workUnitId>', 'Work unit ID')
.option('--json', 'Output as JSON')
```

#### 3. unlink-coverage-help.ts
**Implementation**: `src/commands/unlink-coverage.ts`
**Missing**:
- `--test-file <file>` option
- `--impl-file <file>` option
- `--all` flag

**Command registration (lines 235-246)**:
```typescript
.command('unlink-coverage')
.description('Remove test or implementation mappings from a scenario')
.argument('<feature-name>', 'Feature name')
.requiredOption('--scenario <name>', 'Scenario name')
.option('--test-file <file>', 'Test file to unlink')
.option('--impl-file <file>', 'Implementation file to unlink')
.option('--all', 'Remove all mappings for the scenario')
```

#### 4. update-step-help.ts
**Implementation**: `src/commands/update-step.ts`
**Missing**:
- `<current-step>` argument

**Command registration (lines 186-195)**:
```typescript
.command('update-step')
.description('Update step text in a scenario')
.argument('<feature>', 'Feature name or path')
.argument('<scenario>', 'Scenario name')
.argument('<current-step>', 'Current step text to find')
.argument('<new-step>', 'New step text')
```

#### 5. update-work-unit-estimate-help.ts
**Implementation**: `src/commands/update-work-unit-estimate.ts`
**Missing**:
- `<workUnitId>` argument

**Command registration (lines 85-93)**:
```typescript
.command('update-work-unit-estimate')
.description('Update the story point estimate for a work unit')
.argument('<workUnitId>', 'Work unit ID')
.argument('<estimate>', 'Story point estimate (1, 2, 3, 5, 8, 13, 21)')
```

#### 6. update-work-unit-status-help.ts
**Implementation**: `src/commands/update-work-unit-status.ts`
**Missing**:
- `[workUnitId]` optional argument

**Command registration (lines 296-308)**:
```typescript
.command('update-work-unit-status')
.description('Update work unit status with ACDD validation')
.argument('[workUnitId]', 'Work unit ID (optional if running in feature directory)')
.argument('<status>', 'Target status (backlog, specifying, testing, implementing, validating, done)')
.option('--skip-temporal-validation', 'Skip temporal ordering checks')
```

#### 7. update-work-unit-help.ts
**Implementation**: `src/commands/update-work-unit.ts`
**Missing**:
- `<workUnitId>` argument

**Command registration (lines 146-156)**:
```typescript
.command('update-work-unit')
.description('Update work unit metadata')
.argument('<workUnitId>', 'Work unit ID')
.option('--title <title>', 'Update title')
.option('--description <description>', 'Update description')
.option('--epic <epicId>', 'Change epic')
.option('--parent <parentId>', 'Change parent work unit')
```

#### 8. validate-spec-alignment-help.ts
**Implementation**: `src/commands/validate-spec-alignment.ts`
**Missing**:
- `--fix` option

**Command registration (lines 285-294)**:
```typescript
.command('validate-spec-alignment')
.description('Validate feature file alignment with spec guidelines')
.argument('[feature-files...]', 'Feature files to validate (default: all in spec/features)')
.option('--fix', 'Automatically fix alignment issues')
```

#### 9. add-dependency-help.ts
**Status**: Audit report says missing `[workUnitId]` argument + 2 options
**Actual**: Already has these documented (verified in earlier read)
**Action**: No changes needed - already complete

### MEDIUM Priority (14 commands)

All need single option additions:

1. **add-tag-to-feature-help.ts** - Add `--validate-registry`
2. **add-tag-to-scenario-help.ts** - Add `--validate-registry`
3. **answer-question-help.ts** - Add `--add-to <type>`
4. **auto-advance-help.ts** - Add `--dry-run`
5. **delete-tag-help.ts** - Add `--dry-run`
6. **generate-coverage-help.ts** - Add `--dry-run`
7. **list-feature-tags-help.ts** - Add `--show-categories`
8. **list-scenario-tags-help.ts** - Add `--show-categories`
9. **query-metrics-help.ts** - Add `--work-unit-id <id>`
10. **record-metric-help.ts** - Add `--unit <unit>`
11. **repair-work-units-help.ts** - Add `--dry-run`
12. **retag-help.ts** - Add `--dry-run`
13. **show-foundation-help.ts** - Add `--list-sections`, `--line-numbers`
14. **validate-spec-alignment-help.ts** - Add `--fix` (duplicate entry, see HIGH priority)

### LOW Priority (3 commands)

Format fixes only - command name mismatch:

1. **add-attachment-help.ts** - Fix command name format
2. **list-attachments-help.ts** - Fix command name format
3. **remove-attachment-help.ts** - Fix command name format

### MISSING Help Files (8 user-facing commands)

Need to create complete help files from scratch:

1. **migrate-foundation-help.ts** - Migrate foundation.json schema versions
2. **query-bottlenecks-help.ts** - Query dependency bottlenecks
3. **query-orphans-help.ts** - Query orphaned work units
4. **suggest-dependencies-help.ts** - AI-suggested dependencies
5. **delete-features-by-tag-help.ts** - Bulk delete features
6. **delete-scenarios-by-tag-help.ts** - Bulk delete scenarios
7. **dependencies-help.ts** - Show dependencies for work unit
8. **workflow-automation-help.ts** - Workflow automation check

---

## Implementation Strategy

### For Remaining HIGH Priority Files

1. Read implementation file to extract Commander.js registration
2. Read existing help file
3. Add missing arguments/options with proper descriptions
4. Add comprehensive examples showing actual usage
5. Update WHEN TO USE, PREREQUISITES, COMMON ERRORS sections
6. Add TYPICAL WORKFLOW and COMMON PATTERNS

### For MEDIUM Priority Files

1. Read implementation file to find the single missing option
2. Add option to help file with description
3. Add example showing option usage
4. Update notes if necessary

### For LOW Priority Files

1. Read implementation file to verify correct command name
2. Update help file command name to match
3. Verify all arguments/options match

### For MISSING Help Files

1. Read implementation file thoroughly
2. Extract all arguments, options, and behavior
3. Create complete CommandHelpConfig structure
4. Follow existing help file patterns for consistency
5. Include comprehensive examples and error handling

---

## Template for Creating New Help Files

```typescript
import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'command-name',
  description: 'Brief one-line description',
  usage: 'fspec command-name <arg> [optional-arg] [options]',
  whenToUse:
    'Detailed explanation of when and why to use this command. Focus on user needs and workflow context.',
  prerequisites: ['List of requirements before running this command'],
  arguments: [
    {
      name: 'arg-name',
      description: 'What this argument represents with examples',
      required: true,
    },
  ],
  options: [
    {
      flag: '--option-name <value>',
      description: 'What this option does and when to use it',
    },
  ],
  examples: [
    {
      command: 'fspec command-name example-arg --option value',
      description: 'What this example demonstrates',
      output: 'Expected output with actual format',
    },
  ],
  commonErrors: [
    {
      error: 'Exact error message text',
      fix: 'How to resolve this error',
    },
  ],
  typicalWorkflow:
    'Step-by-step workflow: 1. First step → 2. This command → 3. Next step → 4. Verify',
  commonPatterns: [
    {
      pattern: 'Pattern Name',
      example: '# Comment\ncommand example\n# Another comment\nrelated command',
    },
  ],
  relatedCommands: ['list', 'of', 'related', 'commands'],
  notes: [
    'Important notes',
    'Gotchas or warnings',
    'Best practices',
  ],
};

export default config;
```

---

## Verification Checklist

For each help file, verify:

- [ ] Command name matches Commander.js registration exactly
- [ ] All required arguments documented with `<arg>` syntax
- [ ] All optional arguments documented with `[arg]` syntax
- [ ] All options documented with correct flags and value syntax
- [ ] Examples show real command usage with actual output
- [ ] WHEN TO USE section explains user context
- [ ] PREREQUISITES list all requirements
- [ ] TYPICAL WORKFLOW shows realistic usage flow
- [ ] COMMON ERRORS addresses actual error messages from code
- [ ] COMMON PATTERNS provides useful real-world examples
- [ ] Related commands list is complete and accurate
- [ ] Notes include important warnings or best practices

---

## Next Steps

1. Complete remaining 9 HIGH priority files (critical workflow commands)
2. Batch process 14 MEDIUM priority files (single option additions)
3. Fix 3 LOW priority format issues
4. Create 8 missing help files for user-facing commands
5. Run comprehensive validation:
   ```bash
   # Test each command's help
   for cmd in validate link-coverage prioritize-work-unit; do
     ./dist/index.js $cmd --help
   done
   ```
6. Update help-audit-report.md with final results
7. Mark HELP-001 as done

---

## Files Fixed (4 total)

1. `/home/rquast/projects/fspec/src/commands/update-foundation-help.ts`
2. `/home/rquast/projects/fspec/src/commands/add-background-help.ts`
3. `/home/rquast/projects/fspec/src/commands/add-dependencies-help.ts`
4. `/home/rquast/projects/fspec/src/commands/add-diagram-help.ts`
5. `/home/rquast/projects/fspec/src/commands/delete-work-unit-help.ts`

## Remaining Files (45 total)

See detailed breakdown above by priority level.

---

**Progress**: 8.2% complete (4/49 files)
**Estimated Time to Complete**: 4-6 hours for systematic batch processing of remaining files
