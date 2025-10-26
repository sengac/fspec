/**
 * Advanced similarity algorithms for scenario matching
 *
 * This file contains implementations of:
 * - Jaro-Winkler Distance (prefix matching)
 * - Token Set Ratio (word reordering)
 * - Trigram Similarity (typo tolerance)
 * - Jaccard Similarity (keyword overlap)
 * - Gherkin Structural Analysis (Given/When/Then awareness)
 * - Hybrid Algorithm (weighted combination)
 */

export interface Scenario {
  name: string;
  steps: string[];
}

export interface SimilarityConfig {
  // Hybrid algorithm weights (must sum to 1.0)
  jaroWinklerWeight: number;
  tokenSetWeight: number;
  gherkinStructuralWeight: number;
  trigramWeight: number;
  jaccardWeight: number;
}

/**
 * Default similarity configuration for normal-length strings (>= 20 chars)
 * Weights emphasize character similarity and balanced multi-algorithm approach
 */
export const DEFAULT_SIMILARITY_CONFIG: SimilarityConfig = {
  jaroWinklerWeight: 0.3,
  tokenSetWeight: 0.25,
  gherkinStructuralWeight: 0.2,
  trigramWeight: 0.15,
  jaccardWeight: 0.1,
};

/**
 * Short string configuration for strings < 20 chars
 * Emphasizes word-level algorithms over character-level to prevent false positives
 */
export const SHORT_STRING_CONFIG: SimilarityConfig = {
  tokenSetWeight: 0.35, // ⬆️ Word-level changes critical
  jaccardWeight: 0.2, // ⬆️ Keyword overlap important
  gherkinStructuralWeight: 0.2,
  jaroWinklerWeight: 0.15, // ⬇️ Character similarity misleading
  trigramWeight: 0.1, // ⬇️ N-grams less meaningful
};

/**
 * Calculate Jaro-Winkler similarity between two strings
 * Better for short strings and prefix matching
 *
 * @param str1 - First string
 * @param str2 - Second string
 * @returns Similarity score (0-1)
 */
export function jaroWinklerSimilarity(str1: string, str2: string): number {
  const s1 = str1.toLowerCase();
  const s2 = str2.toLowerCase();

  if (s1 === s2) {
    return 1.0;
  }

  const len1 = s1.length;
  const len2 = s2.length;

  if (len1 === 0 && len2 === 0) {
    return 1.0;
  }
  if (len1 === 0 || len2 === 0) {
    return 0.0;
  }

  // Maximum allowed distance for matching characters
  const matchDistance = Math.floor(Math.max(len1, len2) / 2) - 1;

  const s1Matches = new Array(len1).fill(false);
  const s2Matches = new Array(len2).fill(false);

  let matches = 0;
  let transpositions = 0;

  // Find matches
  for (let i = 0; i < len1; i++) {
    const start = Math.max(0, i - matchDistance);
    const end = Math.min(i + matchDistance + 1, len2);

    for (let j = start; j < end; j++) {
      if (s2Matches[j] || s1[i] !== s2[j]) {
        continue;
      }
      s1Matches[i] = true;
      s2Matches[j] = true;
      matches++;
      break;
    }
  }

  if (matches === 0) {
    return 0.0;
  }

  // Find transpositions
  let k = 0;
  for (let i = 0; i < len1; i++) {
    if (!s1Matches[i]) {
      continue;
    }
    while (!s2Matches[k]) {
      k++;
    }
    if (s1[i] !== s2[k]) {
      transpositions++;
    }
    k++;
  }

  // Calculate Jaro similarity
  const jaro =
    (matches / len1 +
      matches / len2 +
      (matches - transpositions / 2) / matches) /
    3;

  // Calculate common prefix length (up to 4 characters)
  let prefix = 0;
  for (let i = 0; i < Math.min(len1, len2, 4); i++) {
    if (s1[i] === s2[i]) {
      prefix++;
    } else {
      break;
    }
  }

  // Jaro-Winkler = Jaro + (prefix * 0.1 * (1 - Jaro))
  return jaro + prefix * 0.1 * (1 - jaro);
}

/**
 * Calculate Token Set Ratio similarity between scenarios
 * Handles word reordering without penalty
 *
 * @param scenario1 - First scenario
 * @param scenario2 - Second scenario
 * @returns Similarity score (0-1)
 */
export function tokenSetRatio(
  scenario1: Scenario,
  scenario2: Scenario
): number {
  // Combine name and steps, strip Gherkin keywords
  const text1 = `${scenario1.name} ${scenario1.steps.join(' ')}`
    .toLowerCase()
    .replace(/\b(given|when|then|and|but)\b/gi, '')
    .trim();
  const text2 = `${scenario2.name} ${scenario2.steps.join(' ')}`
    .toLowerCase()
    .replace(/\b(given|when|then|and|but)\b/gi, '')
    .trim();

  // Tokenize into words
  const tokens1 = text1.split(/\s+/).filter(t => t.length > 0);
  const tokens2 = text2.split(/\s+/).filter(t => t.length > 0);

  if (tokens1.length === 0 && tokens2.length === 0) {
    return 1.0;
  }
  if (tokens1.length === 0 || tokens2.length === 0) {
    return 0.0;
  }

  // Create sorted sets
  const set1 = new Set(tokens1);
  const set2 = new Set(tokens2);

  // Calculate intersection and differences
  const intersection = new Set([...set1].filter(t => set2.has(t)));
  const diff1 = new Set([...set1].filter(t => !set2.has(t)));
  const diff2 = new Set([...set2].filter(t => !set1.has(t)));

  // Create sorted strings
  const sortedIntersection = [...intersection].sort().join(' ');
  const sortedSet1 = [...set1].sort().join(' ');
  const sortedSet2 = [...set2].sort().join(' ');

  // If sets are identical, return 1.0
  if (diff1.size === 0 && diff2.size === 0) {
    return 1.0;
  }

  // Calculate similarity using set overlap
  const unionSize = set1.size + set2.size - intersection.size;
  return intersection.size / unionSize;
}

/**
 * Calculate Trigram similarity between two strings
 * Good for typo tolerance and fuzzy matching
 *
 * @param str1 - First string
 * @param str2 - Second string
 * @returns Similarity score (0-1)
 */
export function trigramSimilarity(str1: string, str2: string): number {
  const s1 = str1.toLowerCase();
  const s2 = str2.toLowerCase();

  if (s1 === s2) {
    return 1.0;
  }

  if (s1.length === 0 && s2.length === 0) {
    return 1.0;
  }
  if (s1.length === 0 || s2.length === 0) {
    return 0.0;
  }

  // Extract trigrams with padding
  const extractTrigrams = (text: string): string[] => {
    const padded = `  ${text}  `; // Add padding
    const trigrams: string[] = [];
    for (let i = 0; i < padded.length - 2; i++) {
      trigrams.push(padded.substring(i, i + 3));
    }
    return trigrams;
  };

  const trigrams1 = extractTrigrams(s1);
  const trigrams2 = extractTrigrams(s2);

  if (trigrams1.length === 0 && trigrams2.length === 0) {
    return 1.0;
  }

  // Count common trigrams
  const set1 = new Set(trigrams1);
  const set2 = new Set(trigrams2);
  const intersection = [...set1].filter(t => set2.has(t)).length;

  // Dice coefficient: 2 * |intersection| / (|set1| + |set2|)
  return (2 * intersection) / (set1.size + set2.size);
}

/**
 * Calculate Jaccard similarity between scenarios based on raw tokens
 * Set-based similarity for token overlap (NO stopword filtering)
 *
 * @param scenario1 - First scenario
 * @param scenario2 - Second scenario
 * @returns Similarity score (0-1)
 */
export function jaccardSimilarity(
  scenario1: Scenario,
  scenario2: Scenario
): number {
  // Extract raw tokens (alphanumeric, NO stopword filtering)
  const extractTokens = (scenario: Scenario): Set<string> => {
    const text = `${scenario.name} ${scenario.steps.join(' ')}`
      .toLowerCase()
      .replace(/\b(given|when|then|and|but)\b/gi, '');

    // Extract alphanumeric tokens (supports OAuth2, SHA256, etc.)
    const tokens = text.match(/\b[a-z0-9]+\b/gi) || [];

    // NO stopword filtering - raw tokenization only
    return new Set(tokens.map(t => t.toLowerCase()));
  };

  const tokens1 = extractTokens(scenario1);
  const tokens2 = extractTokens(scenario2);

  // Handle empty token sets without division by zero
  if (tokens1.size === 0 && tokens2.size === 0) {
    return 1.0; // Both empty = identical
  }
  if (tokens1.size === 0 || tokens2.size === 0) {
    return 0.0; // One empty, one not = no similarity
  }

  // Calculate intersection and union
  const intersection = new Set([...tokens1].filter(t => tokens2.has(t)));
  const union = new Set([...tokens1, ...tokens2]);

  // Jaccard = |intersection| / |union|
  return intersection.size / union.size;
}

/**
 * Calculate Gherkin Structural similarity
 * Analyzes Given/When/Then sections separately with weighted outcomes
 *
 * @param scenario1 - First scenario
 * @param scenario2 - Second scenario
 * @returns Similarity score (0-1)
 */
export function gherkinStructuralSimilarity(
  scenario1: Scenario,
  scenario2: Scenario
): number {
  // Parse steps by type
  const parseSteps = (scenario: Scenario) => {
    const given: string[] = [];
    const when: string[] = [];
    const then: string[] = [];

    for (const step of scenario.steps) {
      const normalizedStep = step.toLowerCase().trim();
      if (normalizedStep.startsWith('given')) {
        given.push(step.replace(/^given\s+/i, '').toLowerCase());
      } else if (normalizedStep.startsWith('when')) {
        when.push(step.replace(/^when\s+/i, '').toLowerCase());
      } else if (normalizedStep.startsWith('then')) {
        then.push(step.replace(/^then\s+/i, '').toLowerCase());
      } else if (
        normalizedStep.startsWith('and') ||
        normalizedStep.startsWith('but')
      ) {
        // Add to the last category
        const cleaned = step.replace(/^(and|but)\s+/i, '').toLowerCase();
        if (then.length > 0) {
          then.push(cleaned);
        } else if (when.length > 0) {
          when.push(cleaned);
        } else if (given.length > 0) {
          given.push(cleaned);
        }
      }
    }

    return { given, when, then };
  };

  const steps1 = parseSteps(scenario1);
  const steps2 = parseSteps(scenario2);

  // Calculate Jaccard similarity for each section
  const jaccardForSteps = (steps1: string[], steps2: string[]): number => {
    if (steps1.length === 0 && steps2.length === 0) {
      return 1.0;
    }
    if (steps1.length === 0 || steps2.length === 0) {
      return 0.0;
    }

    const set1 = new Set(steps1);
    const set2 = new Set(steps2);
    const intersection = new Set([...set1].filter(s => set2.has(s)));
    const union = new Set([...set1, ...set2]);

    return intersection.size / union.size;
  };

  const givenSim = jaccardForSteps(steps1.given, steps2.given);
  const whenSim = jaccardForSteps(steps1.when, steps2.when);
  const thenSim = jaccardForSteps(steps1.then, steps2.then);

  // Weight Then steps 1.5x higher than Given/When
  // Formula: (given + when + then * 1.5) / 3.5
  const weightedScore = (givenSim + whenSim + thenSim * 1.5) / 3.5;

  return weightedScore;
}

/**
 * Calculate hybrid similarity using weighted combination of 5 algorithms
 *
 * Combines multiple similarity techniques for more robust matching:
 * - Jaro-Winkler: prefix matching
 * - Token Set: word reordering tolerance
 * - Trigram: typo tolerance
 * - Jaccard: keyword overlap
 * - Gherkin Structural: Given/When/Then awareness
 *
 * No specific accuracy guarantees - effectiveness depends on scenario content.
 *
 * @param scenario1 - First scenario
 * @param scenario2 - Second scenario
 * @param config - Optional algorithm weights configuration (auto-detects if not provided)
 * @returns Similarity score (0-1)
 */
export function hybridSimilarity(
  scenario1: Scenario,
  scenario2: Scenario,
  config?: SimilarityConfig
): number {
  // Auto-detect short strings and use appropriate config
  const titleLength = Math.min(
    scenario1.name.trim().length,
    scenario2.name.trim().length
  );

  const effectiveConfig =
    config ||
    (titleLength < 20 ? SHORT_STRING_CONFIG : DEFAULT_SIMILARITY_CONFIG);

  // Calculate individual algorithm scores
  const titleText1 = scenario1.name;
  const titleText2 = scenario2.name;

  const jaroWinklerScore = jaroWinklerSimilarity(titleText1, titleText2);
  const tokenSetScore = tokenSetRatio(scenario1, scenario2);
  const gherkinStructuralScore = gherkinStructuralSimilarity(
    scenario1,
    scenario2
  );

  // Trigram on combined text
  const combinedText1 = `${scenario1.name} ${scenario1.steps.join(' ')}`;
  const combinedText2 = `${scenario2.name} ${scenario2.steps.join(' ')}`;
  const trigramScore = trigramSimilarity(combinedText1, combinedText2);

  const jaccardScore = jaccardSimilarity(scenario1, scenario2);

  // Combine with configured weights
  const hybridScore =
    jaroWinklerScore * effectiveConfig.jaroWinklerWeight +
    tokenSetScore * effectiveConfig.tokenSetWeight +
    gherkinStructuralScore * effectiveConfig.gherkinStructuralWeight +
    trigramScore * effectiveConfig.trigramWeight +
    jaccardScore * effectiveConfig.jaccardWeight;

  return hybridScore;
}
