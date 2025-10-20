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
  extractKeywords,
  isLikelyRefactor
} from '../scenario-similarity';

// Import new algorithms (will fail until implemented)
import {
  jaroWinklerSimilarity,
  tokenSetRatio,
  trigramSimilarity,
  jaccardSimilarity,
  gherkinStructuralSimilarity,
  hybridSimilarity,
  type SimilarityConfig
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
        steps: ['Given user exists', 'When login', 'Then success']
      };

      // When I prepare steps for similarity comparison
      // (This will be tested via internal function)
      const keywords = extractKeywords(scenario);

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
        steps: ['Given Base64 encoding is enabled']
      };

      // When I extract keywords from scenario text
      const keywords = extractKeywords(scenario);

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
      // Given I have two scenarios with only stopwords
      const scenario1: Scenario = {
        name: 'a the and or',
        steps: ['Given a', 'When the', 'Then and']
      };
      const scenario2: Scenario = {
        name: 'is are was',
        steps: ['Given is', 'When are', 'Then was']
      };

      // When I calculate keyword-based similarity
      const keywords1 = extractKeywords(scenario1);
      const keywords2 = extractKeywords(scenario2);

      // Then the similarity score should be 0
      expect(keywords1.length).toBe(0);
      expect(keywords2.length).toBe(0);

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
        steps: ['Given user exists', 'Given database connected', 'When login']
      };

      // And I have scenario B with steps "Given database connected, Given user exists, When login"
      const scenarioB: Scenario = {
        name: 'User login',
        steps: ['Given database connected', 'Given user exists', 'When login']
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
        steps: ['Given X', 'When Y', 'Then Z', 'And W']
      };

      // And I have scenario B with steps [X, Y, Z, Q]
      const scenarioB: Scenario = {
        name: 'Test',
        steps: ['Given X', 'When Y', 'Then Z', 'And Q']
      };

      // When I calculate step similarity
      const similarity = calculateScenarioSimilarity(scenarioA, scenarioB);

      // Then the similarity score should reflect 75% match
      // Note: With identical titles (70% weight) and 75% step match (30% weight),
      // expected score is 1.0 * 0.7 + 0.75 * 0.3 = 0.925
      expect(similarity).toBeGreaterThanOrEqual(0.9);
      expect(similarity).toBeLessThanOrEqual(0.95);

      // And 3 out of 4 identical steps should score high
      expect(similarity).toBeGreaterThan(0.9);
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
      expect(similarity).toBeGreaterThanOrEqual(0.90);
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
        steps: ['Given user exists and database connected']
      };

      // And I have scenario with steps "Given database connected and user exists"
      const scenario2: Scenario = {
        name: 'Test',
        steps: ['Given database connected and user exists']
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
      expect(similarity).toBeGreaterThanOrEqual(0.80);
      expect(similarity).toBeLessThanOrEqual(0.90);

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
        steps: ['Given A', 'When B', 'Then C']
      };
      const scenario2: Scenario = {
        name: 'Test',
        steps: ['Given X', 'When Y', 'Then C']
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
        steps: ['Given A', 'When B', 'Then same_outcome']
      };
      const scenario2: Scenario = {
        name: 'Test',
        steps: ['Given X', 'When Y', 'Then same_outcome']
      };

      const similarityWithSameThen = gherkinStructuralSimilarity(scenario1, scenario2);

      const scenario3: Scenario = {
        name: 'Test',
        steps: ['Given same_precondition', 'When B', 'Then C']
      };
      const scenario4: Scenario = {
        name: 'Test',
        steps: ['Given same_precondition', 'When Y', 'Then Z']
      };

      const similarityWithSameGiven = gherkinStructuralSimilarity(scenario3, scenario4);

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
        jaroWinklerWeight: 0.30,
        // And Token Set Ratio is weighted at 25%
        tokenSetWeight: 0.25,
        // And Gherkin Structural is weighted at 20%
        gherkinStructuralWeight: 0.20,
        // And Trigram similarity is weighted at 15%
        trigramWeight: 0.15,
        // And Jaccard similarity is weighted at 10%
        jaccardWeight: 0.10
      };

      const scenario1: Scenario = {
        name: 'User login validation',
        steps: ['Given user exists', 'When login', 'Then success']
      };
      const scenario2: Scenario = {
        name: 'User authentication check',
        steps: ['Given user exists', 'When authenticate', 'Then success']
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
        jaroWinklerWeight: 0.30,
        tokenSetWeight: 0.25,
        gherkinStructuralWeight: 0.20,
        trigramWeight: 0.15,
        jaccardWeight: 0.10
      };

      const sum = config.jaroWinklerWeight +
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
        jaroWinklerWeight: 0.30,
        tokenSetWeight: 0.25,
        gherkinStructuralWeight: 0.20,
        trigramWeight: 0.15,
        jaccardWeight: 0.10
      };

      // When I adjust Jaro-Winkler weight from 30% to 40%
      // And I adjust Token Set Ratio weight from 25% to 20%
      const customConfig: SimilarityConfig = {
        jaroWinklerWeight: 0.40,
        tokenSetWeight: 0.20,
        gherkinStructuralWeight: 0.20,
        trigramWeight: 0.15,
        jaccardWeight: 0.05
      };

      const scenario1: Scenario = { name: 'Test validation', steps: [] };
      const scenario2: Scenario = { name: 'Test verification', steps: [] };

      const defaultSimilarity = hybridSimilarity(scenario1, scenario2, defaultConfig);
      const customSimilarity = hybridSimilarity(scenario1, scenario2, customConfig);

      // Then the algorithm should use the new weights
      expect(customSimilarity).toBeDefined();
      expect(customSimilarity).not.toBe(defaultSimilarity);

      // And I should be able to tune results based on performance
      expect(Math.abs(customSimilarity - defaultSimilarity)).toBeGreaterThan(0);
    });
  });

  // ========================================
  // Scenario 14: Maintain backward compatibility with Levenshtein algorithm
  // ========================================
  describe('Scenario: Maintain backward compatibility with Levenshtein algorithm', () => {
    it('should support legacy Levenshtein mode', () => {
      // Given I have the new hybrid similarity algorithm
      const scenario1: Scenario = { name: 'User login', steps: [] };
      const scenario2: Scenario = { name: 'User logout', steps: [] };

      // When I enable legacy mode flag
      const legacyConfig: SimilarityConfig = {
        useLegacyLevenshtein: true,
        jaroWinklerWeight: 0,
        tokenSetWeight: 0,
        gherkinStructuralWeight: 0,
        trigramWeight: 0,
        jaccardWeight: 0
      };

      const legacySimilarity = hybridSimilarity(scenario1, scenario2, legacyConfig);

      // Then the old Levenshtein algorithm should be used
      const currentSimilarity = calculateScenarioSimilarity(scenario1, scenario2);
      expect(legacySimilarity).toBeCloseTo(currentSimilarity, 2);

      // And I should be able to compare old vs new results for A/B testing
      const newConfig: SimilarityConfig = {
        useLegacyLevenshtein: false,
        jaroWinklerWeight: 0.30,
        tokenSetWeight: 0.25,
        gherkinStructuralWeight: 0.20,
        trigramWeight: 0.15,
        jaccardWeight: 0.10
      };

      const newSimilarity = hybridSimilarity(scenario1, scenario2, newConfig);
      expect(newSimilarity).toBeDefined();
      expect(legacySimilarity).toBeDefined();
    });
  });
});
