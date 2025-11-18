# Bug Report: Bounded Context Map UI Duplicates Each Context Name

## Issue Summary

The bounded context map visualization UI renders each bounded context name **twice** in the display, even though the underlying `foundation.json` data contains only single entries for each bounded context.

## Visual Evidence

Screenshot showing the issue:

```
Bounded Context Map
┌─────────────────────┬───────────────┬────────────────┬─────────────────────┬─────────────────────┬───────────────────┬───────────────────┐
│ Conversation        │ Mind Mapping  │ AI Integration │ Workspace & Storage │ Task Orchestration  │ Tool Integration  │ Media Management  │
│ Management          │ Mind Mapping  │ AI Integration │ Workspace & Storage │ Task Orchestration  │ Tool Integration  │ Media Management  │
│ Conversation        │               │                │                     │                     │                   │                   │
│ Management          │               │                │                     │                     │                   │                   │
└─────────────────────┴───────────────┴────────────────┴─────────────────────┴─────────────────────┴───────────────────┴───────────────────┘
```

Each bounded context name appears **twice**:
- "Conversation Management" appears on 2 lines
- "Mind Mapping" appears on 2 lines
- "AI Integration" appears on 2 lines
- "Workspace & Storage" appears on 2 lines
- "Task Orchestration" appears on 2 lines
- "Tool Integration" appears on 2 lines
- "Media Management" appears on 2 lines

## Expected Behavior

Each bounded context should appear **once** in the visualization:

```
Bounded Context Map
┌─────────────────────┬──────────────┬────────────────┬─────────────────────┬────────────────────┬──────────────────┬──────────────────┐
│ Conversation        │ Mind Mapping │ AI Integration │ Workspace & Storage │ Task Orchestration │ Tool Integration │ Media Management │
│ Management          │              │                │                     │                    │                  │                  │
└─────────────────────┴──────────────┴────────────────┴─────────────────────┴────────────────────┴──────────────────┴──────────────────┘
```

## Actual Behavior

Each bounded context name is duplicated in the display, creating visual clutter and confusion about whether there are actual duplicate bounded contexts in the data.

## Data Verification

The underlying data in `spec/foundation.json` is **correct** and contains no duplicates:

```json
{
  "eventStorm": {
    "level": "big_picture",
    "items": [
      {
        "id": 1,
        "type": "bounded_context",
        "text": "Conversation Management",
        "color": null,
        "deleted": false,
        "createdAt": "2025-11-18T03:47:33.120Z"
      },
      {
        "id": 2,
        "type": "bounded_context",
        "text": "Mind Mapping",
        "color": null,
        "deleted": false,
        "createdAt": "2025-11-18T03:47:39.462Z"
      },
      {
        "id": 3,
        "type": "bounded_context",
        "text": "AI Integration",
        "color": null,
        "deleted": false,
        "createdAt": "2025-11-18T03:47:44.919Z"
      },
      {
        "id": 4,
        "type": "bounded_context",
        "text": "Workspace & Storage",
        "color": null,
        "deleted": false,
        "createdAt": "2025-11-18T03:47:49.928Z"
      },
      {
        "id": 5,
        "type": "bounded_context",
        "text": "Task Orchestration",
        "color": null,
        "deleted": false,
        "createdAt": "2025-11-18T03:47:54.869Z"
      },
      {
        "id": 6,
        "type": "bounded_context",
        "text": "Tool Integration",
        "color": null,
        "deleted": false,
        "createdAt": "2025-11-18T03:48:00.263Z"
      },
      {
        "id": 7,
        "type": "bounded_context",
        "text": "Media Management",
        "color": null,
        "deleted": false,
        "createdAt": "2025-11-18T03:48:06.744Z"
      }
    ],
    "nextItemId": 120
  }
}
```

**Verification queries show no duplicates:**

```bash
# Check for duplicate bounded contexts
$ fspec show-foundation-event-storm | jq '[.[] | select(.type == "bounded_context")] | group_by(.text) | map(select(length > 1))'
[]

# List all bounded context IDs and names
$ fspec show-foundation-event-storm | jq -r '.[] | select(.type == "bounded_context") | "\(.id): \(.text)"'
1: Conversation Management
2: Mind Mapping
3: AI Integration
4: Workspace & Storage
5: Task Orchestration
6: Tool Integration
7: Media Management
```

## Reproduction Steps

1. Create a new project with Foundation Event Storm
2. Add bounded contexts using `fspec add-foundation-bounded-context "Context Name"`
3. Add aggregates, events, and commands to each bounded context
4. View the bounded context map visualization (command/UI used to generate the screenshot)
5. **Observe**: Each bounded context name appears twice in the visualization

## Impact

- **Medium**: Does not affect data integrity or functionality
- **Visual confusion**: Users may think there are duplicate bounded contexts in the data
- **User experience**: Makes the bounded context map harder to read
- **Trust**: May cause users to question the reliability of the Event Storm feature

## Root Cause Analysis (Hypothesis)

This appears to be a **rendering/display bug** in the UI component that visualizes the bounded context map. Likely causes:

1. **Component rendering issue**: The React/UI component may be rendering each bounded context twice in a loop
2. **Data mapping error**: The visualization code might be processing the bounded context array incorrectly
3. **CSS layout bug**: Text wrapping or layout styles causing names to appear on multiple lines unintentionally
4. **Template duplication**: The display template might have duplicate placeholders for bounded context names

## Suggested Investigation

1. **Find visualization component**: Search for code that renders the bounded context map
   ```bash
   fspec research --tool=ast --operation=find-function --pattern="BoundedContext|EventStorm.*Map|renderBoundedContext"
   ```

2. **Check for rendering loops**: Look for `.map()` or iteration logic that might be executing twice

3. **Review CSS/styling**: Check if text-wrapping or layout styles are causing visual duplication

4. **Test with different data**: Try with 1, 3, 5, or 10 bounded contexts to see if pattern persists

## Suggested Fix

**If it's a rendering loop issue:**
```typescript
// ❌ WRONG - Renders each item twice
boundedContexts.map(bc => (
  <>
    <div>{bc.text}</div>
    <div>{bc.text}</div>  // Duplicate!
  </>
))

// ✅ CORRECT - Renders each item once
boundedContexts.map(bc => (
  <div>{bc.text}</div>
))
```

**If it's a CSS issue:**
```css
/* Check for duplicate pseudo-elements or content generation */
.bounded-context::before {
  content: attr(data-name);  /* Could cause duplication */
}
```

## Testing Requirements

After fix, verify:

1. **Single rendering**: Each bounded context name appears exactly once
2. **All contexts visible**: All 7 bounded contexts are displayed
3. **No data changes**: `foundation.json` remains unchanged
4. **Different counts**: Test with 1, 3, 5, 10, 15 bounded contexts
5. **Long names**: Test with very long bounded context names (50+ characters)
6. **Special characters**: Test with names containing special characters, emojis, unicode

## Environment

- **Project**: mindstrike
- **fspec version**: 0.9.0 (from --sync-version)
- **Platform**: macOS (Darwin 24.6.0)
- **Bounded contexts in test**: 7
- **Data integrity**: ✅ Verified correct (no duplicates in JSON)
- **Visualization**: ❌ Shows duplicates

## Additional Context

This bug was discovered during Foundation Event Storm for the MindStrike project after:
- Running `fspec add-foundation-bounded-context` for 7 bounded contexts
- Adding 28 aggregates, 45 domain events, and 39 commands
- Viewing the bounded context map visualization

The bug is purely visual/rendering - the underlying Event Storm data is completely correct and contains no duplicates.

## Reporter

Discovered by Claude Code AI agent while conducting Foundation Event Storm for the mindstrike project.

Date: 2025-11-18
