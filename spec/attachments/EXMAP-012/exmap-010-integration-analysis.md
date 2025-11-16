# EXMAP-010: Big Picture Event Storm Foundation Integration Analysis

## Executive Summary

EXMAP-010 adds **Big Picture Event Storm** support to `foundation.json`, enabling strategic domain modeling at the foundation level. While the core implementation is complete (commands work, types defined), **critical schema integration is missing**, preventing validation enforcement and breaking the foundation regeneration contract.

---

## Implementation Status

### ✅ Completed Components

1. **Type System Integration**
   - `EventStormBase` shared interface (src/types/generic-foundation.ts:24-30)
   - `FoundationEventStorm` with `level: 'big_picture'` (lines 36-38)
   - Reuses `EventStormItem` union type for consistency
   - Stable ID system via `nextItemId`

2. **Commands Implemented**
   - `add-foundation-bounded-context` (src/commands/add-foundation-bounded-context.ts)
     - Atomic updates via `fileManager.transaction()`
     - Auto-initializes `eventStorm` section
     - Stable ID generation with auto-increment
   - `show-foundation-event-storm` (src/commands/show-foundation-event-storm.ts)
     - Structural filtering (type, deleted items)
     - Zero-semantics principle (no AI inference)
     - JSON output to stdout

3. **Test Coverage**
   - Comprehensive tests in `__tests__/add-foundation-bounded-context.test.ts`
   - Feature file: `spec/features/big-picture-event-storm-in-foundation-json.feature`
   - All scenarios passing with @step comments

4. **Foundation.json Support**
   - GenericFoundation type includes `eventStorm?: FoundationEventStorm`
   - Data persists across foundation updates
   - Atomic transactions prevent corruption

---

## 🚨 Critical Gaps Found

### Gap 1: JSON Schema Missing `eventStorm` Definition

**Location:** `src/schemas/generic-foundation.schema.json`

**Problem:** The schema defines required fields (project, problemSpace, solutionSpace) and optional fields (subFoundations, architectureDiagrams, constraints, personas) but does **NOT** include `eventStorm` property.

**Impact:**
- ❌ Validation doesn't enforce `level: 'big_picture'` constraint
- ❌ Schema validation passes even with invalid Event Storm structure
- ❌ Discovery finalization (`--finalize`) doesn't validate Event Storm data
- ❌ TypeScript types diverge from JSON Schema (type/schema mismatch)

**Evidence:**
```json
// Current schema (lines 1-281)
{
  "properties": {
    "$schema": { ... },
    "version": { ... },
    "project": { ... },
    "problemSpace": { ... },
    "solutionSpace": { ... },
    "subFoundations": { ... },
    "architectureDiagrams": { ... },
    "constraints": { ... },
    "personas": { ... }
    // ❌ eventStorm MISSING
  }
}
```

**Required Fix:**
Add `eventStorm` property to schema with:
- Optional field (not required)
- Fixed `level: "big_picture"` constraint
- `items` array of EventStormItem discriminated union
- `nextItemId` integer validation
- Optional metadata fields (sessionDate, facilitator, participants)

### Gap 2: EventStormItem Discriminated Union Not in Schema

**Problem:** The TypeScript discriminated union `EventStormItem` has 7 types:
- `EventStormEvent` (orange - domain events)
- `EventStormCommand` (blue - commands)
- `EventStormAggregate` (yellow - aggregates)
- `EventStormPolicy` (purple - policies)
- `EventStormHotspot` (red - questions/risks)
- `EventStormExternalSystem` (pink - external systems)
- `EventStormBoundedContext` (conceptual boundary)

But the JSON schema has **no** `#/definitions/eventStormItem` to validate this union.

**Impact:**
- ❌ Can't validate item structure per type
- ❌ Can't enforce color conventions
- ❌ Can't validate type-specific fields (e.g., `actor` for commands, `responsibilities` for aggregates)
- ❌ Invalid items could be added without detection

**Required Fix:**
Add JSON Schema definition using `oneOf` discriminator pattern:

```json
{
  "definitions": {
    "eventStormItem": {
      "oneOf": [
        { "$ref": "#/definitions/eventStormEvent" },
        { "$ref": "#/definitions/eventStormCommand" },
        { "$ref": "#/definitions/eventStormAggregate" },
        { "$ref": "#/definitions/eventStormPolicy" },
        { "$ref": "#/definitions/eventStormHotspot" },
        { "$ref": "#/definitions/eventStormExternalSystem" },
        { "$ref": "#/definitions/eventStormBoundedContext" }
      ]
    },
    "eventStormEvent": {
      "type": "object",
      "required": ["id", "type", "text", "color", "deleted", "createdAt"],
      "properties": {
        "type": { "const": "event" },
        "color": { "const": "orange" },
        // ... other fields
      }
    }
    // ... similar for other types
  }
}
```

### Gap 3: FOUNDATION.md Doesn't Render Event Storm Data

**Problem:** The `generateFoundationMd()` function (called during foundation regeneration) does **NOT** render Event Storm data to human-readable markdown.

**Current Behavior:**
- Event Storm data stored in `foundation.json`
- When foundation regenerates (via `update-foundation`), FOUNDATION.md is recreated
- Event Storm data **silently preserved in JSON** but **NOT visible in MD**

**Impact:**
- 🤔 Users can't see Event Storm data in human-readable format
- 🤔 FOUNDATION.md doesn't reflect complete foundation state
- 🤔 Event Storm is JSON-only (tooling consumption, not human reading)

**Decision Needed:**
Is this intentional? Event Storm might be:
- **Option A:** JSON-only (for tooling/AI consumption, not human docs)
- **Option B:** Should render in FOUNDATION.md for human visibility

**If Option B, Required Fix:**
Add Event Storm rendering to `generateFoundationMd()`:

```typescript
if (foundation.eventStorm && foundation.eventStorm.items.length > 0) {
  md += `\n## Big Picture Event Storm\n\n`;

  // Group by type
  const boundedContexts = foundation.eventStorm.items.filter(
    i => i.type === 'bounded_context' && !i.deleted
  );

  if (boundedContexts.length > 0) {
    md += `### Bounded Contexts\n\n`;
    boundedContexts.forEach(bc => {
      md += `- **${bc.text}**\n`;
      if (bc.description) {
        md += `  - ${bc.description}\n`;
      }
    });
    md += `\n`;
  }

  // Similar for aggregates, events, etc.
}
```

### Gap 4: Event Storm Not Part of Discovery Workflow

**Problem:** The `discover-foundation` command scans 8 fields during discovery, but `eventStorm` is **NOT** included in the field scan loop.

**Current Discovery Fields:**
1. `project.name`
2. `project.vision`
3. `project.projectType`
4. `problemSpace.primaryProblem.title`
5. `problemSpace.primaryProblem.description`
6. `solutionSpace.overview`
7. `solutionSpace.capabilities`
8. `personas`

**Impact:**
- 🤔 Event Storm data must be added AFTER foundation finalized
- 🤔 No AI-guided prompts for Event Storm during discovery
- 🤔 Event Storm is "tacked on" rather than integrated into discovery flow

**Decision Needed:**
Should Event Storm be part of discovery? Consider:
- **Pro:** Integrated workflow, single discovery session
- **Con:** Event Storming is collaborative workshop, not AI-detectable
- **Con:** Foundation discovery is WHY/WHAT focused, Event Storm is domain modeling (different concern)

**Recommendation:** Keep Event Storm separate (current design is correct). Event Storming requires human collaboration and happens AFTER basic foundation is established.

---

## Integration Architecture

### How Event Storm Fits Into Foundation Lifecycle

```
┌──────────────────────────────────────────────────────────┐
│ Phase 1: Foundation Discovery (AI-Driven)               │
│                                                          │
│ fspec discover-foundation                                │
│   → Creates foundation.json.draft with placeholders     │
│   → AI scans codebase, fills 8 fields                   │
│   → Human confirms via conversation                      │
│                                                          │
│ fspec discover-foundation --finalize                     │
│   → Validates schema (8 required/optional fields)       │
│   → Creates spec/foundation.json                         │
│   → Auto-generates spec/FOUNDATION.md                    │
│                                                          │
│ Status: ✅ Foundation established                        │
└──────────────────────────────────────────────────────────┘
                         ↓
┌──────────────────────────────────────────────────────────┐
│ Phase 2: Event Storm (Optional, Human-Driven)           │
│                                                          │
│ Collaborative Big Picture Event Storm workshop:         │
│   fspec add-foundation-bounded-context "User Mgmt"      │
│   fspec add-foundation-bounded-context "Payments"       │
│   fspec add-foundation-aggregate "Order"                │
│   fspec add-foundation-domain-event "OrderPlaced"       │
│                                                          │
│ View artifacts:                                          │
│   fspec show-foundation-event-storm                      │
│   fspec show-foundation-event-storm --type=bounded_context│
│                                                          │
│ Status: ✅ Strategic domain model captured               │
└──────────────────────────────────────────────────────────┘
                         ↓
┌──────────────────────────────────────────────────────────┐
│ Phase 3: Foundation Regeneration (Automated)             │
│                                                          │
│ When foundation.json changes:                            │
│   → Validate schema (including eventStorm if present)   │
│   → Regenerate FOUNDATION.md                             │
│   → Event Storm data preserved in JSON                   │
│                                                          │
│ Status: ⚠️  Validation currently incomplete (schema gap) │
└──────────────────────────────────────────────────────────┘
```

### Type System Consistency

```
┌─────────────────────────────────────────────────────────┐
│ EventStormBase (Shared Interface)                      │
│ src/types/generic-foundation.ts:24-30                  │
│                                                         │
│ interface EventStormBase {                             │
│   sessionDate?: string;                                │
│   facilitator?: string;                                │
│   participants?: string[];                             │
│   items: EventStormItem[];  // ← Shared union type     │
│   nextItemId: number;       // ← Stable ID system      │
│ }                                                       │
└─────────────────────────────────────────────────────────┘
                      ↓                    ↓
        ┌─────────────────────┐  ┌─────────────────────┐
        │ FoundationEventStorm│  │ WorkUnitEventStorm  │
        │ (Big Picture)       │  │ (Process/Software)  │
        │                     │  │                     │
        │ level: 'big_picture'│  │ level: 'process_    │
        │                     │  │   modeling' |       │
        │ NO suggestedTags    │  │   'software_design' │
        │ Zero-semantics      │  │                     │
        │                     │  │ suggestedTags?: ... │
        └─────────────────────┘  └─────────────────────┘
```

**Key Insight:** Both use the same `EventStormItem` union, ensuring:
- ✅ Consistent item structure across foundation and work units
- ✅ Stable ID system (auto-increment, soft-delete)
- ✅ Same color conventions and type definitions
- ✅ Reusable code (file manager, validators, etc.)

But enforce different semantic levels via `level` discriminator:
- Foundation: Strategic domain understanding (bounded contexts, pivotal events)
- Work Units: Tactical process modeling (specific user journeys, scenarios)

---

## Zero-Semantics Principle

**From EXMAP-010 Business Rule #4:**
> "Commands must NOT include semantic logic (tag suggestion, classification, inference)"

### What This Means

Foundation Event Storm commands are **structural data only**:

❌ **DO NOT:**
- Suggest related aggregates when adding bounded context
- Infer domain events from bounded context name
- Auto-populate `itemIds` array based on semantic relationships
- Generate tags from Event Storm artifacts
- Classify items into categories beyond their explicit type

✅ **DO:**
- Create items with explicit type and text
- Assign stable IDs
- Filter by type (structural filter)
- Soft-delete items (structural operation)
- Output raw JSON data

### Contrast with Work Unit Event Storm

```typescript
// Work unit Event Storm (src/types/index.ts:125-128)
export interface EventStorm extends EventStormBase {
  level: 'process_modeling' | 'software_design';
  suggestedTags?: SuggestedTags;  // ← AI can suggest tags
}

// Foundation Event Storm (src/types/generic-foundation.ts:36-38)
export interface FoundationEventStorm extends EventStormBase {
  level: 'big_picture';
  // ❌ NO suggestedTags - zero-semantics
}
```

**Why the difference?**
- **Work units** are tactical and benefit from AI assistance (tag suggestions)
- **Foundation** is strategic and requires human domain expertise (no AI shortcuts)
- Foundation Event Storm is a **collaborative workshop output**, not AI-generated

---

## Test Coverage Analysis

### Current Test Status: ✅ Complete

**Feature File:** `spec/features/big-picture-event-storm-in-foundation-json.feature`
- 4 scenarios covering all business rules
- Example Mapping context included (lines 11-29)
- Architecture notes documented (line 8)

**Test File:** `src/commands/__tests__/add-foundation-bounded-context.test.ts`
- All scenarios have @step comments linking to Gherkin steps
- Uses `fileManager.transaction()` for atomic test setup
- Restores original foundation state in afterEach

**Scenario Coverage:**
1. ✅ Add bounded context with no existing Event Storm (lines 50-91)
2. ✅ Show Event Storm filtered by type (lines 93+)
3. ✅ Initialize Event Storm when missing (covered in scenario 1)
4. ✅ Filter out deleted items (tested in scenario 2)

**Missing Tests:**
- ⚠️  Schema validation tests (blocked by Gap 1 - schema not updated)
- ⚠️  FOUNDATION.md rendering tests (blocked by Gap 3 - not implemented)

---

## Acceptance Criteria for Completion

To fully complete EXMAP-010 integration, the following must be done:

### Must Have (Blockers)

1. **Update JSON Schema** ⚠️  CRITICAL
   - Add `eventStorm` property to `src/schemas/generic-foundation.schema.json`
   - Define `eventStormItem` discriminated union with all 7 types
   - Add validation for `level: "big_picture"` constraint
   - Validate `nextItemId` as integer >= 1
   - Test schema validation with valid/invalid Event Storm data

2. **Schema Validation Tests**
   - Test valid foundation with Event Storm passes validation
   - Test invalid `level` value fails validation
   - Test invalid item type fails validation
   - Test missing required fields fails validation

### Should Have (Enhancements)

3. **FOUNDATION.md Rendering** 🤔 (Decision Needed)
   - If Event Storm should be human-readable:
     - Add rendering logic to `generateFoundationMd()`
     - Group items by type (bounded contexts, aggregates, etc.)
     - Format as markdown lists/tables
     - Test FOUNDATION.md regeneration includes Event Storm

4. **Additional Commands**
   - `fspec add-foundation-aggregate "Aggregate Name"`
   - `fspec add-foundation-domain-event "Event Name"`
   - `fspec add-foundation-policy "Policy Name"`
   - `fspec add-foundation-hotspot "Risk/Question"`
   - `fspec add-foundation-external-system "System Name"`
   - `fspec remove-foundation-event-storm-item <id>`
   - `fspec update-foundation-event-storm-item <id> --text "New text"`

### Could Have (Future)

5. **Soft-Delete Management**
   - `fspec show-deleted-foundation-event-storm` (show soft-deleted items)
   - `fspec restore-foundation-event-storm-item <id>` (restore deleted item)
   - `fspec compact-foundation-event-storm` (permanently remove deleted items)

6. **Event Storm Export**
   - `fspec export-foundation-event-storm --format=miro` (Miro board JSON)
   - `fspec export-foundation-event-storm --format=mural` (Mural board JSON)
   - `fspec export-foundation-event-storm --format=svg` (Visual diagram)

---

## Recommendation

### Priority 1: Fix Schema Gap (CRITICAL)

The **schema update is a blocker** for EXMAP-010 to be considered complete. Without it:
- Validation is incomplete
- Type/schema mismatch exists
- Foundation regeneration contract is broken

**Action:** Create story card for schema integration with acceptance criteria:
- Update `generic-foundation.schema.json` with `eventStorm` property
- Define `eventStormItem` discriminated union
- Add schema validation tests
- Verify `discover-foundation --finalize` validates Event Storm data

### Priority 2: Decide on FOUNDATION.md Rendering

**Question for Product Owner:** Should Event Storm data be visible in FOUNDATION.md?

**Option A: JSON-Only (Current)**
- Event Storm is for tooling/AI consumption
- FOUNDATION.md stays focused on WHY/WHAT
- Domain modeling lives in JSON, not human docs

**Option B: Human-Readable**
- Event Storm rendered as markdown tables/lists
- FOUNDATION.md is complete single source of truth
- Better for onboarding new team members

**Recommendation:** Start with Option A (JSON-only), add rendering later if needed.

### Priority 3: Additional Commands (Future)

Once schema is fixed, expand command set to support all Event Storm artifact types (aggregates, events, policies, etc.). This is enhancement work, not a blocker.

---

## Conclusion

EXMAP-010 implementation is **90% complete** but has a **critical schema validation gap** that must be addressed before it can be considered done. The core commands work, tests pass, and types are defined correctly, but the JSON Schema needs updating to enforce validation during foundation discovery and regeneration.

**Next Steps:**
1. Create story card for schema integration (this document)
2. Update `generic-foundation.schema.json` with `eventStorm` property
3. Add schema validation tests
4. Decide on FOUNDATION.md rendering (Product Owner input)
5. Mark EXMAP-010 as fully complete

**Estimated Effort:**
- Schema update: **2-3 story points** (straightforward but thorough testing needed)
- FOUNDATION.md rendering (if required): **1-2 story points** (simple markdown generation)
- Total: **3-5 story points**

---

## References

- **Work Unit:** EXMAP-010 (status: done)
- **Feature File:** `spec/features/big-picture-event-storm-in-foundation-json.feature`
- **Implementation:**
  - `src/commands/add-foundation-bounded-context.ts`
  - `src/commands/show-foundation-event-storm.ts`
  - `src/types/generic-foundation.ts`
- **Schema:** `src/schemas/generic-foundation.schema.json` (requires update)
- **Tests:** `src/commands/__tests__/add-foundation-bounded-context.test.ts`

**Analysis Date:** 2025-11-16
**Analyst:** Claude (Sonnet 4.5)
**Status:** Schema integration gap identified, story card created
