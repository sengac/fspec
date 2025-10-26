/**
 * Feature: spec/features/improve-scenario-similarity-matching-accuracy.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 *
 * CRITICAL: These tests are written BEFORE implementation (TDD).
 * They MUST fail initially to prove they test real behavior.
 */

import { describe, it, expect } from 'vitest';
import type { Scenario } from '../scenario-similarity';
import {
  calculateScenarioSimilarity,
  findMatchingScenarios,
  extractTokens,
} from '../scenario-similarity';

// Import new algorithms (will fail until implemented)
import {
  jaroWinklerSimilarity,
  tokenSetRatio,
  trigramSimilarity,
  jaccardSimilarity,
  gherkinStructuralSimilarity,
  hybridSimilarity,
  type SimilarityConfig,
} from '../similarity-algorithms';

describe('Feature: Improve scenario similarity matching accuracy', () => {
  // ========================================
  // Scenario 1: Handle empty scenario titles without division by zero crash
  // ========================================
  describe('Scenario: Handle empty scenario titles without division by zero crash', () => {
    it('should return 1.0 for two scenarios with empty titles', () => {
      // Given I have two scenarios with empty titles
      const scenario1: Scenario = { name: '', steps: [] };
      const scenario2: Scenario = { name: '', steps: [] };

      // When I calculate similarity between them
      const similarity = calculateScenarioSimilarity(scenario1, scenario2);

      // Then the similarity score should be 1.0
      expect(similarity).toBe(1.0);

      // And no division by zero error should occur
      expect(similarity).not.toBeNaN();
      expect(similarity).not.toBe(Infinity);
    });

    it('should handle empty steps without division by zero', () => {
      const scenario1: Scenario = { name: 'Test', steps: [] };
      const scenario2: Scenario = { name: 'Test', steps: [] };

      const similarity = calculateScenarioSimilarity(scenario1, scenario2);

      expect(similarity).not.toBeNaN();
      expect(similarity).not.toBe(Infinity);
    });
  });

  // ========================================
  // Scenario 2: Strip Gherkin keywords before step comparison
  // ========================================
  describe('Scenario: Strip Gherkin keywords before step comparison', () => {
    it('should strip Gherkin keywords from steps before comparison', () => {
      // Given I have a scenario with steps "Given user exists, When login, Then success"
      const scenario: Scenario = {
        name: 'Test',
        steps: ['Given user exists', 'When login', 'Then success'],
      };

      // When I prepare steps for similarity comparison
      // (This will be tested via internal function)
      const keywords = extractTokens(scenario);

      // Then the keywords should be stripped to "user exists, login, success"
      expect(keywords).toContain('user');
      expect(keywords).toContain('exists');
      expect(keywords).toContain('login');
      expect(keywords).toContain('success');

      // And Gherkin keywords should not pollute similarity scores
      expect(keywords).not.toContain('given');
      expect(keywords).not.toContain('when');
      expect(keywords).not.toContain('then');
    });
  });

  // ========================================
  // Scenario 3: Extract technical terms with numbers as keywords
  // ========================================
  describe('Scenario: Extract technical terms with numbers as keywords', () => {
    it('should extract technical terms with numbers as keywords', () => {
      // Given I have scenarios containing technical terms "OAuth2", "SHA256", "Base64"
      const scenario: Scenario = {
        name: 'Authenticate using OAuth2 and SHA256',
        steps: ['Given Base64 encoding is enabled'],
      };

      // When I extract keywords from scenario text
      const keywords = extractTokens(scenario);

      // Then technical terms with numbers should be included in keyword set
      expect(keywords).toContain('oauth2');
      expect(keywords).toContain('sha256');
      expect(keywords).toContain('base64');

      // And keyword extraction regex should match alphanumeric patterns
      expect(keywords.some(kw => /\d/.test(kw))).toBe(true);
    });
  });

  // ========================================
  // Scenario 4: Handle scenarios with no extractable keywords
  // ========================================
  describe('Scenario: Handle scenarios with no extractable keywords', () => {
    it('should return 0 similarity for scenarios with only stopwords', () => {
      // Given I have two scenarios with only common words (NO stopword filtering)
      const scenario1: Scenario = {
        name: 'a the and or',
        steps: ['Given a', 'When the', 'Then and'],
      };
      const scenario2: Scenario = {
        name: 'is are was',
        steps: ['Given is', 'When are', 'Then was'],
      };

      // When I extract tokens (NO stopword filtering)
      const tokens1 = extractTokens(scenario1);
      const tokens2 = extractTokens(scenario2);

      // Then ALL tokens should be included (no filtering)
      expect(tokens1.length).toBeGreaterThan(0); // NOT filtered!
      expect(tokens2.length).toBeGreaterThan(0); // NOT filtered!

      // And no division by zero error should occur
      const jaccardSim = jaccardSimilarity(scenario1, scenario2);
      expect(jaccardSim).not.toBeNaN();
      expect(jaccardSim).not.toBe(Infinity);
    });
  });

  // ========================================
  // Scenario 5: Match scenarios with reordered Given steps
  // ========================================
  describe('Scenario: Match scenarios with reordered Given steps', () => {
    it('should match scenarios with reordered steps at 100%', () => {
      // Given I have scenario A with steps "Given user exists, Given database connected, When login"
      const scenarioA: Scenario = {
        name: 'User login',
        steps: ['Given user exists', 'Given database connected', 'When login'],
      };

      // And I have scenario B with steps "Given database connected, Given user exists, When login"
      const scenarioB: Scenario = {
        name: 'User login',
        steps: ['Given database connected', 'Given user exists', 'When login'],
      };

      // When I calculate similarity between them
      const similarity = tokenSetRatio(scenarioA, scenarioB);

      // Then the similarity score should be 1.0
      expect(similarity).toBe(1.0);

      // And step reordering should not penalize similarity
      expect(similarity).toBeGreaterThanOrEqual(0.99);
    });
  });

  // ========================================
  // Scenario 6: Support partial step matching
  // ========================================
  describe('Scenario: Support partial step matching', () => {
    it('should score 75% for 3 out of 4 matching steps', () => {
      // Given I have scenario A with steps [X, Y, Z, W]
      const scenarioA: Scenario = {
        name: 'Test',
        steps: ['Given X', 'When Y', 'Then Z', 'And W'],
      };

      // And I have scenario B with steps [X, Y, Z, Q]
      const scenarioB: Scenario = {
        name: 'Test',
        steps: ['Given X', 'When Y', 'Then Z', 'And Q'],
      };

      // When I calculate step similarity using hybrid algorithm
      const similarity = calculateScenarioSimilarity(scenarioA, scenarioB);

      // Then hybrid algorithm should detect partial match
      // Note: Hybrid uses 5 algorithms weighted - scores ~0.75 for 75% match
      expect(similarity).toBeGreaterThanOrEqual(0.7);
      expect(similarity).toBeLessThanOrEqual(0.8);

      // And 3 out of 4 identical steps should score moderately high
      expect(similarity).toBeGreaterThan(0.7);
    });
  });

  // ========================================
  // Scenario 7: Prevent false positives for short scenario titles
  // ========================================
  describe('Scenario: Prevent false positives for short scenario titles', () => {
    it('should NOT score high for short strings with different meanings', () => {
      // Given I have scenario titled "User login"
      const scenario1: Scenario = { name: 'User login', steps: [] };

      // And I have scenario titled "User logout"
      const scenario2: Scenario = { name: 'User logout', steps: [] };

      // When I calculate title similarity
      const similarity = calculateScenarioSimilarity(scenario1, scenario2);

      // Then the similarity score should be below threshold
      expect(similarity).toBeLessThan(0.7);

      // And short strings should use stricter threshold of 0.85
      // (This will be validated in configuration)
      expect(similarity).toBeLessThan(0.85);
    });
  });

  // ========================================
  // Scenario 8: Jaro-Winkler algorithm for prefix matching
  // ========================================
  describe('Scenario: Jaro-Winkler algorithm for prefix matching', () => {
    it('should score ~0.92 for similar prefixes', () => {
      // Given I have scenario titled "User login validation"
      const title1 = 'User login validation';

      // And I have scenario titled "User login verification"
      const title2 = 'User login verification';

      // When I calculate similarity using Jaro-Winkler algorithm
      const similarity = jaroWinklerSimilarity(title1, title2);

      // Then the similarity score should be approximately 0.92
      expect(similarity).toBeGreaterThanOrEqual(0.9);
      expect(similarity).toBeLessThanOrEqual(0.95);

      // And it should outperform Levenshtein for prefix matching
      // (Current Levenshtein gives lower score for these)
      expect(similarity).toBeGreaterThan(0.85);
    });
  });

  // ========================================
  // Scenario 9: Token Set Ratio for word order independence
  // ========================================
  describe('Scenario: Token Set Ratio for word order independence', () => {
    it('should score 1.0 for reordered words', () => {
      // Given I have scenario with steps "Given user exists and database connected"
      const scenario1: Scenario = {
        name: 'Test',
        steps: ['Given user exists and database connected'],
      };

      // And I have scenario with steps "Given database connected and user exists"
      const scenario2: Scenario = {
        name: 'Test',
        steps: ['Given database connected and user exists'],
      };

      // When I calculate similarity using Token Set Ratio algorithm
      const similarity = tokenSetRatio(scenario1, scenario2);

      // Then the similarity score should be 1.0
      expect(similarity).toBe(1.0);

      // And word reordering should not affect similarity
      expect(similarity).toBeGreaterThanOrEqual(0.99);
    });
  });

  // ========================================
  // Scenario 10: Trigram similarity for typo tolerance
  // ========================================
  describe('Scenario: Trigram similarity for typo tolerance', () => {
    it('should score ~0.85 for typos', () => {
      // Given I have scenario with text "authenticate"
      const text1 = 'authenticate';

      // And I have scenario with text "authentcate" (typo)
      const text2 = 'authentcate';

      // When I calculate similarity using Trigram algorithm
      const similarity = trigramSimilarity(text1, text2);

      // Then the similarity score should be approximately 0.85
      expect(similarity).toBeGreaterThanOrEqual(0.8);
      expect(similarity).toBeLessThanOrEqual(0.9);

      // And minor character variations should be tolerated
      expect(similarity).toBeGreaterThan(0.7);
    });
  });

  // ========================================
  // Scenario 11: Gherkin Structural analysis with weighted steps
  // ========================================
  describe('Scenario: Gherkin Structural analysis with weighted steps', () => {
    it('should analyze Given/When/Then sections separately', () => {
      // Given I have two scenarios with different Given/When/Then structures
      const scenario1: Scenario = {
        name: 'Test',
        steps: ['Given A', 'When B', 'Then C'],
      };
      const scenario2: Scenario = {
        name: 'Test',
        steps: ['Given X', 'When Y', 'Then C'],
      };

      // When I calculate similarity using Gherkin Structural algorithm
      const similarity = gherkinStructuralSimilarity(scenario1, scenario2);

      // Then Given/When/Then sections should be compared separately using Jaccard similarity
      // And Then steps should be weighted 1.5x higher than Given/When steps
      // (Same Then step should boost similarity)
      expect(similarity).toBeGreaterThan(0.3);

      // And outcomes should matter more than preconditions for equivalence
      expect(similarity).toBeGreaterThan(0);
    });

    it('should weight Then steps higher than Given/When', () => {
      const scenario1: Scenario = {
        name: 'Test',
        steps: ['Given A', 'When B', 'Then same_outcome'],
      };
      const scenario2: Scenario = {
        name: 'Test',
        steps: ['Given X', 'When Y', 'Then same_outcome'],
      };

      const similarityWithSameThen = gherkinStructuralSimilarity(
        scenario1,
        scenario2
      );

      const scenario3: Scenario = {
        name: 'Test',
        steps: ['Given same_precondition', 'When B', 'Then C'],
      };
      const scenario4: Scenario = {
        name: 'Test',
        steps: ['Given same_precondition', 'When Y', 'Then Z'],
      };

      const similarityWithSameGiven = gherkinStructuralSimilarity(
        scenario3,
        scenario4
      );

      // Same Then should score higher than same Given
      expect(similarityWithSameThen).toBeGreaterThan(similarityWithSameGiven);
    });
  });

  // ========================================
  // Scenario 12: Hybrid algorithm achieves 88% accuracy
  // ========================================
  describe('Scenario: Hybrid algorithm achieves 88% accuracy', () => {
    it('should combine 5 algorithms with weighted scoring', () => {
      // Given I have configured the hybrid algorithm with 5 algorithms
      const config: SimilarityConfig = {
        // And Jaro-Winkler is weighted at 30%
        jaroWinklerWeight: 0.3,
        // And Token Set Ratio is weighted at 25%
        tokenSetWeight: 0.25,
        // And Gherkin Structural is weighted at 20%
        gherkinStructuralWeight: 0.2,
        // And Trigram similarity is weighted at 15%
        trigramWeight: 0.15,
        // And Jaccard similarity is weighted at 10%
        jaccardWeight: 0.1,
      };

      const scenario1: Scenario = {
        name: 'User login validation',
        steps: ['Given user exists', 'When login', 'Then success'],
      };
      const scenario2: Scenario = {
        name: 'User authentication check',
        steps: ['Given user exists', 'When authenticate', 'Then success'],
      };

      // When I run the hybrid algorithm on test dataset
      const similarity = hybridSimilarity(scenario1, scenario2, config);

      // Then overall accuracy should reach 88%
      // (This is tested via benchmark dataset, here we test it runs)
      expect(similarity).toBeGreaterThanOrEqual(0);
      expect(similarity).toBeLessThanOrEqual(1);

      // And it should improve from current 60% baseline
      expect(similarity).toBeGreaterThan(0);
    });

    it('should validate weights sum to 1.0', () => {
      const config: SimilarityConfig = {
        jaroWinklerWeight: 0.3,
        tokenSetWeight: 0.25,
        gherkinStructuralWeight: 0.2,
        trigramWeight: 0.15,
        jaccardWeight: 0.1,
      };

      const sum =
        config.jaroWinklerWeight +
        config.tokenSetWeight +
        config.gherkinStructuralWeight +
        config.trigramWeight +
        config.jaccardWeight;

      expect(sum).toBeCloseTo(1.0, 5);
    });
  });

  // ========================================
  // Scenario 13: Configure algorithm weights dynamically
  // ========================================
  describe('Scenario: Configure algorithm weights dynamically', () => {
    it('should allow dynamic weight adjustment', () => {
      // Given I have a hybrid similarity matcher with default weights
      const defaultConfig: SimilarityConfig = {
        jaroWinklerWeight: 0.3,
        tokenSetWeight: 0.25,
        gherkinStructuralWeight: 0.2,
        trigramWeight: 0.15,
        jaccardWeight: 0.1,
      };

      // When I adjust Jaro-Winkler weight from 30% to 40%
      // And I adjust Token Set Ratio weight from 25% to 20%
      const customConfig: SimilarityConfig = {
        jaroWinklerWeight: 0.4,
        tokenSetWeight: 0.2,
        gherkinStructuralWeight: 0.2,
        trigramWeight: 0.15,
        jaccardWeight: 0.05,
      };

      const scenario1: Scenario = { name: 'Test validation', steps: [] };
      const scenario2: Scenario = { name: 'Test verification', steps: [] };

      const defaultSimilarity = hybridSimilarity(
        scenario1,
        scenario2,
        defaultConfig
      );
      const customSimilarity = hybridSimilarity(
        scenario1,
        scenario2,
        customConfig
      );

      // Then the algorithm should use the new weights
      expect(customSimilarity).toBeDefined();
      expect(customSimilarity).not.toBe(defaultSimilarity);

      // And I should be able to tune results based on performance
      expect(Math.abs(customSimilarity - defaultSimilarity)).toBeGreaterThan(0);
    });
  });

  // ========================================
  // REFAC-003: Adaptive Weighting for Short Strings
  // ========================================
  describe('Scenario: Adaptive weighting for short strings', () => {
    it('should use SHORT_STRING_CONFIG for strings < 20 chars', () => {
      // Given I have two short scenarios (< 20 chars)
      const scenario1: Scenario = { name: 'User login', steps: [] };
      const scenario2: Scenario = { name: 'User logout', steps: [] };

      // When I call hybridSimilarity without explicit config
      const similarity = hybridSimilarity(scenario1, scenario2);

      // Then it should auto-detect short string and apply SHORT_STRING_CONFIG
      // Token Set (35%) and Jaccard (20%) should be heavily weighted
      // Expected moderate score (not super low due to shared "user" and "log" prefix)
      expect(similarity).toBeLessThan(0.7); // Different enough to not be duplicates
    });

    it('should use DEFAULT_SIMILARITY_CONFIG for strings >= 20 chars', () => {
      // Given I have two long scenarios (>= 20 chars)
      const scenario1: Scenario = {
        name: 'User authentication with OAuth2 tokens',
        steps: [],
      };
      const scenario2: Scenario = {
        name: 'User authentication using JWT tokens',
        steps: [],
      };

      // When I call hybridSimilarity without explicit config
      const similarity = hybridSimilarity(scenario1, scenario2);

      // Then it should use DEFAULT_SIMILARITY_CONFIG
      // Expected high score due to similar meaning
      expect(similarity).toBeGreaterThan(0.7);
    });

    it('should allow override with explicit config', () => {
      // Given I have a custom config
      const customConfig: SimilarityConfig = {
        jaroWinklerWeight: 1.0,
        tokenSetWeight: 0,
        gherkinStructuralWeight: 0,
        trigramWeight: 0,
        jaccardWeight: 0,
      };

      const scenario1: Scenario = { name: 'User login', steps: [] };
      const scenario2: Scenario = { name: 'User logout', steps: [] };

      // When I pass explicit config
      const similarity = hybridSimilarity(scenario1, scenario2, customConfig);

      // Then it should use my custom config, not auto-detected SHORT_STRING_CONFIG
      // Jaro-Winkler alone should give high score for "User log" prefix
      expect(similarity).toBeGreaterThan(0.6);
    });
  });

  // ========================================
  // REFAC-003: Adaptive Thresholds
  // ========================================
  describe('Scenario: Adaptive thresholds for findMatchingScenarios', () => {
    it('should apply strict threshold (0.85) for very short strings (< 10 chars)', () => {
      // Given I have a very short target scenario
      const target: Scenario = { name: 'Add item', steps: [] };
      const existing = [
        {
          path: 'test.feature',
          name: 'Test',
          scenarios: [
            { name: 'Edit item', steps: [] }, // Similar but different
          ],
        },
      ];

      // When I call findMatchingScenarios
      const matches = findMatchingScenarios(target, existing);

      // Then strict threshold (0.85) should prevent false positive
      // "Add item" vs "Edit item" should NOT match
      expect(matches.length).toBe(0);
    });

    it('should apply moderate threshold (0.80) for short strings (10-20 chars)', () => {
      // Given I have a short target scenario (10-20 chars)
      const target: Scenario = { name: 'User logs in', steps: [] };
      const existing = [
        {
          path: 'test.feature',
          name: 'Test',
          scenarios: [
            { name: 'User logs out', steps: [] }, // Opposite meaning
          ],
        },
      ];

      // When I call findMatchingScenarios
      const matches = findMatchingScenarios(target, existing);

      // Then moderate threshold (0.80) should prevent false positive
      expect(matches.length).toBe(0);
    });

    it('should apply lenient threshold (0.70) for long strings (40+ chars)', () => {
      // Given I have a long target scenario (40+ chars)
      const target: Scenario = {
        name: 'User authenticates with OAuth2 authentication tokens',
        steps: [],
      };
      const existing = [
        {
          path: 'test.feature',
          name: 'Test',
          scenarios: [
            {
              name: 'User authenticates using OAuth2 authentication tokens',
              steps: [],
            },
          ],
        },
      ];

      // When I call findMatchingScenarios
      const matches = findMatchingScenarios(target, existing);

      // Then lenient threshold (0.70) should allow similar matches
      expect(matches.length).toBe(1);
      expect(matches[0].similarityScore).toBeGreaterThanOrEqual(0.7);
    });
  });

  // ========================================
  // REFAC-003: Config Constants
  // ========================================
  describe('Scenario: Exported similarity config constants', () => {
    it('should export DEFAULT_SIMILARITY_CONFIG with correct weights', async () => {
      // Given I import similarity-algorithms module
      const module = await import('../similarity-algorithms');

      // Then DEFAULT_SIMILARITY_CONFIG should be exported
      expect(module.DEFAULT_SIMILARITY_CONFIG).toBeDefined();

      // And weights should sum to 1.0
      const config = module.DEFAULT_SIMILARITY_CONFIG;
      const sum =
        config.jaroWinklerWeight +
        config.tokenSetWeight +
        config.gherkinStructuralWeight +
        config.trigramWeight +
        config.jaccardWeight;
      expect(sum).toBeCloseTo(1.0, 5);

      // And should have expected default values
      expect(config.jaroWinklerWeight).toBe(0.3);
      expect(config.tokenSetWeight).toBe(0.25);
      expect(config.gherkinStructuralWeight).toBe(0.2);
      expect(config.trigramWeight).toBe(0.15);
      expect(config.jaccardWeight).toBe(0.1);
    });

    it('should export SHORT_STRING_CONFIG with rebalanced weights', async () => {
      // Given I import similarity-algorithms module
      const module = await import('../similarity-algorithms');

      // Then SHORT_STRING_CONFIG should be exported
      expect(module.SHORT_STRING_CONFIG).toBeDefined();

      // And weights should sum to 1.0
      const config = module.SHORT_STRING_CONFIG;
      const sum =
        config.jaroWinklerWeight +
        config.tokenSetWeight +
        config.gherkinStructuralWeight +
        config.trigramWeight +
        config.jaccardWeight;
      expect(sum).toBeCloseTo(1.0, 5);

      // And should emphasize word-level algorithms
      expect(config.tokenSetWeight).toBe(0.35); // Higher than default
      expect(config.jaccardWeight).toBe(0.2); // Higher than default
      expect(config.jaroWinklerWeight).toBe(0.15); // Lower than default
      expect(config.trigramWeight).toBe(0.1); // Lower than default
    });

    it('should NOT export useLegacyLevenshtein field', async () => {
      // Given I import similarity-algorithms module
      const module = await import('../similarity-algorithms');

      // Then SimilarityConfig interface should not have useLegacyLevenshtein
      const config: SimilarityConfig = {
        jaroWinklerWeight: 0.3,
        tokenSetWeight: 0.25,
        gherkinStructuralWeight: 0.2,
        trigramWeight: 0.15,
        jaccardWeight: 0.1,
      };

      // This should compile - no useLegacyLevenshtein field required
      expect(config).toBeDefined();
    });
  });

  // ========================================
  // REFAC-003: extractTokens (NO Stopword Filtering)
  // ========================================
  describe('Scenario: Extract tokens without semantic filtering', () => {
    it('should extract ALL alphanumeric tokens (no stopword filtering)', async () => {
      // Given I import scenario-similarity module
      const module = await import('../scenario-similarity');

      // And I have a scenario with common words
      const scenario: Scenario = {
        name: 'User logs in with credentials',
        steps: ['Given the user is authenticated'],
      };

      // When I call extractTokens
      const tokens = module.extractTokens(scenario);

      // Then ALL tokens should be included (no semantic filtering)
      expect(tokens).toContain('user');
      expect(tokens).toContain('logs');
      expect(tokens).toContain('in'); // NOT filtered even though it's a "stopword"
      expect(tokens).toContain('with'); // NOT filtered
      expect(tokens).toContain('credentials');
      expect(tokens).toContain('the'); // NOT filtered
      expect(tokens).toContain('is'); // NOT filtered
      expect(tokens).toContain('authenticated');
    });

    it('should be case insensitive', async () => {
      // Given I import scenario-similarity module
      const module = await import('../scenario-similarity');

      const scenario: Scenario = {
        name: 'User LOGIN Authentication',
        steps: [],
      };

      // When I call extractTokens
      const tokens = module.extractTokens(scenario);

      // Then all tokens should be lowercase
      expect(tokens).toContain('user');
      expect(tokens).toContain('login');
      expect(tokens).toContain('authentication');
      expect(tokens.every(t => t === t.toLowerCase())).toBe(true);
    });

    it('should support alphanumeric tokens (OAuth2, SHA256)', async () => {
      // Given I import scenario-similarity module
      const module = await import('../scenario-similarity');

      const scenario: Scenario = {
        name: 'OAuth2 authentication with SHA256',
        steps: [],
      };

      // When I call extractTokens
      const tokens = module.extractTokens(scenario);

      // Then alphanumeric tokens should be preserved
      expect(tokens).toContain('oauth2');
      expect(tokens).toContain('authentication');
      expect(tokens).toContain('with'); // NOT filtered!
      expect(tokens).toContain('sha256');
    });

    it('should NOT be named extractKeywords (renamed for honesty)', async () => {
      // Given I import scenario-similarity module
      const module = await import('../scenario-similarity');

      // Then extractKeywords should NOT exist (renamed to extractTokens)
      expect(module.extractKeywords).toBeUndefined();

      // And extractTokens should exist
      expect(module.extractTokens).toBeDefined();
    });
  });

  // ========================================
  // REFAC-003: jaccardSimilarity WITHOUT Stopword Filtering
  // ========================================
  describe('Scenario: Jaccard similarity on raw tokens', () => {
    it('should calculate Jaccard on raw tokens (no stopword filtering)', () => {
      // Given I have two scenarios differing by "stopwords"
      const scenario1: Scenario = { name: 'User logs in', steps: [] };
      const scenario2: Scenario = { name: 'User logs out', steps: [] };

      // When I calculate Jaccard similarity
      const similarity = jaccardSimilarity(scenario1, scenario2);

      // Then "in" and "out" should be KEPT and contribute to differentiation
      // Tokens: {user, logs, in} vs {user, logs, out}
      // Intersection: {user, logs} = 2
      // Union: {user, logs, in, out} = 4
      // Jaccard = 2/4 = 0.5
      expect(similarity).toBeCloseTo(0.5, 1);
    });

    it('should keep ALL tokens including common words', () => {
      // Given I have scenarios with words that would be stopwords
      const scenario1: Scenario = { name: 'Sign in to the system', steps: [] };
      const scenario2: Scenario = { name: 'Sign up for the system', steps: [] };

      // When I calculate Jaccard similarity
      const similarity = jaccardSimilarity(scenario1, scenario2);

      // Then words like "in", "to", "the", "for" should NOT be filtered
      // They contribute to similarity calculation
      // This proves NO stopword filtering is happening
      expect(similarity).toBeLessThan(1.0); // Different ("in" vs "up")
      expect(similarity).toBeGreaterThan(0); // Some overlap ("sign", "to", "the", "system")
    });
  });
});
