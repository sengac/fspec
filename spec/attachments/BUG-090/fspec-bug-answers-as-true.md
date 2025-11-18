# Bug: Answered Questions Display as "true" Instead of Actual Answer Text

## Issue Summary

When answering Example Mapping questions using `fspec answer-question`, the provided answer text is not preserved. Instead, answered questions display the boolean value `true` in feature file comments and work unit output, losing all the valuable context provided in the answer.

## Steps to Reproduce

1. Create a work unit with a question:
   ```bash
   fspec create-story UI "Test Story"
   fspec update-work-unit-status UI-001 specifying
   fspec add-question UI-001 "@human: When should data be cached?"
   ```

2. Answer the question with detailed text:
   ```bash
   fspec answer-question UI-001 0 --answer "Data should be cached immediately on first access using file-based persistence with 24-hour TTL"
   ```

3. Generate scenarios (which creates feature file):
   ```bash
   fspec generate-scenarios UI-001
   ```

4. View the feature file:
   ```bash
   cat spec/features/test-story.feature
   ```

## Expected Behavior

The actual answer text should be preserved and displayed:

**In feature file comments**:
```gherkin
# QUESTIONS (ANSWERED):
#   Q: When should data be cached?
#   A: Data should be cached immediately on first access using file-based persistence with 24-hour TTL
```

**In work unit output**:
```
Questions:
  [0] @human: When should data be cached?
      Answer: Data should be cached immediately on first access using file-based persistence with 24-hour TTL
```

## Actual Behavior

The answer is replaced with boolean `true`:

**In feature file comments**:
```gherkin
# QUESTIONS (ANSWERED):
#   Q: What should when should playlists be saved? on every change or debounced? be?
#   A: true
#
#   Q: What should how long should album art and metadata be cached? be?
#   A: true
#
#   Q: What should should drag-and-drop support multi-select? how to handle edge cases? be?
#   A: true
#
#   Q: What should with large libraries (10k+ tracks), will search be fast enough? be?
#   A: true
```

All answers are literally the boolean `true`, regardless of what text was provided.

## Impact

### Critical Issues:

1. **Loss of Information**: All answer context is lost
2. **Documentation Failure**: Feature files don't capture decisions made during discovery
3. **Knowledge Gap**: Future developers can't understand why decisions were made
4. **Example Mapping Broken**: Answers inform rules and examples, but they're lost
5. **Audit Trail**: Can't review what was decided during discovery sessions

### Affects:

- Feature file quality (comments are documentation)
- Knowledge retention
- Discovery session value
- Team communication
- Decision tracking

## Real-World Evidence

From UI-005 Event Storm session:

**Questions Answered**:
```bash
fspec answer-question UI-005 0 \
  --answer "Playlists are saved immediately on every change using Zustand persist middleware for real-time persistence"

fspec answer-question UI-005 1 \
  --answer "Album art and metadata are cached server-side using music-metadata-cache service with file-based persistence"

fspec answer-question UI-005 2 \
  --answer "Yes, drag-and-drop supports multi-select with shift/ctrl/meta key modifiers for batch operations"

fspec answer-question UI-005 3 \
  --answer "Search uses client-side filtering with react-virtualized for performance with large libraries, filtering on title, artist, album, and genre"
```

**Expected in Feature File**:
```gherkin
# QUESTIONS (ANSWERED):
#   Q: When should playlists be saved?
#   A: Playlists are saved immediately on every change using Zustand persist middleware for real-time persistence
#
#   Q: How long should album art and metadata be cached?
#   A: Album art and metadata are cached server-side using music-metadata-cache service with file-based persistence
```

**Actual in Feature File**:
```gherkin
# QUESTIONS (ANSWERED):
#   Q: What should when should playlists be saved? on every change or debounced? be?
#   A: true
#
#   Q: What should how long should album art and metadata be cached? be?
#   A: true
```

All detailed answers replaced with `true`.

## Root Cause Analysis

**Hypothesis**: Answer data is stored incorrectly or retrieved incorrectly during feature file generation.

### Possibility 1: Storage Issue

Answer is stored as boolean flag instead of text:

```typescript
// Current behavior (guessed)
interface Question {
  text: string;
  answered: boolean;  // ❌ Should store answer text, not just flag
}

function answerQuestion(workUnitId: string, index: number, answer: string) {
  const questions = getQuestions(workUnitId);
  questions[index].answered = true;  // ❌ Loses answer text!
  saveWorkUnit(workUnitId);
}
```

**Should be**:
```typescript
interface Question {
  text: string;
  answered: boolean;
  answer?: string;  // ✅ Store actual answer text
}

function answerQuestion(workUnitId: string, index: number, answer: string) {
  const questions = getQuestions(workUnitId);
  questions[index].answered = true;
  questions[index].answer = answer;  // ✅ Preserve answer text
  saveWorkUnit(workUnitId);
}
```

### Possibility 2: Retrieval Issue

Answer is stored correctly but retrieved incorrectly:

```typescript
// Current behavior (guessed)
function formatAnsweredQuestion(question: Question): string {
  return `Q: ${question.text}\nA: ${question.answered}`;  // ❌ Shows boolean!
}
```

**Should be**:
```typescript
function formatAnsweredQuestion(question: Question): string {
  const answerText = question.answer || '(No answer provided)';
  return `Q: ${question.text}\nA: ${answerText}`;  // ✅ Shows actual answer
}
```

### Possibility 3: Type Coercion Issue

Answer stored as string but coerced to boolean:

```typescript
// Somewhere in the code
const answerValue = Boolean(question.answer);  // ❌ Converts to true/false
```

## Suggested Fix

### Approach 1: Store Answer Text (Recommended)

Update the Question interface and storage logic:

```typescript
interface Question {
  text: string;
  answered: boolean;
  answer?: string;
  answeredAt?: string;  // ISO timestamp
}

function answerQuestion(
  workUnitId: string,
  index: number,
  answer: string,
  options?: { addTo?: 'rule' | 'assumption' }
): void {
  const workUnit = getWorkUnit(workUnitId);
  const question = workUnit.exampleMapping.questions[index];

  if (!question) {
    throw new Error(`Question ${index} not found`);
  }

  // Store the answer
  question.answered = true;
  question.answer = answer.trim();
  question.answeredAt = new Date().toISOString();

  // Optionally add to rules or assumptions
  if (options?.addTo === 'rule') {
    workUnit.exampleMapping.rules.push({
      text: answer,
      source: `question-${index}`
    });
  } else if (options?.addTo === 'assumption') {
    workUnit.exampleMapping.assumptions = workUnit.exampleMapping.assumptions || [];
    workUnit.exampleMapping.assumptions.push({
      text: answer,
      source: `question-${index}`
    });
  }

  saveWorkUnit(workUnit);

  console.log(`✓ Answered question: "${question.text}"`);
  console.log(`  Answer: "${answer}"`);
}
```

### Approach 2: Fix Feature File Generation

Ensure feature file comments use answer text:

```typescript
function generateFeatureFileComments(exampleMapping: ExampleMapping): string {
  let comments = '# ========================================\n';
  comments += '# EXAMPLE MAPPING CONTEXT\n';
  comments += '# ========================================\n\n';

  // ... rules section ...

  // Questions section
  if (exampleMapping.questions.length > 0) {
    const answeredQuestions = exampleMapping.questions.filter(q => q.answered);
    const unansweredQuestions = exampleMapping.questions.filter(q => !q.answered);

    if (answeredQuestions.length > 0) {
      comments += '# QUESTIONS (ANSWERED):\n';
      for (const q of answeredQuestions) {
        comments += `#   Q: ${q.text}\n`;
        comments += `#   A: ${q.answer || '(No answer recorded)'}\n#\n`;
      }
    }

    if (unansweredQuestions.length > 0) {
      comments += '# QUESTIONS (UNANSWERED):\n';
      for (const q of unansweredQuestions) {
        comments += `#   Q: ${q.text}\n#\n`;
      }
    }
  }

  return comments;
}
```

## Data Migration

If existing work units have `answered: true` but no `answer` field:

```typescript
function migrateQuestions(workUnit: WorkUnit): void {
  if (!workUnit.exampleMapping?.questions) return;

  for (const question of workUnit.exampleMapping.questions) {
    if (question.answered && !question.answer) {
      // Add placeholder indicating migration needed
      question.answer = '[Answer lost during migration - please re-answer]';
    }
  }
}
```

## Testing Strategy

### Unit Tests:

```typescript
describe('answer-question', () => {
  it('should store answer text, not just boolean flag', () => {
    const workUnit = createWorkUnitWithQuestion('@human: When?');

    answerQuestion(workUnit.id, 0, 'Immediately on change');

    const question = getQuestion(workUnit.id, 0);
    expect(question.answered).toBe(true);
    expect(question.answer).toBe('Immediately on change');
    expect(question.answer).not.toBe('true');  // Not boolean!
  });

  it('should preserve answer text in feature file comments', () => {
    const workUnit = createWorkUnitWithQuestion('@human: How?');
    answerQuestion(workUnit.id, 0, 'Using file-based cache');

    const featureFile = generateScenarios(workUnit.id);
    const content = readFile(featureFile);

    expect(content).toContain('Q: @human: How?');
    expect(content).toContain('A: Using file-based cache');
    expect(content).not.toContain('A: true');
  });

  it('should handle multiline answers', () => {
    const longAnswer = `
      Data is cached with the following strategy:
      1. Check cache on first access
      2. Use file-based persistence
      3. TTL of 24 hours
    `.trim();

    answerQuestion('TEST-001', 0, longAnswer);

    const question = getQuestion('TEST-001', 0);
    expect(question.answer).toBe(longAnswer);
  });

  it('should store timestamp when question is answered', () => {
    const before = new Date().toISOString();

    answerQuestion('TEST-001', 0, 'Answer text');

    const after = new Date().toISOString();
    const question = getQuestion('TEST-001', 0);

    expect(question.answeredAt).toBeDefined();
    expect(question.answeredAt).toBeGreaterThanOrEqual(before);
    expect(question.answeredAt).toBeLessThanOrEqual(after);
  });
});
```

### Integration Tests:

```bash
# Add question and answer
fspec create-story TEST "Test"
fspec update-work-unit-status TEST-001 specifying
fspec add-question TEST-001 "@human: When should X happen?"
fspec answer-question TEST-001 0 --answer "Immediately on user action"

# Generate feature file
fspec generate-scenarios TEST-001

# Verify answer is preserved
cat spec/features/test.feature | grep -A 1 "Q: @human:"
# Should show:
#   Q: @human: When should X happen?
#   A: Immediately on user action
# Should NOT show:
#   A: true
```

## Additional Context

- Tested on: macOS (Darwin 24.6.0)
- fspec version: Latest
- Node.js version: v22.20.0
- Work unit: UI-005 (Music Player Event Storm)

## Related Issues

- BUG-088: Malformed questions (affects quality of question text, this affects answers)

## Priority

**High** - Critical information loss. Answers are key decisions made during discovery that must be preserved for future reference. Without answer text, the question/answer cycle provides no value.

## Workaround

Currently, no good workaround. Answer information is lost permanently once `generate-scenarios` is run. Users would need to:
1. Keep manual notes of answers outside fspec
2. Manually edit feature file comments after generation (fragile, error-prone)

Neither is acceptable.
