/**
 * Scenario similarity detection utilities
 *
 * Provides semantic analysis to detect when scenarios are refactors of existing ones
 * versus genuinely new scenarios.
 */

export interface ScenarioMatch {
  feature: string;
  scenario: string;
  similarityScore: number;
  featurePath: string;
}

export interface Scenario {
  name: string;
  steps: string[];
}

export interface FeatureFile {
  path: string;
  name: string;
  scenarios: Scenario[];
}

/**
 * Calculate similarity between two strings using Levenshtein distance
 */
function levenshteinDistance(str1: string, str2: string): number {
  const len1 = str1.length;
  const len2 = str2.length;
  const matrix: number[][] = [];

  // Initialize matrix
  for (let i = 0; i <= len1; i++) {
    matrix[i] = [i];
  }
  for (let j = 0; j <= len2; j++) {
    matrix[0][j] = j;
  }

  // Fill matrix
  for (let i = 1; i <= len1; i++) {
    for (let j = 1; j <= len2; j++) {
      const cost = str1[i - 1] === str2[j - 1] ? 0 : 1;
      matrix[i][j] = Math.min(
        matrix[i - 1][j] + 1, // deletion
        matrix[i][j - 1] + 1, // insertion
        matrix[i - 1][j - 1] + cost // substitution
      );
    }
  }

  return matrix[len1][len2];
}

/**
 * Calculate similarity score between two strings (0-1 range)
 */
function calculateStringSimilarity(str1: string, str2: string): number {
  const normalized1 = str1.toLowerCase().trim();
  const normalized2 = str2.toLowerCase().trim();

  if (normalized1 === normalized2) {
    return 1.0;
  }

  const maxLen = Math.max(normalized1.length, normalized2.length);

  // BUG-1 FIX: Handle division by zero for empty strings
  if (maxLen === 0) {
    return 1.0; // Both empty = identical
  }

  const distance = levenshteinDistance(normalized1, normalized2);
  return 1 - distance / maxLen;
}

/**
 * Calculate semantic similarity between two scenarios
 */
export function calculateScenarioSimilarity(
  scenario1: Scenario,
  scenario2: Scenario
): number {
  // Weight title similarity heavily (70%)
  const titleSimilarity = calculateStringSimilarity(scenario1.name, scenario2.name);

  // BUG-7 FIX: Apply short string bias (< 20 chars uses stricter threshold)
  const title1Length = scenario1.name.trim().length;
  const title2Length = scenario2.name.trim().length;
  const isShortString = title1Length < 20 || title2Length < 20;

  // BUG-2 FIX: Strip Gherkin keywords before comparison
  const cleanSteps1 = scenario1.steps.map((s) =>
    s.replace(/^(Given|When|Then|And|But)\s+/i, '').toLowerCase()
  );
  const cleanSteps2 = scenario2.steps.map((s) =>
    s.replace(/^(Given|When|Then|And|But)\s+/i, '').toLowerCase()
  );

  // BUG-6 FIX: Support partial step matching (compare steps individually)
  let stepsSimilarity: number;
  if (cleanSteps1.length === 0 && cleanSteps2.length === 0) {
    stepsSimilarity = 1.0;
  } else if (cleanSteps1.length === 0 || cleanSteps2.length === 0) {
    stepsSimilarity = 0.0;
  } else {
    // Calculate individual step similarities and average
    const maxSteps = Math.max(cleanSteps1.length, cleanSteps2.length);
    let totalSimilarity = 0;

    for (let i = 0; i < maxSteps; i++) {
      const step1 = cleanSteps1[i] || '';
      const step2 = cleanSteps2[i] || '';
      totalSimilarity += calculateStringSimilarity(step1, step2);
    }

    stepsSimilarity = totalSimilarity / maxSteps;
  }

  let finalScore = titleSimilarity * 0.7 + stepsSimilarity * 0.3;

  // Apply penalty for short strings to prevent false positives
  if (isShortString && finalScore > 0.7 && finalScore < 0.85) {
    // Reduce score for short strings that are similar but not identical
    finalScore = finalScore * 0.85;
  }

  return finalScore;
}

/**
 * Find matching scenarios across existing feature files
 */
export function findMatchingScenarios(
  targetScenario: Scenario,
  existingFeatures: FeatureFile[],
  threshold = 0.7
): ScenarioMatch[] {
  const matches: ScenarioMatch[] = [];

  for (const feature of existingFeatures) {
    for (const scenario of feature.scenarios) {
      const similarity = calculateScenarioSimilarity(targetScenario, scenario);

      if (similarity >= threshold) {
        matches.push({
          feature: feature.name,
          scenario: scenario.name,
          similarityScore: similarity,
          featurePath: feature.path
        });
      }
    }
  }

  // Sort by similarity score (highest first)
  return matches.sort((a, b) => b.similarityScore - a.similarityScore);
}

/**
 * Extract keywords from scenario title and steps for semantic analysis
 */
export function extractKeywords(scenario: Scenario): string[] {
  const text = `${scenario.name} ${scenario.steps.join(' ')}`.toLowerCase();

  // Common words to exclude
  const stopWords = new Set([
    'a', 'an', 'the', 'and', 'or', 'but', 'in', 'on', 'at', 'to', 'for',
    'of', 'with', 'by', 'from', 'as', 'is', 'are', 'was', 'were', 'be',
    'been', 'being', 'have', 'has', 'had', 'do', 'does', 'did', 'will',
    'would', 'should', 'could', 'may', 'might', 'must', 'can', 'given',
    'when', 'then', 'and', 'but', 'i', 'that', 'it', 'this'
  ]);

  // BUG-5 FIX: Extract words including numbers (OAuth2, SHA256, Base64, etc.)
  const words = text.match(/\b[a-z0-9]+\b/gi) || [];

  // Filter out stop words and short words
  return words.filter((word) => word.length > 2 && !stopWords.has(word.toLowerCase()));
}

/**
 * Check if scenario appears to be a refactor based on keywords and context
 */
export function isLikelyRefactor(
  targetScenario: Scenario,
  match: ScenarioMatch,
  userIntent?: string
): boolean {
  // High similarity strongly indicates refactor
  if (match.similarityScore > 0.85) {
    return true;
  }

  // Check if user intent suggests refactoring (contains words like "fix", "update", "refactor")
  if (userIntent) {
    const refactorIndicators = /\b(fix|update|refactor|improve|enhance|change|modify)\b/i;
    if (refactorIndicators.test(userIntent)) {
      return match.similarityScore > 0.65;
    }
  }

  // Check keyword overlap
  const targetKeywords = new Set(extractKeywords(targetScenario));
  const matchKeywords = extractKeywords({
    name: match.scenario,
    steps: [] // We don't have steps from the match object
  });

  const overlap = matchKeywords.filter((kw) => targetKeywords.has(kw)).length;
  const overlapRatio = overlap / Math.max(targetKeywords.size, matchKeywords.length);

  return match.similarityScore > 0.7 && overlapRatio > 0.6;
}
