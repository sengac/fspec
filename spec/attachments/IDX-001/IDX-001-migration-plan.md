# IDX-001: Stable Indices with Soft Delete - Complete Migration Plan

**Date:** 2025-11-01
**Work Unit:** IDX-001
**Dependency:** MIG-001 (completed)
**Target Version:** 0.7.0

## Executive Summary

This document provides a comprehensive analysis and implementation plan for migrating from string-based arrays to object-based collections with stable IDs and soft-delete semantics. This change eliminates the critical bug where sequential AI removal operations cause data loss due to array index shifting when using `.splice()`.

---

## Problem Statement

### Current Behavior (BROKEN)

**Data Structure:**
```typescript
interface WorkUnit {
  rules?: string[];           // ['Rule A', 'Rule B', 'Rule C']
  examples?: string[];        // ['Ex 1', 'Ex 2', 'Ex 3']
  assumptions?: string[];     // ['Assume A', 'Assume B']
  architectureNotes?: string[]; // ['Note 1', 'Note 2']
  questions?: QuestionItem[]; // ALREADY CORRECT - uses objects!
}
```

**The Bug:**
When AI agent removes items at indices 1 and 2 sequentially:
1. AI calls `remove-rule AUTH-001 1` → removes "Rule B", array becomes `['Rule A', 'Rule C']`
2. AI calls `remove-rule AUTH-001 2` → **ERROR! Index 2 doesn't exist**, or removes wrong item

**Why This Happens:**
- `remove-rule.ts:56` uses `workUnit.rules.splice(options.index, 1)`
- `.splice()` mutates the array IN-PLACE, shifting all subsequent indices
- The second removal targets the wrong index because indices shifted after first removal

---

## Solution Design

### New Data Structure

```typescript
// Base interface for all items with stable IDs
interface ItemWithId {
  id: number;           // Auto-incrementing, never reused
  text: string;         // The actual content
  deleted: boolean;     // Soft-delete flag
  createdAt: string;    // ISO 8601 timestamp
  deletedAt?: string;   // ISO 8601 timestamp (only when deleted=true)
}

// Specific item types (all extend ItemWithId)
interface RuleItem extends ItemWithId {}
interface ExampleItem extends ItemWithId {}
interface ArchitectureNoteItem extends ItemWithId {}
// QuestionItem already has similar structure but needs alignment

// Updated WorkUnit interface
interface WorkUnit {
  ...
  // Collections now use objects with stable IDs
  rules?: RuleItem[];
  examples?: ExampleItem[];
  assumptions?: string[];  // Still strings (not in scope for IDX-001)
  architectureNotes?: ArchitectureNoteItem[];

  // ID counters for auto-increment
  nextRuleId?: number;
  nextExampleId?: number;
  nextQuestionId?: number;
  nextNoteId?: number;
  ...
}
```

### Soft-Delete Pattern

**Remove Operation:**
```typescript
// OLD (src/commands/remove-rule.ts:56)
workUnit.rules.splice(options.index, 1);  // ❌ SHIFTS INDICES

// NEW
const rule = workUnit.rules.find(r => r.id === ruleId);
if (!rule) throw new Error(`Rule with ID ${ruleId} not found`);
if (rule.deleted) {
  // Idempotent - already deleted
  console.log(`Rule ID ${ruleId} already deleted`);
  return;
}
rule.deleted = true;
rule.deletedAt = new Date().toISOString();
// ✅ Array indices NEVER shift
```

**Add Operation:**
```typescript
// OLD (src/commands/add-rule.ts:46)
workUnit.rules.push(options.rule);  // ❌ Just strings

// NEW
if (!workUnit.nextRuleId) workUnit.nextRuleId = 0;  // Backward compat
const newRule: RuleItem = {
  id: workUnit.nextRuleId++,
  text: options.rule,
  deleted: false,
  createdAt: new Date().toISOString()
};
workUnit.rules.push(newRule);
// ✅ Stable ID, never reused
```

**Display Filtering:**
```typescript
// Utility function to filter active (non-deleted) items
function filterActive<T extends ItemWithId>(items: T[]): T[] {
  return items.filter(item => !item.deleted);
}

// Usage in show-work-unit.ts
const activeRules = filterActive(workUnit.rules || []);
activeRules.forEach((rule, displayIndex) => {
  console.log(`[${rule.id}] ${rule.text}`);  // Show stable ID, not array index
});

if (workUnit.rules && workUnit.rules.some(r => r.deleted)) {
  const deletedCount = workUnit.rules.filter(r => r.deleted).length;
  console.log(`${activeRules.length} active items (${deletedCount} deleted)`);
}
```

---

## Migration Strategy

### Migration File: `src/migrations/migrations/001-stable-indices.ts`

**Purpose:** Convert v0.6.0 string arrays to v0.7.0 object arrays with stable IDs

**Migration Logic:**
```typescript
import { Migration } from '../types';

const migration001: Migration = {
  version: '0.7.0',
  name: 'stable-indices',
  description: 'Convert string arrays to objects with stable IDs',

  up: (data) => {
    const now = new Date().toISOString();

    for (const workUnitId in data.workUnits) {
      const workUnit = data.workUnits[workUnitId];

      // Migrate rules: string[] → RuleItem[]
      if (workUnit.rules && Array.isArray(workUnit.rules)) {
        workUnit.rules = workUnit.rules.map((text, index) => {
          // Handle mixed format (partially migrated data)
          if (typeof text === 'object' && 'id' in text) {
            return text;  // Already migrated
          }
          return {
            id: index,
            text: typeof text === 'string' ? text : text.text,
            deleted: false,
            createdAt: now
          };
        });
        workUnit.nextRuleId = workUnit.rules.length;
      }

      // Migrate examples: string[] → ExampleItem[]
      if (workUnit.examples && Array.isArray(workUnit.examples)) {
        workUnit.examples = workUnit.examples.map((text, index) => {
          if (typeof text === 'object' && 'id' in text) {
            return text;
          }
          return {
            id: index,
            text: typeof text === 'string' ? text : text.text,
            deleted: false,
            createdAt: now
          };
        });
        workUnit.nextExampleId = workUnit.examples.length;
      }

      // Migrate architectureNotes: string[] → ArchitectureNoteItem[]
      if (workUnit.architectureNotes && Array.isArray(workUnit.architectureNotes)) {
        workUnit.architectureNotes = workUnit.architectureNotes.map((text, index) => {
          if (typeof text === 'object' && 'id' in text) {
            return text;
          }
          return {
            id: index,
            text: typeof text === 'string' ? text : text.text,
            deleted: false,
            createdAt: now
          };
        });
        workUnit.nextNoteId = workUnit.architectureNotes.length;
      }

      // Align questions with ItemWithId structure (add id, deleted, createdAt)
      if (workUnit.questions && Array.isArray(workUnit.questions)) {
        workUnit.questions = workUnit.questions.map((q, index) => {
          if ('id' in q && 'deleted' in q) {
            return q;  // Already migrated
          }
          return {
            ...q,
            id: index,
            deleted: false,
            createdAt: now
          };
        });
        workUnit.nextQuestionId = workUnit.questions.length;
      }
    }

    return data;
  },

  down: (data) => {
    // Rollback: convert objects back to strings
    for (const workUnitId in data.workUnits) {
      const workUnit = data.workUnits[workUnitId];

      if (workUnit.rules) {
        workUnit.rules = workUnit.rules
          .filter(r => !r.deleted)  // Remove soft-deleted items
          .map(r => r.text);
        delete workUnit.nextRuleId;
      }

      if (workUnit.examples) {
        workUnit.examples = workUnit.examples
          .filter(e => !e.deleted)
          .map(e => e.text);
        delete workUnit.nextExampleId;
      }

      if (workUnit.architectureNotes) {
        workUnit.architectureNotes = workUnit.architectureNotes
          .filter(n => !n.deleted)
          .map(n => n.text);
        delete workUnit.nextNoteId;
      }

      if (workUnit.questions) {
        workUnit.questions = workUnit.questions
          .filter(q => !q.deleted)
          .map(({ id, deleted, createdAt, deletedAt, ...rest }) => rest);
        delete workUnit.nextQuestionId;
      }
    }

    return data;
  }
};

export default migration001;
```

**Registration:** Add to `src/migrations/registry.ts`:
```typescript
import migration001 from './migrations/001-stable-indices';

const migrations: Migration[] = [
  migration001,  // v0.7.0
];
```

**Automatic Execution:**
Migration runs automatically on first command after upgrade via `ensureLatestVersion()` in `src/utils/ensure-files.ts`

**Backup Creation:**
Creates `spec/work-units.json.backup-0.7.0-{timestamp}` before migration

---

## Implementation Checklist

### Phase 1: Type Definitions
- [ ] Update `src/types/index.ts`:
  - [ ] Add `ItemWithId` base interface
  - [ ] Add `RuleItem extends ItemWithId`
  - [ ] Add `ExampleItem extends ItemWithId`
  - [ ] Add `ArchitectureNoteItem extends ItemWithId`
  - [ ] Update `QuestionItem` to extend `ItemWithId` (add `id`, `deleted`, `createdAt`, `deletedAt?`)
  - [ ] Update `WorkUnit` interface:
    - [ ] Change `rules?: string[]` to `rules?: RuleItem[]`
    - [ ] Change `examples?: string[]` to `examples?: ExampleItem[]`
    - [ ] Change `architectureNotes?: string[]` to `architectureNoteItem[]`
    - [ ] Add `nextRuleId?: number`
    - [ ] Add `nextExampleId?: number`
    - [ ] Add `nextQuestionId?: number`
    - [ ] Add `nextNoteId?: number`

### Phase 2: Migration System
- [ ] Create `src/migrations/migrations/001-stable-indices.ts` (see above)
- [ ] Register migration in `src/migrations/registry.ts`
- [ ] Test migration with sample data (backward compatibility, mixed format handling)
- [ ] Test rollback (`down()` function)

### Phase 3: Command Updates (Add Operations)
- [ ] **add-rule.ts** (line 46):
  - [ ] Initialize `nextRuleId` if undefined
  - [ ] Create RuleItem object with `{ id, text, deleted: false, createdAt }`
  - [ ] Increment `nextRuleId`
- [ ] **add-example.ts** (line 48):
  - [ ] Initialize `nextExampleId` if undefined
  - [ ] Create ExampleItem object
  - [ ] Increment `nextExampleId`
- [ ] **add-architecture-note.ts** (line 36):
  - [ ] Initialize `nextNoteId` if undefined
  - [ ] Create ArchitectureNoteItem object
  - [ ] Increment `nextNoteId`
- [ ] **add-question.ts**:
  - [ ] Initialize `nextQuestionId` if undefined
  - [ ] Add `id`, `deleted: false`, `createdAt` to QuestionItem
  - [ ] Increment `nextQuestionId`

### Phase 4: Command Updates (Remove Operations - Soft Delete)
- [ ] **remove-rule.ts** (line 56):
  - [ ] Replace `.splice()` with soft-delete logic
  - [ ] Find by ID: `workUnit.rules.find(r => r.id === ruleId)`
  - [ ] Validate ID exists, throw error if not found
  - [ ] If already deleted, return idempotent success message
  - [ ] Set `deleted: true`, `deletedAt: ISO timestamp`
- [ ] **remove-example.ts**:
  - [ ] Same soft-delete pattern as remove-rule
- [ ] **remove-architecture-note.ts**:
  - [ ] Same soft-delete pattern
- [ ] **remove-question.ts**:
  - [ ] Same soft-delete pattern

### Phase 5: New Commands (Restore Operations)
- [ ] **src/commands/restore-rule.ts**:
  - [ ] Find rule by ID
  - [ ] Validate exists (error if not)
  - [ ] If already active (`deleted: false`), return idempotent message
  - [ ] Set `deleted: false`, delete `deletedAt` field
  - [ ] Support bulk restore: parse comma-separated IDs `"2,5,7"`
- [ ] **src/commands/restore-example.ts** (same pattern)
- [ ] **src/commands/restore-question.ts** (same pattern)
- [ ] **src/commands/restore-architecture-note.ts** (same pattern)
- [ ] Help files:
  - [ ] `src/commands/restore-rule-help.ts`
  - [ ] `src/commands/restore-example-help.ts`
  - [ ] `src/commands/restore-question-help.ts`
  - [ ] `src/commands/restore-architecture-note-help.ts`

### Phase 6: New Commands (Compaction)
- [ ] **src/commands/compact-work-unit.ts**:
  - [ ] Filter out deleted items: `items.filter(i => !i.deleted)`
  - [ ] Sort remaining items by `createdAt` (chronological order)
  - [ ] Renumber IDs sequentially: `0, 1, 2, ...`
  - [ ] Reset ID counters: `nextRuleId = rules.length`
  - [ ] Require `--force` flag for confirmation (permanent operation)
  - [ ] Display warning if compacting during non-done status
- [ ] **src/commands/compact-work-unit-help.ts**
- [ ] **Auto-compact on done status:**
  - [ ] Update `src/commands/update-work-unit-status.ts`
  - [ ] Before setting `status='done'`, call `compactWorkUnit(workUnit)`

### Phase 7: New Commands (Show Deleted Items)
- [ ] **src/commands/show-deleted.ts**:
  - [ ] Filter deleted items: `items.filter(i => i.deleted)`
  - [ ] Display format: `[ID] text (deleted: ISO timestamp)`
  - [ ] Show counts: "2 deleted rules, 1 deleted example"
- [ ] **src/commands/show-deleted-help.ts**

### Phase 8: Display Updates
- [ ] **show-work-unit.ts**:
  - [ ] Filter active items: `filterActive(workUnit.rules || [])`
  - [ ] Display with stable IDs: `[${rule.id}] ${rule.text}`
  - [ ] Show item counts: `"X active items (Y deleted)"`
  - [ ] Add `--verbose` flag:
    - [ ] Display `createdAt` timestamp for all items
    - [ ] Display `deletedAt` timestamp for deleted items (if --verbose includes deleted)
- [ ] **list-work-units.ts**: Update to filter active items
- [ ] **generate-scenarios.ts**: Filter active rules/examples when generating scenarios
- [ ] **export-example-map.ts**: Filter active items in export
- [ ] Any other commands that display these collections

### Phase 9: Testing
- [ ] Unit tests for migration (`src/migrations/__tests__/migration-system.test.ts`):
  - [ ] Test string → object conversion
  - [ ] Test ID counter initialization
  - [ ] Test mixed format handling (partial migration)
  - [ ] Test rollback (down() function)
- [ ] Integration tests for soft-delete:
  - [ ] Sequential removal without index shifts
  - [ ] Display with gaps in indices
  - [ ] Restore deleted item to original index
  - [ ] Idempotent remove on already-deleted item
  - [ ] Idempotent restore on already-active item
- [ ] Integration tests for compaction:
  - [ ] Manual compaction with --force
  - [ ] Auto-compact on done status
  - [ ] Chronological ordering preservation
  - [ ] ID counter reset
- [ ] Integration tests for new commands:
  - [ ] Bulk restore with comma-separated IDs
  - [ ] Show deleted items
  - [ ] Verbose mode timestamps

### Phase 10: Documentation
- [ ] Update `src/commands/bootstrap.ts`:
  - [ ] Document stable indices system
  - [ ] Document soft-delete pattern
  - [ ] Document restore commands
  - [ ] Document compact command
  - [ ] Document auto-compact on done
- [ ] Update `src/help.ts` (if applicable)
- [ ] Add help files for all new commands (registered in help-registry.ts)

---

## Files Requiring Changes

### NEW Files (to be created)
```
src/migrations/migrations/001-stable-indices.ts   (migration script)
src/commands/restore-rule.ts                     (restore soft-deleted rule)
src/commands/restore-example.ts                  (restore soft-deleted example)
src/commands/restore-question.ts                 (restore soft-deleted question)
src/commands/restore-architecture-note.ts        (restore soft-deleted note)
src/commands/compact-work-unit.ts                (permanent deletion + renumbering)
src/commands/show-deleted.ts                     (show deleted items)
src/commands/restore-rule-help.ts                (help file)
src/commands/restore-example-help.ts             (help file)
src/commands/restore-question-help.ts            (help file)
src/commands/restore-architecture-note-help.ts   (help file)
src/commands/compact-work-unit-help.ts           (help file)
src/commands/show-deleted-help.ts                (help file)
```

### MODIFIED Files (existing code changes)
```
src/types/index.ts                               (type definitions)
src/migrations/registry.ts                       (register migration)
src/commands/add-rule.ts                         (create RuleItem object)
src/commands/add-example.ts                      (create ExampleItem object)
src/commands/add-architecture-note.ts            (create ArchitectureNoteItem object)
src/commands/add-question.ts                     (add id, deleted, createdAt)
src/commands/remove-rule.ts                      (soft-delete pattern)
src/commands/remove-example.ts                   (soft-delete pattern)
src/commands/remove-architecture-note.ts         (soft-delete pattern)
src/commands/remove-question.ts                  (soft-delete pattern)
src/commands/answer-question.ts                  (handle QuestionItem structure)
src/commands/show-work-unit.ts                   (filter active, show IDs, counts, --verbose)
src/commands/update-work-unit-status.ts          (auto-compact on done)
src/commands/list-work-units.ts                  (filter active items if displaying)
src/commands/generate-scenarios.ts               (filter active rules/examples)
src/commands/export-example-map.ts               (filter active items)
src/commands/bootstrap.ts                        (documentation updates)
```

### TESTS to Update/Create
```
src/migrations/__tests__/migration-system.test.ts  (migration tests)
src/commands/__tests__/stable-indices.test.ts      (new test file for soft-delete)
src/commands/__tests__/compact-work-unit.test.ts   (compaction tests)
src/commands/__tests__/example-mapping.test.ts     (update for new structure)
```

---

## Risk Analysis & Mitigation

### High Risk Areas

1. **Backward Compatibility**
   - **Risk:** Existing work-units.json files break after upgrade
   - **Mitigation:**
     - Migration handles both string and object formats (mixed data)
     - down() function allows rollback
     - Automatic backup before migration
     - Test migration with production-like data

2. **Data Loss During Migration**
   - **Risk:** Migration fails mid-process, corrupting work-units.json
   - **Mitigation:**
     - Atomic migration (backup → migrate → save)
     - If migration.up() throws, file is NOT saved
     - Backup path shown in error message for manual restore

3. **Breaking Existing Commands**
   - **Risk:** Commands expecting strings get objects
   - **Mitigation:**
     - Update ALL commands that interact with these arrays
     - Comprehensive grep for `.splice()`, `.push()`, array indexing
     - Integration tests cover all command paths

4. **AI Agent Confusion**
   - **Risk:** AI uses old index-based removal syntax
   - **Mitigation:**
     - Update help files to document ID-based syntax
     - CLI still accepts indices but interprets as IDs post-migration
     - System reminders in show-work-unit output explain stable IDs

### Medium Risk Areas

1. **Performance Impact**
   - **Risk:** Filtering deleted items on every display slows down
   - **Mitigation:**
     - Collections are small (typically < 50 items per work unit)
     - `.filter()` is O(n) but n is small
     - Compaction removes deleted items to keep arrays lean

2. **Compaction Timing**
   - **Risk:** Auto-compact on done status destroys accidentally deleted items
   - **Mitigation:**
     - Restore commands available before compaction
     - Manual compact requires --force flag
     - show-deleted command helps identify items before compaction

### Low Risk Areas

1. **Disk Space (Deleted Items)**
   - **Risk:** Deleted items accumulate indefinitely
   - **Mitigation:**
     - Auto-compact on done status removes deleted items
     - Manual compact available during development
     - Typical work units have < 50 items, disk impact negligible

---

## Testing Strategy

### Unit Tests
```typescript
describe('Migration 001: Stable Indices', () => {
  it('converts string array to RuleItem array', () => {
    const input = { workUnits: { 'AUTH-001': { rules: ['Rule A', 'Rule B'] } } };
    const output = migration001.up(input);
    expect(output.workUnits['AUTH-001'].rules).toEqual([
      { id: 0, text: 'Rule A', deleted: false, createdAt: expect.any(String) },
      { id: 1, text: 'Rule B', deleted: false, createdAt: expect.any(String) }
    ]);
    expect(output.workUnits['AUTH-001'].nextRuleId).toBe(2);
  });

  it('handles mixed format (partial migration)', () => {
    const input = {
      workUnits: {
        'AUTH-001': {
          rules: [
            'Old string',
            { id: 1, text: 'New object', deleted: false, createdAt: '2025-01-31T10:00:00Z' }
          ]
        }
      }
    };
    const output = migration001.up(input);
    expect(output.workUnits['AUTH-001'].rules[0]).toEqual({
      id: 0, text: 'Old string', deleted: false, createdAt: expect.any(String)
    });
    expect(output.workUnits['AUTH-001'].rules[1].text).toBe('New object');
  });
});

describe('Soft Delete', () => {
  it('sequential removal does not shift indices', async () => {
    // Setup: Create work unit with 5 rules
    await createStory({ prefix: 'AUTH', title: 'Test', description: 'Test' });
    await updateWorkUnitStatus({ workUnitId: 'AUTH-001', status: 'specifying' });
    await addRule({ workUnitId: 'AUTH-001', rule: 'Rule A' });
    await addRule({ workUnitId: 'AUTH-001', rule: 'Rule B' });
    await addRule({ workUnitId: 'AUTH-001', rule: 'Rule C' });

    // Remove rules at IDs 1 and 2
    await removeRule({ workUnitId: 'AUTH-001', index: 1 });  // ID 1
    await removeRule({ workUnitId: 'AUTH-001', index: 2 });  // ID 2

    // Verify: Rules 0 and 2 still exist (NO shift)
    const workUnit = await showWorkUnit({ workUnitId: 'AUTH-001' });
    expect(workUnit.rules).toEqual([
      { id: 0, text: 'Rule A', deleted: false, ... },
      { id: 1, text: 'Rule B', deleted: true, deletedAt: expect.any(String) },
      { id: 2, text: 'Rule C', deleted: true, deletedAt: expect.any(String) }
    ]);
  });
});
```

### Integration Tests
```bash
# Test full workflow with migration
fspec create-story AUTH "User Login"
fspec update-work-unit-status AUTH-001 specifying
fspec add-rule AUTH-001 "Password must be 8 chars"
fspec add-rule AUTH-001 "Email must be valid"
fspec add-rule AUTH-001 "Session expires after 24h"
fspec remove-rule AUTH-001 1  # Remove "Email must be valid"
fspec show-work-unit AUTH-001  # Should show [0] and [2], not [0] and [1]
fspec restore-rule AUTH-001 1  # Restore deleted rule
fspec show-work-unit AUTH-001  # Should show [0], [1], [2]
fspec compact-work-unit AUTH-001 --force  # Compact (no deleted items, so no change)
fspec update-work-unit-status AUTH-001 done  # Auto-compact on done
```

---

## Open Questions & Decisions

### Resolved (from Example Mapping)

1. **Q:** Should we provide a 'show-deleted' command?
   **A:** YES (Rule #29) - helps with debugging and selective restoration

2. **Q:** Should compact-work-unit require confirmation or run silently?
   **A:** Require --force flag (Rule #30) - permanent operation, needs user confirmation

3. **Q:** Should we warn or auto-compact when deleted item count > threshold?
   **A:** NO automatic threshold (Assumption #1) - only auto-compact on 'done' status

4. **Q:** Should restore commands support bulk operations?
   **A:** YES (Rule #31) - support comma-separated IDs like `"2,5,7"`

5. **Q:** Should we provide a separate 'purge' command for immediate deletion?
   **A:** NO (Assumption #2) - use compact-work-unit for permanent deletion

6. **Q:** Should display commands show timestamps in verbose mode?
   **A:** YES (Rule #32) - add --verbose flag to show createdAt/deletedAt

### Still Open (for implementation phase)

1. **CLI Syntax for Restore:**
   - Current: `fspec restore-rule AUTH-001 2,5,7`
   - Alternative: `fspec restore-rule AUTH-001 --ids 2,5,7`
   - **Decision:** Use positional argument (simpler, consistent with remove-rule)

2. **Display Format for Deleted Items:**
   - Option A: `[2] Rule B (deleted: 2025-01-31T12:00:00Z)`
   - Option B: `[2] ~~Rule B~~ (deleted 1 hour ago)`
   - **Decision:** Use Option A (ISO timestamps are machine-readable, no relative time parsing)

3. **Auto-Compact Messaging:**
   - Should auto-compact on done status log to console or run silently?
   - **Decision:** Log brief message: `"✓ Auto-compacted work unit (removed 3 deleted items)"`

---

## Success Criteria

### Must Have (Definition of Done)
- [ ] All tests pass (unit + integration)
- [ ] Migration completes successfully on sample data
- [ ] No data loss when removing multiple items sequentially
- [ ] Restore commands work correctly (single + bulk)
- [ ] Compact command removes deleted items and renumbers IDs
- [ ] Auto-compact triggers on done status
- [ ] show-work-unit displays stable IDs with gaps
- [ ] --verbose flag shows timestamps
- [ ] All help files created and registered
- [ ] Bootstrap documentation updated

### Should Have
- [ ] Performance benchmarks (migration time for 1000 work units)
- [ ] Error messages are clear and actionable
- [ ] System reminders guide AI agents through new commands

### Nice to Have
- [ ] Visual diff of before/after migration in test suite
- [ ] Metrics on migration success rate across different data formats

---

## Timeline Estimate

### Story Points: 8 (Complex, multiple components)

**Breakdown:**
- Type definitions: 1 point (straightforward)
- Migration script: 2 points (critical, needs thorough testing)
- Command updates (add/remove): 2 points (many files, repetitive)
- New commands (restore/compact/show-deleted): 2 points (new functionality)
- Testing + Documentation: 1 point (comprehensive tests)

**Dependencies:**
- MIG-001 ✅ COMPLETE (migration system exists)
- No external blockers

**Estimated Duration:** 6-8 hours
- Phase 1-2: 1 hour (types + migration)
- Phase 3-5: 2 hours (command updates)
- Phase 6-7: 1.5 hours (new commands)
- Phase 8: 1 hour (display updates)
- Phase 9: 2 hours (testing)
- Phase 10: 0.5 hours (documentation)

---

## Appendix: Code Reference Map

### Current Files Using `.splice()` (MUST UPDATE)
```
src/commands/remove-rule.ts:56          → Soft-delete pattern
src/commands/remove-example.ts:~56      → Soft-delete pattern
src/commands/remove-architecture-note.ts:~40 → Soft-delete pattern
src/commands/remove-question.ts:~50     → Soft-delete pattern
```

### Current Files Using `.push()` (MUST UPDATE)
```
src/commands/add-rule.ts:46             → Create RuleItem object
src/commands/add-example.ts:48          → Create ExampleItem object
src/commands/add-architecture-note.ts:36 → Create ArchitectureNoteItem object
src/commands/add-question.ts:~45        → Add id, deleted, createdAt
src/commands/answer-question.ts:84,94   → Create RuleItem/AssumptionItem (if migrating assumptions)
```

### Display Commands (MUST FILTER)
```
src/commands/show-work-unit.ts:129-145  → Filter active items, show counts
src/commands/list-work-units.ts         → Filter active if displaying
src/commands/generate-scenarios.ts      → Filter active rules/examples
src/commands/export-example-map.ts      → Filter active items
```

---

## Conclusion

This migration is **CRITICAL** for preventing data loss in AI-driven Example Mapping workflows. The current `.splice()` behavior causes silent data corruption when AI agents remove multiple items sequentially. The stable ID + soft-delete pattern eliminates this bug while providing recovery mechanisms (restore) and cleanup tools (compact).

**Next Steps:**
1. Review this migration plan with human for approval
2. Implement in order of phases (1 → 10)
3. Run comprehensive tests at each phase
4. Create checkpoint before starting implementation
5. Move IDX-001 to `testing` status once all phases complete

**Last Updated:** 2025-11-01
**Author:** Claude (AI Agent)
**Status:** Ready for Implementation
