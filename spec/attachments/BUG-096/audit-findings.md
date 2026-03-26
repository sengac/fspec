# BUG-096: Foundation Command Audit — Complete Findings

## Audit Date: 2026-03-26

## How to Reproduce This Audit

Use `GraphSearch` to index and explore:

```
# Index the codebase (needed once per session)
GraphSearch(ast_index, path="src/commands")
GraphSearch(ast_index, path="src/cli")

# Find all foundation event storm files
GraphSearch(ast_search, query="foundation")
GraphSearch(ast_search, query="show-foundation-event-storm")
GraphSearch(ast_search, query="add-foundation-bounded-context")
GraphSearch(ast_search, query="add-aggregate-to-foundation")
GraphSearch(ast_search, query="add-domain-event-to-foundation")
GraphSearch(ast_search, query="add-command-to-foundation")

# Verify no remove commands exist
GraphSearch(ast_search, query="remove-foundation-bounded-context")  → 0 results
GraphSearch(ast_search, query="remove-aggregate-from-foundation")   → 0 results
```

---

## Executive Summary

6 issues found:
- **4 missing remove commands** for foundation event storm items (in scope)
- 1 documented-but-unimplemented command (`derive-tags-from-foundation`) (out of scope)
- 1 bug in `add-diagram`/`delete-diagram` pair (out of scope)

---

## Complete Cross-Reference: Every File That Mentions Foundation Event Storm Commands

### Reference Command: `add-foundation-bounded-context`

| File | Lines | Role |
|------|-------|------|
| `src/commands/add-foundation-bounded-context.ts` | 25-131 | **Implementation** — `addFoundationBoundedContext()`, `addFoundationBoundedContextCommand()`, `registerAddFoundationBoundedContextCommand()` |
| `src/commands/add-foundation-bounded-context-help.ts` | 4-48 | **Help** — command help definition |
| `src/cli/program.ts` | 135, 321 | **Registration** — import + `registerAddFoundationBoundedContextCommand(program)` |
| `src/help.ts` | 1204-1210 | **Global help** — CLI help text |
| `src/commands/discover-foundation.ts` | 433 | **Documentation** — auto-created work unit description mentions this command |
| `src/commands/bootstrap.ts` | 196, 225 | **Documentation** — bootstrap help output |
| `src/utils/slashCommandSections/bigPictureEventStorm.ts` | 51-53, 80, 96-98 | **Documentation** — slash command section examples + comparison table |
| `src/commands/add-bounded-context-help.ts` | 61 | **See also** — related commands list |
| `src/commands/add-aggregate-to-foundation-help.ts` | 57, 66 | **See also + error fix** — related commands + "create BC first" fix |
| `src/commands/add-domain-event-to-foundation-help.ts` | 60, 69 | **See also + error fix** |
| `src/commands/add-command-to-foundation-help.ts` | 60, 69 | **See also + error fix** |
| `src/commands/__tests__/add-foundation-bounded-context.test.ts` | 5-274 | **Tests** — 6 test scenarios |
| `src/commands/__tests__/add-foundation-event-storm-items.test.ts` | 35 | **Tests** — expects section to contain this command |
| `src/commands/__tests__/discover-foundation-finalize-auto-create-work-unit.test.ts` | 168 | **Tests** — verifies auto-created work unit mentions this command |
| `src/commands/__tests__/auto-create-big-picture-event-storm-work-unit.test.ts` | 139, 257 | **Tests** — verifies work unit description |
| `src/commands/__tests__/bootstrap-event-storm-reminder.test.ts` | 89, 160 | **Tests** — bootstrap output verification |
| `src/utils/slashCommandSections/__tests__/bigPictureEventStorm.test.ts` | 35, 52-53 | **Tests** — slash command section verification |
| `src/generators/__tests__/generate-event-storm-section-in-foundation-md.test.ts` | 7, 69-70 | **Tests** — imports and uses `addFoundationBoundedContext` |

### Reference Command: `add-aggregate-to-foundation`

| File | Lines | Role |
|------|-------|------|
| `src/commands/add-aggregate-to-foundation.ts` | 23-162 | **Implementation** — `addAggregateToFoundation()`, register function |
| `src/commands/add-aggregate-to-foundation-help.ts` | 4-66 | **Help** — command help definition |
| `src/cli/program.ts` | 136, 322 | **Registration** |
| `src/commands/discover-foundation.ts` | 434 | **Documentation** — work unit description |
| `src/commands/bootstrap.ts` | 197, 226 | **Documentation** |
| `src/utils/slashCommandSections/bigPictureEventStorm.ts` | 56-58, 107-108 | **Documentation** |
| `src/commands/add-foundation-bounded-context-help.ts` | 45 | **See also** |
| `src/commands/add-domain-event-to-foundation-help.ts` | 61 | **See also** |
| `src/commands/add-command-to-foundation-help.ts` | 61 | **See also** |
| `src/commands/__tests__/add-foundation-event-storm-items.test.ts` | 5, 70-83, 288-302 | **Tests** |
| `src/commands/__tests__/auto-create-big-picture-event-storm-work-unit.test.ts` | 140, 259-260 | **Tests** |
| `src/commands/__tests__/bootstrap-event-storm-reminder.test.ts` | 90, 161 | **Tests** |
| `src/utils/slashCommandSections/__tests__/bigPictureEventStorm.test.ts` | 36 | **Tests** |

### Reference Command: `add-domain-event-to-foundation`

| File | Lines | Role |
|------|-------|------|
| `src/commands/add-domain-event-to-foundation.ts` | 23-162 | **Implementation** |
| `src/commands/add-domain-event-to-foundation-help.ts` | 4-69 | **Help** |
| `src/cli/program.ts` | 137, 323 | **Registration** |
| `src/commands/discover-foundation.ts` | 435 | **Documentation** |
| `src/commands/bootstrap.ts` | 198, 227 | **Documentation** |
| `src/utils/slashCommandSections/bigPictureEventStorm.ts` | 61-63, 117-119 | **Documentation** |
| `src/commands/add-foundation-bounded-context-help.ts` | 46 | **See also** |
| `src/commands/add-aggregate-to-foundation-help.ts` | 58 | **See also** |
| `src/commands/add-command-to-foundation-help.ts` | 62 | **See also** |
| `src/commands/__tests__/add-foundation-event-storm-items.test.ts` | 6, 148-159 | **Tests** |
| `src/commands/__tests__/auto-create-big-picture-event-storm-work-unit.test.ts` | 141, 262-263 | **Tests** |
| `src/commands/__tests__/bootstrap-event-storm-reminder.test.ts` | 91, 162 | **Tests** |
| `src/utils/slashCommandSections/__tests__/bigPictureEventStorm.test.ts` | 37 | **Tests** |

### Reference Command: `add-command-to-foundation`

| File | Lines | Role |
|------|-------|------|
| `src/commands/add-command-to-foundation.ts` | 23-162 | **Implementation** |
| `src/commands/add-command-to-foundation-help.ts` | 4-69 | **Help** |
| `src/cli/program.ts` | 138, 324 | **Registration** |
| `src/utils/slashCommandSections/bigPictureEventStorm.ts` | 66-67 | **Documentation** |
| `src/commands/add-foundation-bounded-context-help.ts` | 47 | **See also** |
| `src/commands/add-aggregate-to-foundation-help.ts` | 59 | **See also** |
| `src/commands/add-domain-event-to-foundation-help.ts` | 62 | **See also** |
| `src/commands/__tests__/add-foundation-event-storm-items.test.ts` | 7, 221-232 | **Tests** |
| `src/utils/slashCommandSections/__tests__/bigPictureEventStorm.test.ts` | 38 | **Tests** |

### Related: `show-foundation-event-storm`

| File | Lines | Role |
|------|-------|------|
| `src/commands/show-foundation-event-storm.ts` | 22-145 | **Implementation** — filters `!item.deleted` on line 67 |
| `src/commands/show-foundation-event-storm-help.ts` | 4-31 | **Help** |
| `src/cli/program.ts` | 134, 320 | **Registration** |
| `src/help.ts` | 1198-1202 | **Global help** |
| `src/commands/discover-foundation.ts` | 436 | **Documentation** |
| `src/commands/bootstrap.ts` | 199, 228 | **Documentation** |
| `src/utils/slashCommandSections/bigPictureEventStorm.ts` | 70, 124 | **Documentation** |
| Multiple help files | Various | **See also** in 6+ help files |
| Multiple test files | Various | **Tests** in 5+ test files |

### Related: `derive-tags-from-foundation` (NOT IMPLEMENTED)

| File | Lines | Role |
|------|-------|------|
| `src/utils/slashCommandSections/bigPictureEventStorm.ts` | 146 | **Documentation only** — referenced in slash command section |
| `src/utils/slashCommandSections/__tests__/bigPictureEventStorm.test.ts` | 113, 121-122 | **Tests** — tests existence in documentation string |

**No implementation file, no registration in program.ts, no exported function.**

---

## Key Implementation Details

### Data Model (from `src/types/index.ts` and `src/types/generic-foundation.ts`)

```typescript
// Foundation-level event storm structure
interface FoundationEventStorm extends EventStormBase {
  level: 'big_picture';
}

interface EventStormBase {
  items: EventStormItem[];      // All artifacts
  nextItemId: number;           // Auto-increment counter
}

// Every item has soft-delete fields via ItemWithId
interface ItemWithId {
  id: number;
  deleted: boolean;
  text: string;
  type: string;
  createdAt: string;
}

// Child items link to parent via boundedContextId
interface EventStormAggregate extends EventStormItemBase {
  type: 'aggregate';
  boundedContextId?: number;  // set on items inside a bounded context
}
```

### Soft-Delete Already Filters in Show Command

`src/commands/show-foundation-event-storm.ts` line 67:
```typescript
let items = foundation.eventStorm.items.filter(item => !item.deleted);
```

### Soft-Delete Already Filters in FOUNDATION.md Generation

`src/generators/foundation-md.ts` line 329:
```typescript
const boundedContexts = foundation.eventStorm.items.filter(
  (item): item is EventStormBoundedContext =>
    item.type === 'bounded_context' && !item.deleted
);
```

And per-context item filters on lines 98, 106, 114 all include `!item.deleted`.

### Transaction Pattern (from existing add commands)

All add commands use `fileManager.transaction()` for atomic updates:
```typescript
await fileManager.transaction<GenericFoundation>(foundationPath, async data => {
  // mutate data.eventStorm.items
});
```

### FOUNDATION.md Regeneration

All add commands call `generateFoundationMdCommand({ cwd })` after mutation.

---

## Files to Create for BUG-096

### New Source Files

| File | Purpose |
|------|---------|
| `src/commands/remove-foundation-bounded-context.ts` | Implementation + CLI registration |
| `src/commands/remove-foundation-bounded-context-help.ts` | Help definition |
| `src/commands/remove-aggregate-from-foundation.ts` | Implementation + CLI registration |
| `src/commands/remove-aggregate-from-foundation-help.ts` | Help definition |
| `src/commands/remove-domain-event-from-foundation.ts` | Implementation + CLI registration |
| `src/commands/remove-domain-event-from-foundation-help.ts` | Help definition |
| `src/commands/remove-command-from-foundation.ts` | Implementation + CLI registration |
| `src/commands/remove-command-from-foundation-help.ts` | Help definition |

### New Test Files

| File | Purpose |
|------|---------|
| `src/commands/__tests__/remove-foundation-bounded-context.test.ts` | Tests for bounded context removal |
| `src/commands/__tests__/remove-aggregate-from-foundation.test.ts` | Tests for aggregate removal |
| `src/commands/__tests__/remove-domain-event-from-foundation.test.ts` | Tests for domain event removal |
| `src/commands/__tests__/remove-command-from-foundation.test.ts` | Tests for command removal |

### Files to Modify

| File | Change |
|------|--------|
| `src/cli/program.ts` | Add 4 import lines (~line 135-138) + 4 register calls (~line 321-324) |
| `src/help.ts` | Add help text for 4 new commands (~line 1210) |
| `src/utils/slashCommandSections/bigPictureEventStorm.ts` | Add remove command examples in documentation |
| `src/commands/add-foundation-bounded-context-help.ts` | Add to `relatedCommands` |
| `src/commands/add-aggregate-to-foundation-help.ts` | Add to `relatedCommands` |
| `src/commands/add-domain-event-to-foundation-help.ts` | Add to `relatedCommands` |
| `src/commands/add-command-to-foundation-help.ts` | Add to `relatedCommands` |
| `src/commands/show-foundation-event-storm-help.ts` | Add to `relatedCommands` |
| `src/commands/discover-foundation.ts` | Add remove commands to auto-created work unit description (~line 432-436) |
| `src/commands/bootstrap.ts` | Add remove commands to bootstrap help text (~line 196-199, 225-228) |

---

## Design Decisions

### 1. Cascade Deletion for Bounded Contexts

**Recommendation:** Refuse by default, allow `--cascade` flag.

When removing a bounded context:
- Without `--cascade`: Error listing child item count + suggestion to use `--cascade`
- With `--cascade`: Soft-delete the bounded context + all items where `boundedContextId === context.id`

### 2. Soft-Delete Pattern

Set `deleted: true` and add `deletedAt` timestamp — consistent with work-unit stable-index pattern.

The show and FOUNDATION.md generators already filter `!item.deleted`.

### 3. Draft vs Finalized Foundation

Check for `foundation.json.draft` first, fall back to `foundation.json` — same as existing add commands.

**Note:** Current add commands ONLY use `foundation.json` (not draft). The remove commands should follow the same pattern.

### 4. FOUNDATION.md Regeneration

Call `generateFoundationMdCommand({ cwd })` after every removal — same as add commands.
