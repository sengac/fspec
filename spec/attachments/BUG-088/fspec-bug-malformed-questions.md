# Bug: Malformed Question Text When Transforming Hotspots to Example Mapping

## Issue Summary

When transforming Event Storm hotspots to Example Mapping questions via `fspec generate-example-mapping-from-event-storm`, the generated question text is malformed and unreadable. The question mixes the hotspot concern text with template placeholders, resulting in grammatically incorrect questions.

## Steps to Reproduce

1. Create a work unit and add hotspots during Event Storm:
   ```bash
   fspec create-story UI "Test Story"
   fspec update-work-unit-status UI-001 specifying
   fspec discover-event-storm UI-001

   fspec add-hotspot UI-001 "Playlist persistence timing" \
     --concern "When should playlists be saved? On every change or debounced?"

   fspec add-hotspot UI-001 "Track metadata caching" \
     --concern "How long should album art and metadata be cached?"
   ```

2. Transform Event Storm to Example Mapping:
   ```bash
   fspec generate-example-mapping-from-event-storm UI-001
   ```

3. View the generated questions:
   ```bash
   fspec show-work-unit UI-001
   ```

## Expected Behavior

Generated questions should be clear, grammatically correct, and directly based on the hotspot concern:

```
Questions:
  [0] @human: When should playlists be saved? On every change or debounced?
  [1] @human: How long should album art and metadata be cached?
  [2] @human: Should drag-and-drop support multi-select? How to handle edge cases?
  [3] @human: With large libraries (10k+ tracks), will search be fast enough?
```

**Format**: `@human: [concern text]`

The concern text should be used as-is, or minimally reformatted into a proper question.

## Actual Behavior

Generated questions are malformed with redundant text and broken grammar:

```
Questions:
  [0] @human: What should when should playlists be saved? on every change or debounced? be?
  [1] @human: What should how long should album art and metadata be cached? be?
  [2] @human: What should should drag-and-drop support multi-select? how to handle edge cases? be?
  [3] @human: What should with large libraries (10k+ tracks), will search be fast enough? be?
```

**Problems**:
1. Prepends `"What should "` before concern text
2. Appends `" be?"` after concern text
3. Results in double question words: "What should when should..."
4. Makes questions unreadable and confusing

## Impact

### High Priority Issues:

1. **Unreadable Questions**: Users can't understand what's being asked
2. **Poor UX**: Looks like a bug/glitch in the tool
3. **Requires Manual Editing**: Users must fix questions manually (error-prone)
4. **Workflow Interruption**: Breaks the flow from Event Storm → Example Mapping
5. **Trust Issues**: Users lose confidence in auto-generated content

### Affects:

- Example Mapping discovery phase
- Feature file generation (questions appear as comments)
- Documentation quality
- User adoption (poor first impression)

## Real-World Evidence

From UI-005 Event Storm transformation:

**Input Hotspots**:
```json
[
  {
    "id": 36,
    "type": "hotspot",
    "text": "Playlist persistence timing",
    "concern": "When should playlists be saved? On every change or debounced?",
    "deleted": false
  },
  {
    "id": 37,
    "type": "hotspot",
    "text": "Track metadata caching",
    "concern": "How long should album art and metadata be cached?",
    "deleted": false
  }
]
```

**Generated Questions (malformed)**:
```
[0] @human: What should when should playlists be saved? on every change or debounced? be?
[1] @human: What should how long should album art and metadata be cached? be?
```

**In Generated Feature File**:
```gherkin
# QUESTIONS (ANSWERED):
#   Q: What should when should playlists be saved? on every change or debounced? be?
#   A: true
#
#   Q: What should how long should album art and metadata be cached? be?
#   A: true
```

## Root Cause Analysis

**Hypothesis**: Template string wraps concern text without checking if concern is already a question.

**Likely implementation**:
```typescript
// Current behavior (guessed)
function transformHotspotToQuestion(hotspot: Hotspot): Question {
  return {
    text: `@human: What should ${hotspot.concern.toLowerCase()} be?`,
    answered: false
  };
}
```

**Problems with this approach**:
1. Assumes concern is a noun phrase, not a question
2. Forces question structure even when concern is already a question
3. Lowercases concern (loses capitalization of proper nouns)
4. No intelligent text processing or grammar checking

## Suggested Fix

### Approach 1: Use Concern Text As-Is (Recommended)

```typescript
function transformHotspotToQuestion(hotspot: Hotspot): Question {
  let questionText = hotspot.concern.trim();

  // Ensure it ends with a question mark
  if (!questionText.endsWith('?')) {
    questionText += '?';
  }

  return {
    text: `@human: ${questionText}`,
    answered: false
  };
}
```

**Result**:
```
[0] @human: When should playlists be saved? On every change or debounced?
[1] @human: How long should album art and metadata be cached?
```

### Approach 2: Smart Question Detection

```typescript
function transformHotspotToQuestion(hotspot: Hotspot): Question {
  const concern = hotspot.concern.trim();

  // Check if concern is already a question (starts with question word or ends with ?)
  const questionWords = ['what', 'when', 'where', 'why', 'who', 'how', 'should', 'can', 'will', 'is', 'are'];
  const isQuestion =
    concern.endsWith('?') ||
    questionWords.some(word => concern.toLowerCase().startsWith(word));

  let questionText;
  if (isQuestion) {
    // Use as-is, ensure question mark
    questionText = concern.endsWith('?') ? concern : `${concern}?`;
  } else {
    // Convert noun phrase to question
    questionText = `What should be done about ${concern}?`;
  }

  return {
    text: `@human: ${questionText}`,
    answered: false
  };
}
```

**Result**:
```
# Concern was already a question
[0] @human: When should playlists be saved? On every change or debounced?

# Concern was a noun phrase
[1] @human: What should be done about metadata cache invalidation?
```

### Approach 3: Preserve Hotspot Text as Title

```typescript
function transformHotspotToQuestion(hotspot: Hotspot): Question {
  const concern = hotspot.concern.trim();
  const title = hotspot.text; // e.g., "Playlist persistence timing"

  return {
    text: `@human: ${title} - ${concern.endsWith('?') ? concern : concern + '?'}`,
    answered: false
  };
}
```

**Result**:
```
[0] @human: Playlist persistence timing - When should playlists be saved? On every change or debounced?
[1] @human: Track metadata caching - How long should album art and metadata be cached?
```

## Testing Strategy

### Unit Tests:

```typescript
describe('generate-example-mapping-from-event-storm', () => {
  describe('hotspot to question transformation', () => {
    it('should preserve question text when concern is already a question', () => {
      const hotspot = {
        id: 0,
        type: 'hotspot',
        text: 'Test Hotspot',
        concern: 'When should X be saved?',
        deleted: false
      };

      const question = transformHotspotToQuestion(hotspot);

      expect(question.text).toBe('@human: When should X be saved?');
      expect(question.text).not.toContain('What should');
      expect(question.text).not.toContain(' be?');
    });

    it('should add question mark if concern lacks one', () => {
      const hotspot = {
        id: 0,
        type: 'hotspot',
        text: 'Test Hotspot',
        concern: 'How to handle this',
        deleted: false
      };

      const question = transformHotspotToQuestion(hotspot);

      expect(question.text).toBe('@human: How to handle this?');
    });

    it('should not lowercase proper nouns or question words', () => {
      const hotspot = {
        id: 0,
        type: 'hotspot',
        text: 'API Integration',
        concern: 'Should we use REST or GraphQL?',
        deleted: false
      };

      const question = transformHotspotToQuestion(hotspot);

      expect(question.text).toContain('REST'); // Preserved case
      expect(question.text).toContain('GraphQL'); // Preserved case
    });

    it('should handle multiple sentences in concern', () => {
      const hotspot = {
        id: 0,
        type: 'hotspot',
        text: 'Complexity',
        concern: 'Is this too complex? Should we simplify?',
        deleted: false
      };

      const question = transformHotspotToQuestion(hotspot);

      expect(question.text).toBe('@human: Is this too complex? Should we simplify?');
    });
  });
});
```

### Integration Tests:

```bash
# Create hotspot with question-format concern
fspec add-hotspot TEST-001 "Test" --concern "When should X happen?"

# Transform
fspec generate-example-mapping-from-event-storm TEST-001

# Verify question is clean
fspec show-work-unit TEST-001 | grep -A 1 "Questions:"
# Should show: @human: When should X happen?
# Should NOT show: What should when should...
```

## Additional Context

- Tested on: macOS (Darwin 24.6.0)
- fspec version: Latest
- Node.js version: v22.20.0
- Work unit: UI-005 (Music Player Event Storm)
- Observed during: Event Storm → Example Mapping transformation

## Related Features

- Event Storm hotspots
- Example Mapping question cards
- Feature file generation (questions appear as comments)

## Priority

**High** - Affects readability and usability of core discovery workflow. Makes auto-generated content look broken and requires manual cleanup.

## Screenshots/Examples

**Before (Current - Malformed)**:
```
Questions:
  [0] @human: What should when should playlists be saved? on every change or debounced? be?
```

**After (Fixed)**:
```
Questions:
  [0] @human: When should playlists be saved? On every change or debounced?
```
