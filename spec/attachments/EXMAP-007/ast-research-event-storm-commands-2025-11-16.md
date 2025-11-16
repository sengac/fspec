# AST Research: Event Storm Command Implementation Pattern

**Date**: 2025-11-16
**Work Unit**: EXMAP-007
**Purpose**: Analyze existing Event Storm commands to ensure new commands follow established patterns

## Files Analyzed

### 1. `src/commands/add-domain-event.ts`
- **Pattern**: Command function exports interface `AddDomainEventOptions` and `AddDomainEventResult`
- **Validation**: Checks work unit exists, not in done/blocked state
- **Initialization**: Creates `eventStorm` section if missing with `level: 'process_modeling'`, `items: []`, `nextItemId: 0`
- **Item Creation**: Uses discriminated union with `type: 'event'`, auto-incremented `id`, `color: 'orange'`, `deleted: false`, `createdAt` timestamp
- **Optional Flags**: `--timestamp` (number), `--bounded-context` (string)
- **Transaction**: Uses `fileManager.transaction()` for atomic writes
- **Return**: Returns `{ success: true, eventId: number }` on success

### 2. `src/commands/add-command.ts` (Event Storm command artifact)
- **Pattern**: Same structure as add-domain-event
- **Item Type**: `type: 'command'`
- **Color**: `color: 'blue'`
- **Additional Field**: `commandData?: string` (optional)

### 3. `src/commands/add-aggregate.ts`
- **Pattern**: Same structure as add-domain-event
- **Item Type**: `type: 'aggregate'`
- **Color**: `color: 'yellow'`
- **Additional Fields**: `responsibilities?: string` (from --responsibilities flag)

## Pattern Summary for EXMAP-007

All four new commands MUST follow this exact pattern:

### Command Structure Template
```typescript
export interface Add[Type]Options {
  workUnitId: string;
  text: string;
  timestamp?: number;
  boundedContext?: string;
  // Type-specific fields
  [typeSpecificField]?: string;
  cwd?: string;
}

export interface Add[Type]Result {
  success: boolean;
  error?: string;
  [type]Id?: number;
}
```

### Implementation Pattern
1. Validate work unit exists
2. Validate work unit not in done/blocked state
3. Initialize eventStorm section if missing
4. Create item with auto-incremented ID
5. Add type-specific fields based on flags
6. Append to items array
7. Increment nextItemId
8. Save with fileManager.transaction()
9. Return result with item ID

### Type-Specific Fields (from Perplexity research)

**add-policy**:
- `type: 'policy'`
- `color: 'purple'`
- `when?: string` (from --when flag)
- `then?: string` (from --then flag)

**add-hotspot**:
- `type: 'hotspot'`
- `color: 'red'`
- `concern?: string` (from --concern flag)

**add-external-system**:
- `type: 'external_system'`
- `color: 'pink'`
- `integrationType?: string` (from --type flag, values: REST_API, MESSAGE_QUEUE, DATABASE, THIRD_PARTY_SERVICE, FILE_SYSTEM)

**add-bounded-context**:
- `type: 'bounded_context'`
- `color: null` (conceptual boundary, not sticky note)
- `description?: string` (from --description flag)

## Key Architectural Decisions

1. **Consistency**: All commands follow identical structure (EXMAP-006 pattern)
2. **Stable IDs**: Use auto-incremented IDs, never reuse deleted IDs
3. **Soft Delete**: Set `deleted: true` instead of removing from array
4. **TypeScript Discriminated Unions**: Type field enables type-safe item handling
5. **Atomic Writes**: fileManager.transaction() prevents race conditions
6. **Color Coding**: Follows Event Storming standard (except bounded contexts)

## Refactoring Opportunities

**Potential shared utility** (NOT required for this work unit, future optimization):
- Extract common validation logic (work unit exists, not done/blocked)
- Extract eventStorm initialization logic
- Extract item creation pattern

**Decision**: Implement all four commands with duplicated code first (following EXMAP-006 pattern), then evaluate refactoring in separate work unit if duplication becomes problematic.

## References

- Existing implementations: `src/commands/add-domain-event.ts`, `src/commands/add-command.ts`, `src/commands/add-aggregate.ts`
- Event Storming research: `spec/attachments/EXMAP-007/perplexity-event-storming-artifacts-2025-11-16.md`
- Type definitions: `src/types.ts` (EventStormItem discriminated union)
