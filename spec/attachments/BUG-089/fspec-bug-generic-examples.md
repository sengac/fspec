# Bug: Generic and Unhelpful Examples Generated from Event Storm Domain Events

## Issue Summary

When transforming Event Storm domain events to Example Mapping examples via `fspec generate-example-mapping-from-event-storm`, the generated examples are generic, repetitive, and provide no concrete detail. All examples follow the same template pattern "User [event name] and is logged in", which defeats the purpose of Example Mapping (concrete examples illustrating business rules).

## Steps to Reproduce

1. Create a work unit with domain events:
   ```bash
   fspec create-story UI "Test Story"
   fspec update-work-unit-status UI-001 specifying
   fspec discover-event-storm UI-001

   fspec add-domain-event UI-001 "TrackPlayed"
   fspec add-domain-event UI-001 "TrackPaused"
   fspec add-domain-event UI-001 "VolumeChanged"
   fspec add-domain-event UI-001 "PlaylistCreated"
   ```

2. Transform to Example Mapping:
   ```bash
   fspec generate-example-mapping-from-event-storm UI-001
   ```

3. View generated examples:
   ```bash
   fspec show-work-unit UI-001
   ```

## Expected Behavior

Examples should be **concrete, specific scenarios** that illustrate how business rules apply:

```
Examples:
  [0] User plays track "Bohemian Rhapsody" from "All Tracks" library, track starts playing from 0:00
  [1] User pauses currently playing track at 1:23, playback stops and position is preserved
  [2] User adjusts volume from 50% to 75% while track is playing, volume changes immediately
  [3] User creates playlist "Workout Mix", empty playlist is added to sidebar
  [4] User adds 5 tracks to "Workout Mix" playlist via drag-and-drop, all tracks appear in playlist
  [5] User clicks next track while on last song with shuffle OFF, playback loops to first track
  [6] User clicks next track while on last song with shuffle ON, random track plays next
```

**Characteristics of good examples:**
- Specific data (track names, playlist names, numbers)
- Context (where user is, what state system is in)
- Observable outcome (what happens as a result)
- Variation (different scenarios, edge cases)

## Actual Behavior

All examples follow the same generic template with no variation or detail:

```
Examples:
  [0] User track played and is logged in
  [1] User track played and is logged in         # Duplicate
  [2] User track paused and is logged in
  [3] User track stopped and is logged in
  [4] User track ended and is logged in
  [5] User track paused and is logged in         # Duplicate
  [6] User volume changed and is logged in
  [7] User playlist created and is logged in
  [8] User playlist updated and is logged in
  [9] User playlist deleted and is logged in
  [10] User track added to playlist and is logged in
  [11] User track removed from playlist and is logged in
  [12] User shuffle toggled and is logged in
  [13] User track seeked and is logged in
  [14] User audio files loaded and is logged in
  [15] User playlist tracks reordered and is logged in
```

**Problems**:
1. No concrete data (no track names, times, values)
2. No context (no preconditions, no current state)
3. No outcomes (no expected results)
4. Repetitive "and is logged in" suffix (meaningless for music player)
5. Duplicates (same event → duplicate examples)
6. Not actionable for writing scenarios

## Impact

### High Priority Issues:

1. **Defeats Purpose of Example Mapping**: Examples should clarify business rules, these don't
2. **Manual Work Required**: Users must completely rewrite all examples
3. **Wastes Time**: Auto-generation provides zero value
4. **Poor UX**: Looks like placeholder/unfinished feature
5. **Scenario Generation**: Leads to equally generic scenarios in feature files
6. **No Edge Cases**: Doesn't help discover edge cases or variations

### Affects:

- Example Mapping quality
- Scenario generation (garbage in, garbage out)
- Feature file quality
- Discovery effectiveness
- User adoption

## Real-World Evidence

From UI-005 Event Storm transformation:

**Input Domain Events** (14 unique events):
```
TrackPlayed, TrackPaused, TrackStopped, TrackEnded, TrackSeeked, VolumeChanged,
PlaylistCreated, PlaylistUpdated, PlaylistDeleted, TrackAddedToPlaylist,
TrackRemovedFromPlaylist, PlaylistTracksReordered, ShuffleToggled, AudioFilesLoaded
```

**Generated Examples** (16 total, including duplicates):
```
All follow pattern: "User [event name lowercased] and is logged in"
```

**In Generated Feature File**:
```gherkin
# EXAMPLES:
#   1. User track played and is logged in
#   2. User track played and is logged in   # Duplicate!
#   3. User track paused and is logged in
#   4. User track stopped and is logged in
#   ...
```

Zero concrete detail that would help write meaningful scenarios.

## Root Cause Analysis

**Hypothesis**: Template string uses only event name without any intelligence or variation.

**Likely implementation**:
```typescript
// Current behavior (guessed)
function transformDomainEventToExample(event: DomainEvent): Example {
  const eventNameLower = event.text
    .replace(/([A-Z])/g, ' $1')  // TrackPlayed → Track Played
    .toLowerCase()                // → track played
    .trim();

  return {
    text: `User ${eventNameLower} and is logged in`,
    sourceEvent: event.id
  };
}
```

**Problems**:
1. No context generation
2. No data synthesis
3. No variation between similar events
4. Hardcoded "and is logged in" (irrelevant for many domains)
5. No use of commands or policies to infer behavior

## Suggested Fix

### Approach 1: Don't Auto-Generate Examples (Recommended)

**Rationale**: Good examples require domain knowledge and human creativity. Auto-generation from events alone cannot produce quality examples.

**Implementation**:
```typescript
function transformEventStormToExampleMapping(workUnit: WorkUnit) {
  // Transform policies → rules (already works)
  const rules = transformPoliciesToRules(workUnit.eventStorm);

  // Transform hotspots → questions (already works, but needs BUG-088 fix)
  const questions = transformHotspotsToQuestions(workUnit.eventStorm);

  // DO NOT auto-generate examples - leave empty for human to fill
  const examples: Example[] = [];

  // Add comment to guide user
  const guidance = `
    # GUIDANCE: Add concrete examples to illustrate business rules
    # Good examples have:
    #   - Specific data (names, numbers, values)
    #   - Context (preconditions, current state)
    #   - Observable outcomes (what happens)
    #
    # Commands to add examples:
    #   fspec add-example ${workUnit.id} "User plays 'Song Name', track starts from 0:00"
    #   fspec add-example ${workUnit.id} "User pauses at 1:23, position is preserved"
  `;

  return { rules, questions, examples, guidance };
}
```

**Result**: Empty examples list with guidance comment.

### Approach 2: Event + Command Context

Use both events AND commands to generate more meaningful examples:

```typescript
function generateSmartExamples(eventStorm: EventStormArtifacts): Example[] {
  const examples: Example[] = [];

  // Map events to triggering commands
  const eventCommandPairs = mapEventsToCommands(eventStorm);

  for (const pair of eventCommandPairs) {
    const command = pair.command;  // e.g., "PlayTrack"
    const event = pair.event;      // e.g., "TrackPlayed"

    // Generate example with context
    examples.push({
      text: `User executes ${command}, resulting in ${event}`,
      context: { command: command.id, event: event.id }
    });
  }

  return examples;
}
```

**Result**: `"User executes PlayTrack, resulting in TrackPlayed"` (better but still generic)

### Approach 3: Policy-Driven Examples

Use policies to generate specific examples:

```typescript
function generatePolicyExamples(policies: Policy[]): Example[] {
  return policies.map(policy => ({
    text: `When ${policy.when}, system ${policy.then}`,
    source: `policy-${policy.id}`
  }));
}
```

**Result**:
```
[0] When TrackEnded, system executes NextTrack
[1] When ShuffleToggled, system generates new shuffle order
```

Better, but still not concrete enough for real Example Mapping.

### Approach 4: Hybrid - Minimal Placeholders

Generate minimal placeholder examples with clear guidance:

```typescript
function generatePlaceholderExamples(events: DomainEvent[]): Example[] {
  return events.map(event => ({
    text: `[TODO: Add concrete example for ${event.text}]`,
    placeholder: true,
    guidance: `Replace with specific scenario: data, context, outcome`,
    sourceEvent: event.id
  }));
}
```

**Result**: Clearly marked placeholders that users know to replace.

## Testing Strategy

Since the recommended fix is to NOT auto-generate examples, testing would verify:

### Unit Tests:

```typescript
describe('generate-example-mapping-from-event-storm', () => {
  it('should NOT auto-generate examples from domain events', () => {
    const workUnit = createWorkUnitWithEvents(['EventA', 'EventB']);

    const exampleMapping = transformEventStormToExampleMapping(workUnit);

    expect(exampleMapping.examples).toEqual([]);
    expect(exampleMapping.examples).not.toContain('User event a and is logged in');
  });

  it('should include guidance for adding examples', () => {
    const workUnit = createWorkUnitWithEvents(['EventA']);

    const exampleMapping = transformEventStormToExampleMapping(workUnit);

    expect(exampleMapping.guidance).toContain('fspec add-example');
    expect(exampleMapping.guidance).toContain('concrete examples');
  });

  it('should transform policies to rules successfully', () => {
    const workUnit = createWorkUnitWithPolicy({
      when: 'TrackEnded',
      then: 'NextTrack'
    });

    const exampleMapping = transformEventStormToExampleMapping(workUnit);

    expect(exampleMapping.rules).toContainEqual(
      expect.objectContaining({
        text: expect.stringContaining('next track after track ended')
      })
    );
  });
});
```

### Integration Tests:

```bash
# Transform Event Storm with events but no examples manually added
fspec add-domain-event TEST-001 "EventA"
fspec add-domain-event TEST-001 "EventB"
fspec generate-example-mapping-from-event-storm TEST-001

# Verify no generic examples were created
fspec show-work-unit TEST-001 | grep "Examples:" -A 5
# Should show empty or placeholder guidance, NOT "User event a and is logged in"
```

## Additional Context

- Tested on: macOS (Darwin 24.6.0)
- fspec version: Latest
- Node.js version: v22.20.0
- Work unit: UI-005 (Music Player Event Storm)

## Related Issues

- BUG-087: Duplicate events create duplicate examples
- BUG-088: Malformed questions from hotspots

## Philosophy: Example Mapping Purpose

Example Mapping (from Cucumber community) emphasizes:
> "Examples are **concrete instances** that illustrate business rules in action. They should use **real data** and show **specific scenarios** that help developers understand edge cases and expected behavior."

Auto-generated generic examples violate this principle entirely. Better to have NO examples than misleading placeholders.

## Priority

**Medium-High** - Affects quality of discovery output, but users can work around by adding examples manually. However, auto-generated garbage creates negative impression and wastes time.

## Recommendation

**Remove example auto-generation** from Event Storm transformation. Add clear guidance for users to add examples manually using `fspec add-example` commands. Focus auto-generation on transformations that add real value (policies → rules, which works well).
