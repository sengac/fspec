# AST Research: Foundation Command Pattern

## Research Goal
Analyze existing `add-foundation-bounded-context.ts` to understand the pattern for implementing new foundation Event Storm commands.

## Key Findings

### 1. Function Structure

The command file contains 4 main functions:

1. **Core Function** (`addFoundationBoundedContext`) - Business logic
   - Takes `text: string` and optional `options` parameter
   - Returns `Promise<{ success: boolean; message?: string }>`
   - Handles reading foundation.json, updating it, and auto-regenerating FOUNDATION.md

2. **Transaction Callback** (arrow function within core) - Data mutation
   - Initializes `eventStorm` section if missing
   - Creates new item with proper structure
   - Increments `nextItemId` counter

3. **Command Handler** (`addFoundationBoundedContextCommand`) - CLI wrapper
   - Calls core function
   - Handles errors with chalk formatting
   - Sets exit codes appropriately

4. **Command Registration** (`registerAddFoundationBoundedContextCommand`) - Commander.js integration
   - Registers command with program
   - Defines argument structure
   - Wires up action handler

### 2. Data Flow Pattern

```
CLI Input
  ↓
Command Handler (try/catch, exit codes)
  ↓
Core Function (business logic)
  ↓
fileManager.transaction (atomic update)
  ↓
Transaction Callback (data mutation)
  ↓
generateFoundationMdCommand (auto-regenerate markdown)
  ↓
Success/Error Response
```

### 3. Critical Implementation Details

#### A. Reading foundation.json with Defaults

```typescript
const foundation = await fileManager.readJSON<GenericFoundation>(
  foundationPath,
  {
    version: '2.0.0',
    project: {
      name: '',
      vision: '',
      projectType: 'other' as const,
    },
    problemSpace: {
      primaryProblem: {
        title: '',
        description: '',
        impact: 'medium' as const,
      },
    },
    solutionSpace: {
      overview: '',
      capabilities: [],
    },
  }
);
```

**Key Point**: Always provide defaults structure to handle missing file case.

#### B. Atomic Transaction Pattern

```typescript
await fileManager.transaction<GenericFoundation>(
  foundationPath,
  async data => {
    // Initialize eventStorm section if missing
    if (!data.eventStorm) {
      data.eventStorm = {
        level: 'big_picture',
        items: [],
        nextItemId: 1,
      };
    }

    // Create item
    const boundedContext: EventStormBoundedContext = {
      id: data.eventStorm.nextItemId,
      type: 'bounded_context',
      text,
      color: null,
      deleted: false,
      createdAt: new Date().toISOString(),
    };

    // Add and increment
    data.eventStorm.items.push(boundedContext);
    data.eventStorm.nextItemId++;
  }
);
```

**Key Point**: Transaction ensures atomic file updates (writes to .tmp then renames).

#### C. Auto-Regenerate FOUNDATION.md

```typescript
await generateFoundationMdCommand({ cwd });
```

**Key Point**: Always regenerate markdown after updating foundation.json.

#### D. Item Structure

```typescript
const boundedContext: EventStormBoundedContext = {
  id: data.eventStorm.nextItemId,  // Auto-increment
  type: 'bounded_context',          // Item type
  text,                             // User-provided text
  color: null,                      // Reserved for future use
  deleted: false,                   // Soft-delete flag
  createdAt: new Date().toISOString(), // Timestamp
};
```

### 4. New Commands Implementation Pattern

For `add-aggregate-to-foundation`, `add-domain-event-to-foundation`, `add-command-to-foundation`:

#### Differences from bounded context command:

1. **Additional Parameter**: `contextName` - which bounded context does this belong to?
2. **Validation Required**: Check that bounded context exists
3. **Link to Context**: Add `boundedContextId` field linking to parent context
4. **New Item Types**: `'aggregate'`, `'domain_event'`, `'command'`

#### Item Structure for Child Items

```typescript
interface EventStormAggregate {
  id: number;
  type: 'aggregate';
  text: string;
  boundedContextId: number;  // NEW: Link to parent context
  color: string | null;
  deleted: boolean;
  createdAt: string;
  description?: string;      // NEW: Optional description
}
```

#### Validation Logic

```typescript
// Find bounded context by name
const boundedContext = data.eventStorm.items.find(
  item => item.type === 'bounded_context' && item.text === contextName
);

if (!boundedContext) {
  throw new Error(`Bounded context "${contextName}" not found`);
}
```

#### Creating Child Items

```typescript
const aggregate: EventStormAggregate = {
  id: data.eventStorm.nextItemId,
  type: 'aggregate',
  text: aggregateName,
  boundedContextId: boundedContext.id,  // Link to parent
  color: null,
  deleted: false,
  createdAt: new Date().toISOString(),
  ...(description && { description }),  // Conditional field
};

data.eventStorm.items.push(aggregate);
data.eventStorm.nextItemId++;
```

### 5. File Organization

**File naming convention**:
- `src/commands/add-aggregate-to-foundation.ts`
- `src/commands/add-domain-event-to-foundation.ts`
- `src/commands/add-command-to-foundation.ts`

**Help file convention** (if needed):
- `src/commands/add-aggregate-to-foundation-help.ts`
- `src/commands/add-domain-event-to-foundation-help.ts`
- `src/commands/add-command-to-foundation-help.ts`

### 6. Imports Required

```typescript
import chalk from 'chalk';
import { Command } from 'commander';
import { fileManager } from '../utils/file-manager';
import { generateFoundationMdCommand } from './generate-foundation-md';
import type {
  GenericFoundation,
  EventStormAggregate,
  EventStormDomainEvent,
  EventStormCommand
} from '../types/foundation';
```

### 7. Type Definitions Needed

Need to add to `src/types/foundation.ts`:

```typescript
export interface EventStormAggregate extends EventStormItem {
  type: 'aggregate';
  boundedContextId: number;
  description?: string;
}

export interface EventStormDomainEvent extends EventStormItem {
  type: 'domain_event';
  boundedContextId: number;
  description?: string;
}

export interface EventStormCommand extends EventStormItem {
  type: 'command';
  boundedContextId: number;
  description?: string;
}

export type EventStormItem =
  | EventStormBoundedContext
  | EventStormAggregate
  | EventStormDomainEvent
  | EventStormCommand;
```

## Implementation Checklist

Based on this research, implementing the three new commands requires:

- [ ] Add type definitions to `src/types/foundation.ts`
- [ ] Create `src/commands/add-aggregate-to-foundation.ts`
- [ ] Create `src/commands/add-domain-event-to-foundation.ts`
- [ ] Create `src/commands/add-command-to-foundation.ts`
- [ ] Each command must:
  - [ ] Accept `<contextName>` and `<text>` arguments
  - [ ] Support optional `--description` flag
  - [ ] Validate bounded context exists
  - [ ] Create item with `boundedContextId` linking to parent
  - [ ] Use atomic transaction pattern
  - [ ] Auto-regenerate FOUNDATION.md
  - [ ] Export registration function for `src/index.ts`
- [ ] Register commands in `src/index.ts`
- [ ] Update `show-foundation-event-storm.ts` to add filtering
- [ ] Update `generate-foundation-md.ts` to render Event Storm section

## Code Reusability

**Can extract shared logic**:
- Bounded context lookup by name → shared utility function
- Item creation with auto-increment → similar pattern across all three

**Cannot reuse**:
- Each command needs separate registration (Commander.js limitation)
- Type-specific item structure (aggregate vs event vs command)

## Testing Strategy

Based on scenarios in feature file:

1. **Happy Path Tests**: Create aggregate/event/command with valid context
2. **Validation Tests**: Try to add item to non-existent context (should fail)
3. **Optional Fields**: Test with and without --description flag
4. **Filtering Tests**: Verify show-foundation-event-storm filters work
5. **Markdown Generation**: Verify generate-foundation-md renders Event Storm section

## Conclusion

The pattern is well-established and consistent. New commands should follow the same structure with these additions:
1. Parent context validation
2. `boundedContextId` linking
3. Optional `--description` flag support
4. New item types in foundation.json

All three commands (`add-aggregate-to-foundation`, `add-domain-event-to-foundation`, `add-command-to-foundation`) can follow this exact pattern with only the item type changing.
