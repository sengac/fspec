# Event Storm Types Research

## Key Types from src/types/index.ts

### EventStorm Interface (lines 123-131)
```typescript
export interface EventStorm {
  level: 'process_modeling' | 'software_design';
  sessionDate?: string;
  facilitator?: string;
  participants?: string[];
  items: EventStormItem[]; // Array of Event Storm artifacts
  nextItemId: number;
  suggestedTags?: SuggestedTags;
}
```

### EventStormItem Union Type (lines 105-112)
Discriminated union of all Event Storm item types:
- EventStormEvent
- EventStormCommand
- EventStormAggregate
- EventStormPolicy
- EventStormHotspot
- EventStormExternalSystem
- EventStormBoundedContext

### EventStormItemBase (lines 43-49)
All items extend this base:
```typescript
export interface EventStormItemBase extends ItemWithId {
  color: string;
  timestamp?: number;
  boundedContext?: string;
  relatedTo?: number[];
}
```

### ItemWithId (lines 2-8)
Base interface with soft-delete support:
```typescript
export interface ItemWithId {
  id: number;
  text: string;
  deleted: boolean; // CRITICAL: Filter this out!
  createdAt: string;
  deletedAt?: string;
}
```

## Implementation Notes

1. **Data source**: `workUnit.eventStorm.items` array
2. **Filter**: MUST exclude items where `deleted === true`
3. **Item types**: event, command, aggregate, policy, hotspot, external_system, bounded_context
4. **Output**: Return filtered items array as JSON
5. **No semantics**: Just return raw structural data, no interpretation
