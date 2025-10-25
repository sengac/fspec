/**
 * Scenario similarity detection utilities
 *
 * Thin wrapper around hybrid similarity algorithm for scenario matching.
 * Uses 5-algorithm weighted combination for robust duplicate detection.
 */

import {
  hybridSimilarity,
  DEFAULT_SIMILARITY_CONFIG,
  type Scenario as AlgoScenario,
} from './similarity-algorithms';

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
 * Calculate similarity between two scenarios using hybrid algorithm
 *
 * Uses weighted combination of 5 algorithms for robust matching.
 * No specific accuracy guarantees - effectiveness depends on scenario content.
 *
 * @param scenario1 - First scenario
 * @param scenario2 - Second scenario
 * @returns Similarity score (0-1)
 */
export function calculateScenarioSimilarity(
  scenario1: Scenario,
  scenario2: Scenario
): number {
  // Delegate to hybridSimilarity with auto-detection
  return hybridSimilarity(scenario1 as AlgoScenario, scenario2 as AlgoScenario);
}

/**
 * Find matching scenarios across existing feature files
 *
 * Uses adaptive thresholds based on scenario title length:
 * - Very short (< 10 chars): 0.85 (strict)
 * - Short (10-20 chars): 0.80 (moderate)
 * - Medium (20-40 chars): 0.75 (normal)
 * - Long (40+ chars): 0.70 (lenient)
 *
 * @param targetScenario - Scenario to find matches for
 * @param existingFeatures - Existing feature files to search
 * @param threshold - Base threshold for long strings (default: 0.70)
 * @returns Array of matching scenarios sorted by similarity
 */
export function findMatchingScenarios(
  targetScenario: Scenario,
  existingFeatures: FeatureFile[],
  threshold = 0.7
): ScenarioMatch[] {
  const matches: ScenarioMatch[] = [];

  // Adaptive threshold based on title length
  const titleLength = targetScenario.name.trim().length;
  const adaptiveThreshold =
    titleLength < 10
      ? 0.85 // Very short - strict
      : titleLength < 20
        ? 0.8 // Short - moderate
        : titleLength < 40
          ? 0.75 // Medium - normal
          : threshold; // Long - use base threshold

  for (const feature of existingFeatures) {
    for (const scenario of feature.scenarios) {
      const similarity = calculateScenarioSimilarity(targetScenario, scenario);

      if (similarity >= adaptiveThreshold) {
        matches.push({
          feature: feature.name,
          scenario: scenario.name,
          similarityScore: similarity,
          featurePath: feature.path,
        });
      }
    }
  }

  // Sort by similarity score (highest first)
  return matches.sort((a, b) => b.similarityScore - a.similarityScore);
}

/**
 * Extract raw tokens from scenario for display purposes
 *
 * NOT semantic analysis - just alphanumeric tokenization.
 * Strips Gherkin syntax keywords (Given/When/Then/And/But).
 * No stopword filtering (all tokens included).
 *
 * @param scenario - Scenario to extract tokens from
 * @returns Array of lowercase tokens
 */
export function extractTokens(scenario: Scenario): string[] {
  const text = `${scenario.name} ${scenario.steps.join(' ')}`
    .toLowerCase()
    .replace(/\b(given|when|then|and|but)\b/gi, '');

  // Extract alphanumeric tokens (supports OAuth2, SHA256, etc.)
  const tokens = text.match(/\b[a-z0-9]+\b/gi) || [];

  // NO stopword filtering - raw tokenization only
  return tokens.map(t => t.toLowerCase());
}
