# EXMAP-010 Ultrathinking Research: Big Picture Event Storm in foundation.json

**Work Unit:** EXMAP-010
**Title:** Big Picture Event Storm in foundation.json
**Parent:** EXMAP-004 (Event Storming discovery phase)
**Date:** 2025-11-16
**Researcher:** Claude (AI Agent)

---

## Executive Summary

EXMAP-010 will add a `eventStorm` section to foundation.json for storing **Big Picture Event Storming** data at the project/foundation level. This is strategically different from work unit-level Event Storming (already in `workUnit.eventStorm`).

**CRITICAL ARCHITECTURAL LESSON FROM EXMAP-011:**
- fspec must contain **ZERO SEMANTIC CODE**
- fspec provides **STRUCTURE only**, AI agents provide **SEMANTIC INTERPRETATION**
- Example: EXMAP-008 and EXMAP-009 were blocked because they contained semantic pattern matching

---

## 1. Current State Analysis

### 1.1 Existing Event Storm Support (Work Unit Level)

**Location:** `spec/work-units.json` → `workUnit.eventStorm`

**Type Definition** (from `src/types/index.ts:122-131`):
```typescript
export interface EventStorm {
  level: 'process_modeling' | 'software_design';
  sessionDate?: string;
  facilitator?: string;
  participants?: string[];
  items: EventStormItem[];
  nextItemId: number;
  suggestedTags?: SuggestedTags; // ⚠️ SEMANTIC CODE - should be removed
}
```

**Item Types Supported:**
- `EventStormEvent` (orange - domain events)
- `EventStormCommand` (blue - commands)
- `EventStormAggregate` (yellow - aggregates)
- `EventStormPolicy` (purple - policies)
- `EventStormHotspot` (red - questions/concerns)
- `EventStormExternalSystem` (pink - external systems)
- `EventStormBoundedContext` (blue tape - bounded contexts)

**Existing Commands:**
- `fspec add-domain-event`
- `fspec add-command`
- `fspec add-aggregate`
- `fspec add-policy`
- `fspec add-hotspot`
- `fspec add-external-system`
- `fspec add-bounded-context`
- ~~`fspec suggest-tags-from-events`~~ (REMOVED - EXMAP-011)
- ~~`fspec sync-tags-with-event-storm`~~ (BLOCKED - EXMAP-009)
- `fspec show-event-storm` ✅ (ADDED - EXMAP-011) - returns raw JSON, no semantics

### 1.2 Foundation.json Structure

**Current Type:** `GenericFoundation` (from `src/types/generic-foundation.ts`)

**Current Fields:**
```typescript
export interface GenericFoundation {
  version: string;
  project: ProjectIdentity;
  problemSpace: ProblemSpace;
  solutionSpace: SolutionSpace;
  subFoundations?: string[];
  architectureDiagrams?: MermaidDiagram[];
  constraints?: Constraints;
  personas?: Persona[];
  // ❌ NO eventStorm field (yet)
}
```

**Key Observations:**
1. Foundation uses `GenericFoundation` type, not the old `Foundation` type
2. Schema is validated with Ajv (`src/schemas/generic-foundation.schema.json`)
3. Mermaid diagrams are validated with `mermaid.parse()`
4. Foundation supports hierarchical structure via `subFoundations`

---

## 2. Big Picture Event Storming vs Process Modeling

### 2.1 Event Storming Levels (from Alberto Brandolini)

| Aspect | Big Picture ES | Process Modeling ES | Software Design ES |
|--------|---------------|---------------------|-------------------|
| **Scope** | Entire business domain | Single process/workflow | Single aggregate/bounded context |
| **Participants** | Domain experts, stakeholders, entire team | Domain experts, developers | Developers, architects |
| **Duration** | 4-8 hours | 2-4 hours | 1-2 hours |
| **Output** | Bounded contexts, pivotal events, process flows | Detailed process steps, policies, commands | Aggregate design, event sequences |
| **Storage in fspec** | `foundation.json` ✅ (EXMAP-010) | `workUnit.eventStorm` ✅ (exists) | `workUnit.eventStorm` ✅ (exists) |
| **Purpose** | Strategic domain understanding | Tactical feature discovery | Implementation design |

### 2.2 What Goes in Foundation vs Work Unit?

**Foundation-level Event Storm (`foundation.json`):**
- ✅ **Bounded Contexts** - Strategic domain boundaries (e.g., "User Management", "Billing", "Analytics")
- ✅ **Pivotal Events** - Major system-wide events that cross boundaries
- ✅ **Major Aggregates** - Core domain aggregates (e.g., "WorkUnit", "Feature", "Tag")
- ✅ **External Systems** - Third-party integrations affecting multiple contexts
- ✅ **System-wide Hotspots** - Architectural concerns or uncertainties

**Work Unit-level Event Storm (`workUnit.eventStorm`):**
- ✅ **Process Events** - Specific events within a single user story
- ✅ **Commands** - Actions triggered by users/system
- ✅ **Policies** - Business rules for specific scenarios
- ✅ **Feature-specific Aggregates** - Tactical aggregates for this story
- ✅ **Local Hotspots** - Questions about this specific feature

**Rule of Thumb:**
- Foundation = "What are the **major domains** in our system?"
- Work Unit = "What **events happen** when a user does X?"

---

## 3. Architectural Lessons from EXMAP-011

### 3.1 The Zero-Semantics Principle

**WRONG APPROACH** (EXMAP-008, EXMAP-009):
```typescript
// ❌ Semantic pattern matching in fspec
if (system.text.toLowerCase().includes('oauth')) {
  suggestions.push({ category: 'technical', tagName: '@oauth' });
}

if (eventNames.some(name => name.includes('Login'))) {
  featureGroup = 'authentication';
}
```

**CORRECT APPROACH** (EXMAP-011):
```typescript
// ✅ Return raw structural data, no interpretation
export async function showEventStorm(options) {
  const activeItems = workUnit.eventStorm.items.filter(
    (item) => !item.deleted
  );
  return { success: true, data: activeItems };
}
```

**Principle:**
- **fspec is DUMB** - Returns raw Event Storm JSON data
- **AI agents are SMART** - Interpret semantics using language understanding

### 3.2 Commands for EXMAP-010 (NO Semantic Code)

Based on EXMAP-011 lessons, EXMAP-010 commands should:

**✅ ACCEPTABLE:**
- Add Event Storm items to foundation.json
- Show Event Storm items as JSON
- Delete Event Storm items
- Update Event Storm item properties
- Filter deleted items

**❌ NOT ACCEPTABLE:**
- Suggest tag names from Event Storm data
- Classify bounded contexts by domain
- Infer relationships between aggregates
- Auto-generate capabilities from events
- Pattern match event names for categorization

**Command Design:**
```bash
# ✅ Structural operations only
fspec add-foundation-bounded-context "User Management"
fspec add-foundation-aggregate "Work Unit"
fspec add-foundation-pivotal-event "Work Unit Status Changed"
fspec show-foundation-event-storm
fspec show-foundation-event-storm --type=bounded_context

# ❌ NO semantic operations
fspec suggest-capabilities-from-events  # WRONG
fspec auto-classify-bounded-contexts    # WRONG
fspec infer-aggregate-relationships      # WRONG
```

---

## 4. Type Design for Foundation-Level Event Storm

### 4.1 Proposed GenericFoundation Extension

**Add to `src/types/generic-foundation.ts`:**

```typescript
export interface GenericFoundation {
  version: string;
  project: ProjectIdentity;
  problemSpace: ProblemSpace;
  solutionSpace: SolutionSpace;
  subFoundations?: string[];
  architectureDiagrams?: MermaidDiagram[];
  constraints?: Constraints;
  personas?: Persona[];

  // ✅ NEW: Foundation-level Event Storm
  eventStorm?: FoundationEventStorm;
}
```

### 4.2 FoundationEventStorm Type

**Option A: Reuse EventStorm Type (Simple)**
```typescript
import type { EventStorm } from './index';

export interface GenericFoundation {
  // ...
  eventStorm?: EventStorm; // Reuse existing type
}
```

**Pros:**
- No duplicate code
- Commands can share logic
- Consistent item types

**Cons:**
- `level` field is process-specific ('process_modeling' | 'software_design')
- Big Picture ES doesn't fit these levels

**Option B: New FoundationEventStorm Type (Recommended)**
```typescript
export interface FoundationEventStorm {
  level: 'big_picture'; // Fixed value
  sessionDate?: string;
  facilitator?: string;
  participants?: string[];
  items: EventStormItem[]; // Reuse existing union type
  nextItemId: number;
  // ❌ Remove suggestedTags - semantic code
}
```

**Pros:**
- Clear semantic separation
- Correct level designation ('big_picture')
- No semantic fields (suggestedTags removed)

**Cons:**
- Small duplication with EventStorm type

**Option C: Shared Base Type (Most Flexible)**
```typescript
// Shared base interface
export interface EventStormBase {
  sessionDate?: string;
  facilitator?: string;
  participants?: string[];
  items: EventStormItem[];
  nextItemId: number;
}

// Work unit-level Event Storm
export interface EventStorm extends EventStormBase {
  level: 'process_modeling' | 'software_design';
  suggestedTags?: SuggestedTags; // To be removed in future cleanup
}

// Foundation-level Event Storm
export interface FoundationEventStorm extends EventStormBase {
  level: 'big_picture';
  // No suggestedTags - enforces zero-semantics
}
```

**Pros:**
- DRY (shared fields in base type)
- Clear level distinction
- Type safety for level-specific operations
- Easy to remove suggestedTags later

**Cons:**
- More complex type hierarchy

**RECOMMENDATION: Option C** - Best balance of flexibility, type safety, and future-proofing.

### 4.3 Schema Updates Required

**File:** `src/schemas/generic-foundation.schema.json`

Add optional `eventStorm` property:
```json
{
  "type": "object",
  "properties": {
    "version": { "type": "string" },
    "project": { "$ref": "#/definitions/ProjectIdentity" },
    "problemSpace": { "$ref": "#/definitions/ProblemSpace" },
    "solutionSpace": { "$ref": "#/definitions/SolutionSpace" },
    "eventStorm": { "$ref": "#/definitions/FoundationEventStorm" }
  },
  "definitions": {
    "FoundationEventStorm": {
      "type": "object",
      "properties": {
        "level": { "type": "string", "const": "big_picture" },
        "sessionDate": { "type": "string", "format": "date-time" },
        "facilitator": { "type": "string" },
        "participants": { "type": "array", "items": { "type": "string" } },
        "items": { "type": "array", "items": { "$ref": "#/definitions/EventStormItem" } },
        "nextItemId": { "type": "number" }
      },
      "required": ["level", "items", "nextItemId"]
    }
  }
}
```

---

## 5. Command Implementation Plan

### 5.1 Commands to Implement (Zero Semantics)

**Add Commands:**
1. `fspec add-foundation-bounded-context <name> [--description <desc>]`
2. `fspec add-foundation-aggregate <name> [--description <desc>]`
3. `fspec add-foundation-event <name> [--description <desc>]`
4. `fspec add-foundation-external-system <name> [--description <desc>]`
5. `fspec add-foundation-hotspot <concern>`

**Query Commands:**
6. `fspec show-foundation-event-storm [--type <type>]`
7. `fspec list-foundation-bounded-contexts`

**Management Commands:**
8. `fspec remove-foundation-event-storm-item <id>`
9. `fspec update-foundation-event-storm-item <id> --text <new-text>`

**NO Semantic Commands:**
- ❌ `fspec suggest-capabilities-from-events`
- ❌ `fspec auto-tag-from-bounded-contexts`
- ❌ `fspec infer-domain-relationships`

### 5.2 Command Architecture Pattern

**Example: `add-foundation-bounded-context.ts`**
```typescript
export interface AddFoundationBoundedContextOptions {
  name: string;
  description?: string;
  cwd?: string;
}

export interface AddFoundationBoundedContextResult {
  success: boolean;
  boundedContextId?: number;
  error?: string;
}

export async function addFoundationBoundedContext(
  options: AddFoundationBoundedContextOptions
): Promise<AddFoundationBoundedContextResult> {
  const { name, description, cwd = process.cwd() } = options;
  const foundationFile = `${cwd}/spec/foundation.json`;

  try {
    // Read foundation.json
    const foundation = await fileManager.readJSON<GenericFoundation>(
      foundationFile,
      { /* default */ }
    );

    // Initialize eventStorm if missing
    if (!foundation.eventStorm) {
      foundation.eventStorm = {
        level: 'big_picture',
        items: [],
        nextItemId: 1,
      };
    }

    // Create bounded context item
    const boundedContext: EventStormBoundedContext = {
      id: foundation.eventStorm.nextItemId++,
      type: 'bounded_context',
      text: name,
      color: null,
      deleted: false,
      createdAt: new Date().toISOString(),
      description,
    };

    // Add to items array
    foundation.eventStorm.items.push(boundedContext);

    // Write back using transaction for locking
    await fileManager.transaction<GenericFoundation>(
      foundationFile,
      async (data) => {
        Object.assign(data, foundation);
      }
    );

    return { success: true, boundedContextId: boundedContext.id };
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : 'Unknown error',
    };
  }
}
```

**Key Points:**
- Uses `fileManager.transaction()` for atomic updates
- Initializes `eventStorm` if missing
- Auto-increments `nextItemId`
- No semantic logic - just structural data
- Returns simple success/error result

### 5.3 Show Command Pattern (Following EXMAP-011)

```typescript
export async function showFoundationEventStorm(
  options: ShowFoundationEventStormOptions
): Promise<ShowFoundationEventStormResult> {
  const { type, cwd = process.cwd() } = options;
  const foundationFile = `${cwd}/spec/foundation.json`;

  try {
    const foundation = await fileManager.readJSON<GenericFoundation>(
      foundationFile,
      { /* default */ }
    );

    if (!foundation.eventStorm) {
      return { success: false, error: 'No Event Storm data in foundation' };
    }

    // Filter out deleted items
    let items = foundation.eventStorm.items.filter(item => !item.deleted);

    // Optional type filtering (structural, not semantic)
    if (type) {
      items = items.filter(item => item.type === type);
    }

    return { success: true, data: items };
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : 'Unknown error',
    };
  }
}
```

---

## 6. Integration Points

### 6.1 File Manager Integration

**Use existing `fileManager` utility:**
- `fileManager.readJSON<GenericFoundation>()` - Read foundation.json
- `fileManager.transaction<GenericFoundation>()` - Atomic write with locking

**Example:**
```typescript
await fileManager.transaction<GenericFoundation>(
  foundationFile,
  async (data) => {
    if (!data.eventStorm) {
      data.eventStorm = { level: 'big_picture', items: [], nextItemId: 1 };
    }
    data.eventStorm.items.push(newItem);
    data.eventStorm.nextItemId++;
  }
);
```

### 6.2 Validation Integration

**Schema Validation:**
- Update `src/schemas/generic-foundation.schema.json`
- Use existing `validateFoundationJson()` function
- Run validation before writing foundation.json

**Mermaid Validation:**
- No change needed (Event Storm doesn't use Mermaid)

### 6.3 FOUNDATION.md Generation

**Update `src/generators/foundation-md.ts`:**

Add section for Event Storm visualization (if items exist):

```markdown
## Event Storm (Big Picture)

### Bounded Contexts
- **User Management**: Authentication, authorization, profiles
- **Work Management**: Kanban, work units, epics
- **Specification Management**: Features, scenarios, tags

### Pivotal Events
- Work Unit Status Changed
- Feature File Validated
- Coverage Linked

### Major Aggregates
- Work Unit
- Feature File
- Tag Registry
```

---

## 7. Example Mapping Considerations for EXMAP-010

### 7.1 User Story

```
As a AI agent performing Big Picture Event Storming
I want to capture foundation-level bounded contexts, pivotal events, and major aggregates
So that I can understand the strategic domain structure before diving into tactical work units
```

### 7.2 Business Rules (Blue Cards)

1. Foundation Event Storm must use level='big_picture' (not 'process_modeling' or 'software_design')
2. All Event Storm items must use same types as work unit-level (EventStormItem union type)
3. Foundation Event Storm is OPTIONAL in foundation.json (not required for all projects)
4. Commands must NOT include semantic logic (tag suggestion, classification, inference)
5. Deleted items (deleted=true) must be filtered out when showing Event Storm data
6. nextItemId must auto-increment for each new item to ensure stable IDs
7. Foundation Event Storm commands must use fileManager.transaction() for atomic updates

### 7.3 Examples (Green Cards)

1. Add bounded context "User Management" to foundation Event Storm → creates item with type='bounded_context', text='User Management', deleted=false
2. Show foundation Event Storm filtered by type='bounded_context' → returns only bounded context items, excludes deleted items
3. Foundation.json has no eventStorm section → add-foundation-bounded-context initializes eventStorm with level='big_picture', items=[], nextItemId=1
4. Foundation Event Storm has 3 items, 1 deleted → show-foundation-event-storm returns 2 items only

### 7.4 Questions (Red Cards)

1. **@human**: Should foundation Event Storm support same item types as work unit Event Storm (events, commands, aggregates, policies, hotspots, external systems, bounded contexts)?
   - **Answer**: Yes - reuse EventStormItem union type for consistency

2. **@human**: Should we auto-generate capabilities in solutionSpace from bounded contexts?
   - **Answer**: NO - semantic code violation. AI agents can suggest manually.

3. **@human**: Should we validate that bounded contexts in foundation match bounded contexts in work units?
   - **Answer**: NO - they serve different purposes (strategic vs tactical)

4. **@human**: Should we remove `suggestedTags` from EventStorm type as part of EXMAP-010?
   - **Answer**: Not in EXMAP-010 (separate cleanup task). Keep focus on foundation integration only.

---

## 8. Potential Pitfalls and Solutions

### 8.1 Pitfall: Semantic Creep

**Risk:** Commands slowly add "helpful" semantic features
- Auto-suggest capabilities from bounded contexts
- Infer relationships between aggregates
- Pattern match event names

**Solution:**
- Code review checklist: "Does this command interpret semantics?"
- Add comment header: `// ZERO SEMANTICS: Returns structural data only`
- Write tests that verify NO pattern matching occurs

### 8.2 Pitfall: Type Confusion

**Risk:** Mixing foundation-level and work unit-level Event Storm concepts

**Solution:**
- Use distinct `level` values ('big_picture' vs 'process_modeling'/'software_design')
- Namespace commands: `add-foundation-bounded-context` vs `add-bounded-context`
- Document clearly in help text

### 8.3 Pitfall: Duplicate Bounded Contexts

**Risk:** Same bounded context name in foundation AND work units

**Solution:**
- This is OKAY - they serve different purposes
- Foundation: Strategic domain understanding
- Work Unit: Tactical implementation scope
- Don't try to enforce uniqueness (would be semantic validation)

### 8.4 Pitfall: Over-Engineering

**Risk:** Adding too many foundation-specific Event Storm features

**Solution:**
- Start minimal: bounded-context, aggregate, pivotal-event commands only
- Add more commands ONLY when clear use case emerges
- Follow YAGNI (You Aren't Gonna Need It)

---

## 9. Testing Strategy

### 9.1 Test Coverage Requirements

**Scenarios to Test:**
1. ✅ Add bounded context to foundation with no existing eventStorm section
2. ✅ Add aggregate to foundation with existing eventStorm section
3. ✅ Show foundation Event Storm filtered by type
4. ✅ Show foundation Event Storm with deleted items (should be excluded)
5. ✅ Foundation Event Storm uses level='big_picture'
6. ✅ nextItemId auto-increments correctly

### 9.2 Test File Structure

**Feature File:** `spec/features/big-picture-event-storm-in-foundation.feature`

**Test File:** `src/commands/__tests__/add-foundation-bounded-context.test.ts`

**Example Test:**
```typescript
describe('Scenario: Add bounded context to foundation with no Event Storm', () => {
  it('should initialize eventStorm and add bounded context', async () => {
    // @step Given foundation.json has no eventStorm section
    const foundation: GenericFoundation = {
      version: '2.0.0',
      project: { name: 'test', vision: 'test', projectType: 'cli-tool' },
      problemSpace: { primaryProblem: { title: 'test', description: 'test', impact: 'high' } },
      solutionSpace: { overview: 'test', capabilities: [] },
    };

    await fileManager.writeJSON(foundationFile, foundation);

    // @step When I run "fspec add-foundation-bounded-context 'User Management'"
    const result = await addFoundationBoundedContext({ name: 'User Management' });

    // @step Then eventStorm section should be created with level='big_picture'
    expect(result.success).toBe(true);
    const updated = await fileManager.readJSON<GenericFoundation>(foundationFile);
    expect(updated.eventStorm).toBeDefined();
    expect(updated.eventStorm.level).toBe('big_picture');

    // @step And bounded context should be added to items array
    expect(updated.eventStorm.items).toHaveLength(1);
    expect(updated.eventStorm.items[0].type).toBe('bounded_context');
    expect(updated.eventStorm.items[0].text).toBe('User Management');
  });
});
```

---

## 10. Success Criteria

**EXMAP-010 is complete when:**

1. ✅ `GenericFoundation` type extended with optional `eventStorm: FoundationEventStorm`
2. ✅ `FoundationEventStorm` type defined with `level='big_picture'`
3. ✅ JSON schema updated and validated with Ajv
4. ✅ Commands implemented (add-foundation-bounded-context, add-foundation-aggregate, show-foundation-event-storm)
5. ✅ All commands contain ZERO semantic code
6. ✅ Tests written with @step comments for all scenarios
7. ✅ 100% test coverage (all scenarios linked to tests and implementation)
8. ✅ FOUNDATION.md generator updated to show Event Storm section
9. ✅ Build passes, all tests pass, quality checks pass
10. ✅ Code review confirms zero-semantics compliance

---

## 11. Follow-Up Work Units (Out of Scope for EXMAP-010)

**Future Enhancements:**
1. **EXMAP-012**: Remove `suggestedTags` from `EventStorm` type (cleanup)
2. **EXMAP-013**: Add mermaid diagram generator from Event Storm data (visualization)
3. **EXMAP-014**: Event Storm import/export commands (JSON format)
4. **EXMAP-015**: Event Storm merge utility (combine multiple sessions)

**NOT TO BE DONE (Semantic Code):**
- ❌ Auto-generate capabilities from bounded contexts
- ❌ Suggest tags from Event Storm data
- ❌ Infer aggregate relationships
- ❌ Classify bounded contexts by domain

---

## 12. References

**Event Storming:**
- Alberto Brandolini: "Introducing Event Storming" (book)
- https://www.eventstorming.com/

**fspec Architecture:**
- `spec/CLAUDE.md` - Claude developer guidelines
- `spec/FOUNDATION.md` - Project foundation
- `src/types/generic-foundation.ts` - GenericFoundation type
- `src/types/index.ts` - EventStorm types

**EXMAP-011 Lessons:**
- EXMAP-008: Blocked (semantic tag discovery)
- EXMAP-009: Blocked (semantic tag sync)
- EXMAP-011: ✅ Completed (show-event-storm with zero semantics)

---

## Conclusion

EXMAP-010 is architecturally sound and follows zero-semantics principle established by EXMAP-011.

**Key Decisions:**
1. Use `FoundationEventStorm` type with `level='big_picture'`
2. Reuse `EventStormItem` union type for consistency
3. Implement structural commands only (no semantic logic)
4. Follow EXMAP-011 command pattern (read → filter → return JSON)
5. Use `fileManager.transaction()` for atomic updates

**Next Steps:**
1. Attach this research document to EXMAP-010
2. Complete Example Mapping with rules, examples, questions
3. Generate scenarios from Example Mapping
4. Move to testing phase and write tests with @step comments
5. Implement commands following zero-semantics principle
6. Link coverage and validate

---

**Document Version:** 1.0
**Status:** Ready for Example Mapping
