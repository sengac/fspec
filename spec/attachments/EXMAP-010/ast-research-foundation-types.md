# AST Research: Foundation Types for EXMAP-010

**Work Unit:** EXMAP-010
**Research Date:** 2025-11-16
**Files Analyzed:**
- `src/types/generic-foundation.ts`
- `src/types/index.ts` (EventStorm types)

---

## GenericFoundation Type Structure

**File:** `src/types/generic-foundation.ts:22-46`

```typescript
export interface GenericFoundation {
  $schema?: string;
  version: string;
  project: ProjectIdentity;
  problemSpace: ProblemSpace;
  solutionSpace: SolutionSpace;
  subFoundations?: string[];
  architectureDiagrams?: MermaidDiagram[];
  constraints?: Constraints;
  personas?: Persona[];
  // ⚠️ NO eventStorm field exists yet - EXMAP-010 will add this
}
```

---

## EventStorm Types (Work Unit Level)

**File:** `src/types/index.ts:122-131`

```typescript
export interface EventStorm {
  level: 'process_modeling' | 'software_design';
  sessionDate?: string;
  facilitator?: string;
  participants?: string[];
  items: EventStormItem[];
  nextItemId: number;
  suggestedTags?: SuggestedTags; // Semantic code - should be removed
}
```

---

## EventStormItem Union Type

**File:** `src/types/index.ts:105-112`

```typescript
export type EventStormItem =
  | EventStormEvent
  | EventStormCommand
  | EventStormAggregate
  | EventStormPolicy
  | EventStormHotspot
  | EventStormExternalSystem
  | EventStormBoundedContext;
```

### EventStormBoundedContext Type

**File:** `src/types/index.ts:96-102`

```typescript
export interface EventStormBoundedContext
  extends Omit<EventStormItemBase, 'color'> {
  type: 'bounded_context';
  color: null;
  description?: string;
  itemIds?: number[];
}
```

### EventStormItemBase Type

**File:** `src/types/index.ts:43-49`

```typescript
export interface EventStormItemBase extends ItemWithId {
  color: string;
  timestamp?: number;
  boundedContext?: string;
  relatedTo?: number[];
}
```

### ItemWithId Type

**File:** `src/types/index.ts:2-8`

```typescript
export interface ItemWithId {
  id: number;
  text: string;
  deleted: boolean; // CRITICAL for filtering
  createdAt: string;
  deletedAt?: string;
}
```

---

## Required Changes for EXMAP-010

### 1. Extend GenericFoundation Type

**File:** `src/types/generic-foundation.ts`

Add optional `eventStorm` field:

```typescript
export interface GenericFoundation {
  // ... existing fields
  eventStorm?: FoundationEventStorm; // NEW
}
```

### 2. Create FoundationEventStorm Type

**Recommendation:** Use shared base type approach

```typescript
// Shared base interface (reusable)
export interface EventStormBase {
  sessionDate?: string;
  facilitator?: string;
  participants?: string[];
  items: EventStormItem[];
  nextItemId: number;
}

// Foundation-level Event Storm
export interface FoundationEventStorm extends EventStormBase {
  level: 'big_picture'; // Fixed value
}

// Work unit-level Event Storm (existing, update to extend base)
export interface EventStorm extends EventStormBase {
  level: 'process_modeling' | 'software_design';
  suggestedTags?: SuggestedTags;
}
```

### 3. Update JSON Schema

**File:** `src/schemas/generic-foundation.schema.json`

Add `eventStorm` property definition:

```json
{
  "properties": {
    "eventStorm": {
      "$ref": "#/definitions/FoundationEventStorm"
    }
  },
  "definitions": {
    "FoundationEventStorm": {
      "type": "object",
      "required": ["level", "items", "nextItemId"],
      "properties": {
        "level": { "type": "string", "const": "big_picture" },
        "sessionDate": { "type": "string", "format": "date-time" },
        "facilitator": { "type": "string" },
        "participants": { "type": "array", "items": { "type": "string" } },
        "items": { "type": "array" },
        "nextItemId": { "type": "number", "minimum": 1 }
      }
    }
  }
}
```

---

## FileManager Integration

### Read Foundation

```typescript
const foundation = await fileManager.readJSON<GenericFoundation>(
  'spec/foundation.json',
  {
    version: '2.0.0',
    project: { /* defaults */ },
    problemSpace: { /* defaults */ },
    solutionSpace: { /* defaults */ },
  }
);
```

### Update Foundation (Atomic)

```typescript
await fileManager.transaction<GenericFoundation>(
  'spec/foundation.json',
  async (data) => {
    if (!data.eventStorm) {
      data.eventStorm = {
        level: 'big_picture',
        items: [],
        nextItemId: 1,
      };
    }
    data.eventStorm.items.push(newItem);
    data.eventStorm.nextItemId++;
  }
);
```

---

## Implementation Checklist

- [ ] Add `FoundationEventStorm` type to `src/types/generic-foundation.ts`
- [ ] Add `EventStormBase` interface for DRY
- [ ] Update `EventStorm` to extend `EventStormBase`
- [ ] Add `eventStorm?: FoundationEventStorm` to `GenericFoundation`
- [ ] Update `src/schemas/generic-foundation.schema.json`
- [ ] Implement `addFoundationBoundedContext()` function
- [ ] Implement `showFoundationEventStorm()` function
- [ ] Register CLI commands in `src/index.ts`
- [ ] Write tests with @step comments
- [ ] Link coverage

---

## Zero-Semantics Verification

✅ **Acceptable Operations:**
- Initialize eventStorm section if missing
- Add EventStormItem to items array
- Increment nextItemId
- Filter items by `deleted=false`
- Filter items by `type` (structural property)

❌ **NOT Acceptable (Semantic Code):**
- Suggest capabilities from bounded contexts
- Classify bounded contexts by domain
- Infer relationships between items
- Pattern match text for categorization
- Auto-generate tags from Event Storm data

---

## Conclusion

GenericFoundation type needs minimal changes:
1. Add optional `eventStorm` field
2. Create `FoundationEventStorm` type with `level='big_picture'`
3. Reuse existing `EventStormItem` union type
4. Update JSON schema for validation

All operations will be **structural only** - no semantic interpretation.
