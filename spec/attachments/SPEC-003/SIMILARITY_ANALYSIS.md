# Deep Analysis: Scenario Similarity Matching Implementation

## Executive Summary

The current implementation uses **Levenshtein distance** with **weighted scoring** (70% title, 30% steps). While functional, there are **7 critical bugs** and **15 improvement opportunities** identified.

---

## 🐛 CRITICAL BUGS

### Bug 1: Division by Zero - Empty Strings
**Location**: `src/utils/scenario-similarity.ts:68-70`

```typescript
const maxLen = Math.max(normalized1.length, normalized2.length);
const distance = levenshteinDistance(normalized1, normalized2);
return 1 - distance / maxLen;  // ❌ Division by zero if both strings empty
```

**Impact**: Crashes when comparing scenarios with empty titles or when all steps are empty.

**Fix**:
```typescript
if (maxLen === 0) {
  return 1.0; // Both empty = identical
}
return 1 - distance / maxLen;
```

---

### Bug 2: Gherkin Keywords Pollute Step Similarity
**Location**: `src/commands/generate-scenarios.ts:224` and `src/utils/scenario-similarity.ts:84-85`

```typescript
// Steps include keywords: "Given user exists", "When login", "Then success"
steps: child.scenario!.steps.map((step) => `${step.keyword}${step.text}`)

// Later joined: "Given user exists When login Then success"
const steps1 = scenario1.steps.join(' ').toLowerCase();
```

**Impact**: Every scenario has "given when then and but" adding noise to similarity scoring. Two scenarios might match high purely because they both have "Given When Then" structure.

**Fix**: Strip Gherkin keywords before comparison:
```typescript
const steps1 = scenario1.steps
  .map(step => step.replace(/^(Given|When|Then|And|But)\s+/i, ''))
  .join(' ')
  .toLowerCase();
```

---

### Bug 3: Keyword Extraction from Match Uses Empty Steps
**Location**: `src/utils/scenario-similarity.ts:165-168`

```typescript
const matchKeywords = extractKeywords({
  name: match.scenario,
  steps: [] // ❌ BUG! We don't have steps from the match object
});
```

**Impact**: `isLikelyRefactor()` only compares title keywords, completely ignoring step keywords. The keyword overlap ratio is meaningless because match scenario has no step keywords.

**Fix**: Pass the full scenario object with steps to `isLikelyRefactor()`, or change ScenarioMatch interface to include steps.

---

### Bug 4: Potential Division by Zero - Empty Keywords
**Location**: `src/utils/scenario-similarity.ts:171`

```typescript
const overlapRatio = overlap / Math.max(targetKeywords.size, matchKeywords.length);
// ❌ Division by zero if both are empty
```

**Impact**: Crashes when scenarios have no extractable keywords (e.g., only stopwords).

**Fix**:
```typescript
const denominator = Math.max(targetKeywords.size, matchKeywords.length);
const overlapRatio = denominator === 0 ? 0 : overlap / denominator;
```

---

### Bug 5: Regex Excludes Numbers and Special Characters
**Location**: `src/utils/scenario-similarity.ts:136`

```typescript
const words = text.match(/\b[a-z]+\b/g) || [];
// ❌ Excludes "OAuth2", "Base64", "SHA256", etc.
```

**Impact**: Important technical terms with numbers are excluded from keyword extraction.

**Fix**:
```typescript
const words = text.match(/\b[a-z0-9]+\b/gi) || [];
```

---

### Bug 6: Redundant Lowercasing
**Location**: `src/utils/scenario-similarity.ts:84-85, 61`

```typescript
// Line 84-85:
const steps1 = scenario1.steps.join(' ').toLowerCase();
const steps2 = scenario2.steps.join(' ').toLowerCase();

// Line 61 (called from calculateStringSimilarity):
const normalized1 = str1.toLowerCase().trim();  // ❌ Lowercasing again!
```

**Impact**: Unnecessary computation, minor performance hit.

**Fix**: Remove toLowerCase() from line 84-85 since calculateStringSimilarity does it.

---

### Bug 7: User Intent Parameter Never Used
**Location**: Integration between `generate-scenarios.ts` and `isLikelyRefactor()`

**Issue**: `isLikelyRefactor()` accepts `userIntent` parameter (line 148) but it's **never called** in the main workflow. The work unit title/description could provide valuable context but is ignored.

**Impact**: Missing opportunity to improve detection accuracy based on user's stated goal.

**Fix**: Pass work unit title/description as userIntent when calling matching functions.

---

## 🔍 LOGICAL ISSUES

### Issue 1: Step Order Sensitivity
**Problem**: Steps joined as single string means order matters.

```typescript
// These are semantically equivalent but score differently:
Scenario A: ["Given user exists", "Given database connected", "When login"]
Scenario B: ["Given database connected", "Given user exists", "When login"]
```

**Impact**: False negatives for scenarios that are same but have reordered steps.

---

### Issue 2: No Partial Step Matching
**Problem**: Either all steps match or none do.

```typescript
// 75% of steps identical, but no credit for partial match:
Scenario A: ["Given X", "When Y", "Then Z", "And W"]
Scenario B: ["Given X", "When Y", "Then Z", "And Q"]  // 3/4 match
```

**Impact**: Scenarios with minor step differences score poorly despite being mostly identical.

---

### Issue 3: Short String Bias
**Problem**: Short titles can have high similarity by coincidence.

```typescript
"User login"     vs "User logout"    = 83.3% similar
"Invalid input"  vs "Invalid email"  = 76.9% similar
```

**Impact**: False positives for short, generic scenario titles.

---

### Issue 4: No Context from Feature Name
**Problem**: Same scenario title in different features treated equally.

```typescript
// Both are "Create user" but completely different contexts:
Feature: Admin Panel → Scenario: Create user (as admin)
Feature: Registration → Scenario: Create user (self-registration)
```

**Impact**: False positives when scenario names are generic.

---

### Issue 5: Given/When/Then Structure Ignored
**Problem**: A scenario that mixes up Given/When/Then would still match.

```typescript
// These are structurally wrong but would match:
Scenario A: "Given X, When Y, Then Z"
Scenario B: "When Y, Then Z, Given X"  // Wrong order!
```

**Impact**: Could match structurally invalid scenarios.

---

## 💡 IMPROVEMENT OPPORTUNITIES

### 1. **Configurable Weights**
Current 70/30 split is arbitrary. Allow configuration:
```typescript
interface SimilarityConfig {
  titleWeight: number;  // Default 0.7
  stepsWeight: number;  // Default 0.3
  threshold: number;    // Default 0.7
}
```

---

### 2. **Normalize Whitespace and Punctuation**
```typescript
function normalize(text: string): string {
  return text
    .toLowerCase()
    .replace(/\s+/g, ' ')  // Multiple spaces → single space
    .replace(/[^\w\s]/g, '')  // Remove punctuation
    .trim();
}
```

---

### 3. **Stemming/Lemmatization**
Use library like `natural` or `compromise`:
```typescript
import { PorterStemmer } from 'natural';

// "validating users" → "valid user"
// "authenticated" → "authent"
```

---

### 4. **Step-by-Step Comparison**
Compare individual steps with alignment:
```typescript
function compareSteps(steps1: string[], steps2: string[]): number {
  const given1 = steps1.filter(s => s.startsWith('Given'));
  const given2 = steps2.filter(s => s.startsWith('Given'));

  const givenSimilarity = compareArrays(given1, given2);
  const whenSimilarity = compareArrays(when1, when2);
  const thenSimilarity = compareArrays(then1, then2);

  return (givenSimilarity + whenSimilarity + thenSimilarity) / 3;
}
```

---

### 5. **Jaccard Similarity for Keywords**
Instead of Levenshtein, use set-based similarity:
```typescript
function jaccardSimilarity(set1: Set<string>, set2: Set<string>): number {
  const intersection = new Set([...set1].filter(x => set2.has(x)));
  const union = new Set([...set1, ...set2]);
  return intersection.size / union.size;
}
```

---

### 6. **TF-IDF for Keyword Importance**
Weight keywords by importance across all scenarios:
```typescript
// "login" appears in 50% of scenarios → less important
// "OAuth2" appears in 2% of scenarios → more important
```

---

### 7. **Feature Context Bonus**
Boost similarity if scenarios are in same feature:
```typescript
if (scenario1.featureName === scenario2.featureName) {
  similarity *= 1.2;  // 20% bonus
}
```

---

### 8. **Tag Overlap Consideration**
```typescript
const tagOverlap = intersection(scenario1.tags, scenario2.tags);
if (tagOverlap.length > 0) {
  similarity *= 1.1;  // 10% bonus
}
```

---

### 9. **Confidence Intervals**
```typescript
interface SimilarityResult {
  score: number;
  confidence: 'low' | 'medium' | 'high';
  reasons: string[];  // Why it matched
}
```

---

### 10. **Performance Optimization**
For large codebases, use two-stage matching:
```typescript
// Stage 1: Fast keyword filtering
const candidates = filterByKeywordOverlap(target, allScenarios);

// Stage 2: Expensive Levenshtein only on candidates
const matches = candidates.map(c => calculateFullSimilarity(target, c));
```

---

### 11. **Semantic Embeddings** (Advanced)
Use ML-based semantic similarity:
```typescript
import { SentenceTransformer } from '@xenova/transformers';

const embedding1 = await model.encode(scenario1.text);
const embedding2 = await model.encode(scenario2.text);
const similarity = cosineSimilarity(embedding1, embedding2);
```

---

### 12. **Background Step Inclusion**
```typescript
// Currently ignores Background steps - should include them
const allSteps = [...backgroundSteps, ...scenarioSteps];
```

---

### 13. **Synonym Handling**
```typescript
const synonyms = {
  'login': ['authenticate', 'signin', 'logon'],
  'verify': ['validate', 'check', 'confirm'],
  'user': ['account', 'member', 'customer']
};
```

---

### 14. **Length Penalty for Short Strings**
```typescript
if (title.length < 20) {
  // Apply stricter threshold
  threshold = 0.85;
}
```

---

### 15. **Multiple Match Reporting**
```typescript
// Currently only shows best match
// Should show all matches above threshold
return {
  bestMatch: matches[0],
  alternativeMatches: matches.slice(1, 5),  // Top 5
  totalMatches: matches.length
};
```

---

## 📊 RECOMMENDED PRIORITY

### High Priority (Fix Immediately)
1. ✅ Bug 1: Division by zero (empty strings)
2. ✅ Bug 2: Strip Gherkin keywords from steps
3. ✅ Bug 3: Fix keyword extraction for matches
4. ✅ Bug 4: Division by zero (empty keywords)

### Medium Priority (Next Sprint)
5. ✅ Bug 5: Include numbers in regex
6. ✅ Issue 2: Partial step matching
7. ✅ Improvement 2: Normalize whitespace/punctuation
8. ✅ Improvement 4: Step-by-step comparison
9. ✅ Improvement 7: Feature context bonus

### Low Priority (Future Enhancement)
10. ✅ Improvement 3: Stemming/lemmatization
11. ✅ Improvement 11: Semantic embeddings
12. ✅ Improvement 13: Synonym handling

---

## 🧪 TEST COVERAGE GAPS

Current tests don't cover:
1. ❌ Empty scenario titles
2. ❌ Empty scenario steps
3. ❌ Scenarios with only stopwords
4. ❌ Very short titles (< 10 chars)
5. ❌ Scenarios with special characters
6. ❌ Scenarios with numbers (OAuth2, SHA256, etc.)
7. ❌ Reordered steps (same steps, different order)
8. ❌ Partial step matches (3/4 steps same)
9. ❌ Feature context (same title, different features)
10. ❌ Performance with 100+ scenarios

---

## 🎯 RECOMMENDED FIXES

I recommend fixing the **4 high-priority bugs** immediately, then implementing **step-by-step comparison** and **normalization** in the next iteration.

Would you like me to implement these fixes?
