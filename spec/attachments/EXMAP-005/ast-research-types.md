# AST Research: Event Storm Type Patterns

## Pattern Analysis: ItemWithId Base Interface

Researched existing type patterns in src/types/index.ts to ensure consistency.

### Existing Patterns Found:
1. **ItemWithId base interface** - Used for RuleItem, ExampleItem, ArchitectureNoteItem
   - Fields: id (number), text (string), deleted (boolean), createdAt (string), deletedAt (optional string)
   - Pattern: Stable IDs with soft-delete support

2. **Discriminated unions** - Not currently used in types/index.ts but recommended for Event Storm items

3. **Optional fields** - Used extensively (timestamp?, boundedContext?, relatedTo?)

### Implementation Decision:
- EventStormItemBase extends ItemWithId (reuses stable ID pattern)
- Added EventStorm-specific fields: color, timestamp, boundedContext, relatedTo
- Used discriminated union with 'type' field for type safety
- Each item type (event, command, aggregate, etc.) has specific required fields

### Type Safety Benefits:
- Discriminated union prevents invalid combinations (can't have triggersEvent on an aggregate)
- TypeScript narrows types based on 'type' field
- Compile-time checking for required fields per item type
