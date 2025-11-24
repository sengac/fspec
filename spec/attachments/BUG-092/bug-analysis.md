# BUG-092: Duplicate Question IDs from generate-example-mapping-from-event-storm

## Summary

The `generate-example-mapping-from-event-storm` command creates duplicate question IDs by using `workUnit.questions.length` instead of `workUnit.nextQuestionId` to assign IDs, violating the stable indices system.

## Root Cause Analysis

### The Stable Indices System

fspec uses a **stable indices system** (Migration 001) that prevents data loss during concurrent operations:

1. **ID Counters**: Each work unit maintains `nextRuleId`, `nextExampleId`, `nextQuestionId`, `nextNoteId`
2. **Monotonic Increment**: IDs are assigned using `workUnit.nextQuestionId++` (never reused)
3. **Soft Delete**: Items are marked `deleted: true` but never removed from array (preserves indices)
4. **Selected Flag**: Questions use `selected: true` instead of array removal when answered
5. **Compaction**: Only `compact-work-unit` command removes deleted items and resets counters

### The Bug

**File**: `src/commands/generate-example-mapping-from-event-storm.ts`

**Line 79**:
```typescript
const nextQuestionId = workUnit.questions.length;
```

**Line 136**:
```typescript
workUnit.questions.push({
  id: nextQuestionId + questionsAdded,
  text: questionText,
  deleted: false,
  answer: undefined,
  createdAt: new Date().toISOString(),
});
```

**Problem**: Uses array length instead of `workUnit.nextQuestionId`, causing ID reuse.

### How Duplicates Occur

**Scenario 1: After generate-example-mapping-from-event-storm**

```json
{
  "questions": [
    {"id": 0, "text": "...", "deleted": false},
    {"id": 1, "text": "...", "deleted": false},
    {"id": 2, "text": "...", "deleted": false}
  ],
  "nextQuestionId": 0  // ❌ NEVER UPDATED!
}
```

**Scenario 2: Subsequent add-question command**

```bash
fspec add-question WORK-001 "@human: New question?"
```

Since `nextQuestionId` is still 0, it creates:
```json
{"id": 0, "text": "@human: New question?", "deleted": false}
```

**Result**: Duplicate ID 0!

**Scenario 3: After soft delete + new questions**

```json
{
  "questions": [
    {"id": 0, "text": "...", "deleted": true},   // Soft deleted
    {"id": 1, "text": "...", "deleted": true},   // Soft deleted
    {"id": 2, "text": "...", "deleted": false}
  ],
  "nextQuestionId": 3
}
```

Running `generate-example-mapping-from-event-storm` calculates:
```typescript
nextQuestionId = workUnit.questions.length;  // 3 (correct by accident)
```

But if called again after more operations, breaks.

## Impact

### Data Corruption

1. **Duplicate IDs**: Multiple questions with same ID
2. **Index Confusion**: `answer-question` uses array index, but display uses ID
3. **Selected State Desync**: Questions with same ID cause ambiguous selected state
4. **Validation Failures**: `generate-scenarios` counts unanswered questions incorrectly

### Affected Commands

- ✅ **add-question**: Uses `workUnit.nextQuestionId++` (correct)
- ✅ **add-rule**: Uses `workUnit.nextRuleId++` (correct)
- ✅ **add-example**: Uses `workUnit.nextExampleId++` (correct)
- ❌ **generate-example-mapping-from-event-storm**: Uses `questions.length` (BROKEN)
- ✅ **answer-question**: Uses array index (correct, but confused by duplicates)
- ✅ **compact-work-unit**: Resets counters to `array.length` (correct, only used after cleanup)

## Real-World Example (AGENT-001 in codelet project)

```json
{
  "questions": [
    {"id": 0, "selected": true, "answered": true, "answer": "..."},
    {"id": 1, "selected": true, "answered": true, "answer": "..."},
    {"id": 2, "selected": true, "answered": true, "answer": "..."},
    {"id": 3, "selected": true, "answered": true, "answer": "..."},
    {"id": 4, "selected": true, "answered": true, "answer": "..."},
    {"id": 0, "selected": true, "answered": true, "answer": "..."},  // ❌ DUPLICATE
    {"id": 1, "selected": true, "answered": true, "answer": "..."},  // ❌ DUPLICATE
    {"id": 2, "selected": true, "answered": true, "answer": "..."},  // ❌ DUPLICATE
    {"id": 3, "selected": false},  // ❌ UNANSWERED
    {"id": 4, "selected": false},  // ❌ UNANSWERED
    {"id": 5, "selected": false},  // ❌ UNANSWERED
    {"id": 6, "selected": false},  // ❌ UNANSWERED
    {"id": 7, "selected": false}   // ❌ UNANSWERED
  ],
  "nextQuestionId": 8
}
```

**Timeline**:
1. Event Storm → Example Mapping generated questions 0-4 (using `questions.length`)
2. `nextQuestionId` never updated, stayed at 0
3. Manual `add-question` commands created questions 0-7 (using `nextQuestionId++`)
4. Result: Duplicate IDs 0-4

## Fix Requirements

### Must Update nextQuestionId Counter

The fix MUST:

1. ✅ Initialize `nextQuestionId` if undefined (backward compatibility)
2. ✅ Use `workUnit.nextQuestionId` instead of `questions.length`
3. ✅ Increment counter as items are added: `workUnit.nextQuestionId++`
4. ✅ Apply same pattern to rules and examples (consistency)
5. ✅ Match the pattern used by `add-question.ts`, `add-rule.ts`, `add-example.ts`
6. ✅ Never reset counters (only `compact-work-unit` does that)

### Code Pattern (Correct)

```typescript
// Initialize nextQuestionId if undefined (backward compatibility)
if (workUnit.nextQuestionId === undefined) {
  workUnit.nextQuestionId = 0;
}

// Add questions using monotonic counter
for (const item of workUnit.eventStorm.items) {
  if (item.type === 'hotspot') {
    const hotspot = item as EventStormHotspot;
    if (hotspot.concern) {
      let concernText = hotspot.concern.trim();
      if (!concernText.endsWith('?')) {
        concernText += '?';
      }
      const questionText = `@human: ${concernText}`;
      workUnit.questions.push({
        id: workUnit.nextQuestionId++,  // ✅ Use counter and increment
        text: questionText,
        deleted: false,
        createdAt: new Date().toISOString(),
      });
      questionsAdded++;
    }
  }
}
```

### Code Pattern for Rules (Also Broken)

```typescript
// Initialize nextRuleId if undefined (backward compatibility)
if (workUnit.nextRuleId === undefined) {
  workUnit.nextRuleId = 0;
}

// Add rules using monotonic counter
for (const item of workUnit.eventStorm.items) {
  if (item.type === 'policy') {
    const policy = item as EventStormPolicy;
    if (policy.when && policy.then) {
      const whenText = pascalCaseToSentence(policy.when);
      const thenText = pascalCaseToSentence(policy.then);
      const ruleText = `System must ${thenText} after ${whenText}`;
      workUnit.rules.push({
        id: workUnit.nextRuleId++,  // ✅ Use counter and increment
        text: ruleText,
        deleted: false,
        createdAt: new Date().toISOString(),
      });
      rulesAdded++;
    }
  }
}
```

### Code Pattern for Examples (Currently Disabled)

```typescript
// Initialize nextExampleId if undefined (backward compatibility)
if (workUnit.nextExampleId === undefined) {
  workUnit.nextExampleId = 0;
}

// Add examples using monotonic counter
// (Currently disabled per BUG-089, but if re-enabled must use counter)
for (const item of workUnit.eventStorm.items) {
  if (item.type === 'event') {
    const event = item as EventStormEvent;
    const eventSentence = pascalCaseToSentence(event.text);
    const exampleText = `User ${eventSentence} and system responds`;
    workUnit.examples.push({
      id: workUnit.nextExampleId++,  // ✅ Use counter and increment
      text: exampleText,
      deleted: false,
      createdAt: new Date().toISOString(),
    });
    examplesAdded++;
  }
}
```

## Testing Requirements

### Unit Tests

1. **Test ID Assignment**: Verify IDs are sequential and monotonic
2. **Test Counter Increment**: Verify `nextQuestionId` increments correctly
3. **Test No Duplicates**: Run command twice, verify no duplicate IDs
4. **Test Backward Compatibility**: Work units without `nextQuestionId` are initialized
5. **Test After Soft Delete**: Command works correctly when questions have `deleted: true`
6. **Test Mixed with Manual Commands**: Generate → manual add → generate again

### Integration Tests

1. **Full Workflow**: Event Storm → Example Mapping → Answer Questions → Generate Scenarios
2. **Concurrent Operations**: Generate mapping + add-question in parallel (transaction safety)
3. **Validation**: `generate-scenarios` correctly identifies unanswered questions

## Related Code

### Correct Implementations (Reference)

- `src/commands/add-question.ts:48-55` - Initializes and increments `nextQuestionId`
- `src/commands/add-rule.ts:45-52` - Initializes and increments `nextRuleId`
- `src/commands/add-example.ts:47-54` - Initializes and increments `nextExampleId`
- `src/commands/answer-question.ts:87` - Uses `nextRuleId++` when adding rules from answers

### Migration Reference

- `src/migrations/migrations/001-stable-indices.ts:121` - Sets `nextQuestionId = questions.length` during migration
- `src/migrations/migrations/001-stable-indices.ts:48,74,103` - Sets counters for all collections

### Compaction Reference

- `src/commands/compact-work-unit.ts:126-131` - Resets all counters to `array.length` after removing deleted items

## Acceptance Criteria

### When Fixed

1. ✅ `generate-example-mapping-from-event-storm` initializes `nextQuestionId` if undefined
2. ✅ Questions are assigned IDs using `workUnit.nextQuestionId++`
3. ✅ Rules are assigned IDs using `workUnit.nextRuleId++`
4. ✅ Examples (if re-enabled) are assigned IDs using `workUnit.nextExampleId++`
5. ✅ No duplicate IDs are created across multiple invocations
6. ✅ Counter persists correctly after command execution
7. ✅ Backward compatible with work units lacking counters
8. ✅ All existing tests pass
9. ✅ New tests validate ID uniqueness and counter behavior

### Verification Commands

```bash
# Create work unit and run event storm
fspec create-story TEST "Test Story"
fspec update-work-unit-status TEST-001 specifying
fspec add-domain-event TEST-001 UserLoggedIn
fspec add-hotspot TEST-001 "Security model?" --concern "How to secure?"

# Generate Example Mapping (should not create duplicates)
fspec generate-example-mapping-from-event-storm TEST-001

# Add manual question (should get next sequential ID)
fspec add-question TEST-001 "@human: Manual question?"

# Generate again (should not reuse IDs)
fspec add-hotspot TEST-001 "Another concern?" --concern "What about this?"
fspec generate-example-mapping-from-event-storm TEST-001

# Verify no duplicates
fspec show-work-unit TEST-001 | grep -A 50 "Questions:"
# Should show sequential IDs with no duplicates
```

## Priority

**HIGH** - Data corruption issue affecting core workflow (Event Storm → Example Mapping → Scenarios)

## Resolution Notes

This bug is a violation of the stable indices architecture introduced in Migration 001. The fix is straightforward: replace `questions.length` with `nextQuestionId++` and apply the same pattern to rules. The pattern is already correctly implemented in all other `add-*` commands.
