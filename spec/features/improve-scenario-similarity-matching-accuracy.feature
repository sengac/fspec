@done
@similarity-matching
@scenario-deduplication
@algorithm
@SPEC-003
Feature: Improve scenario similarity matching accuracy
  """
  Current Implementation: src/utils/scenario-similarity.ts uses Levenshtein distance (character-level edit distance) with 70/30 weighted scoring (title vs steps). Has 7 critical bugs and achieves only 60% accuracy.
  Proposed Hybrid Algorithm: Combine 5 algorithms with weighted scoring - Jaro-Winkler (30% - title matching), Token Set Ratio (25% - word reordering), Gherkin Structural (20% - Given/When/Then awareness), Trigram Similarity (15% - fuzzy matching), Jaccard Similarity (10% - keyword overlap). Expected 88% accuracy.
  Bug Fixes Required: (1) Division by zero for empty strings/keywords, (2) Strip Gherkin keywords before comparison, (3) Include numbers in keyword regex for technical terms, (4) Fix keyword extraction for match objects, (5) Remove redundant lowercasing, (6) Handle step reordering, (7) Support partial step matching.
  Test Coverage: Must cover empty strings, empty steps, empty keywords, special characters, numbers in terms (OAuth2/SHA256), short titles, reordered steps, partial matches, typos, and performance with 100+ scenarios. See src/commands/__tests__/scenario-deduplication.test.ts
  References: (1) spec/attachments/SPEC-003/SIMILARITY_ANALYSIS.md - Bug analysis and test gaps, (2) spec/attachments/SPEC-003/ALGORITHM_ANALYSIS.md - Algorithm comparison and implementation guide, (3) Jaro-Winkler, Token Set Ratio, Trigram algorithms ~200 lines total with no dependencies
  Performance:
  - All Phase 1+2 algorithms have no external dependencies and run in O(n*m) time or better. For large codebases (100+ scenarios), consider two-stage matching: (1) Fast keyword filtering, (2) Full similarity only on candidates.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Similarity scoring must handle empty strings without division by zero crashes
  #   2. Gherkin keywords (Given/When/Then/And/But) must be stripped from steps before similarity comparison
  #   3. Keyword extraction must include numbers and special characters (OAuth2, SHA256, Base64, etc.)
  #   4. Division by zero must be prevented when comparing scenarios with empty keyword sets
  #   5. Step reordering must not penalize similarity (Given A, Given B should match Given B, Given A)
  #   6. Partial step matching must be supported (3/4 identical steps should score high)
  #   7. Short scenario titles (< 20 chars) must use stricter thresholds to prevent false positives
  #   8. Hybrid algorithm must combine at least 5 different similarity algorithms with weighted scoring
  #   9. Overall accuracy must reach 88% (improvement from current 60%)
  #   10. Algorithm must handle word reordering without penalty (Token Set Ratio approach)
  #   11. Algorithm must handle typos and minor character variations (trigram similarity)
  #   12. Algorithm must respect Gherkin structure (Given/When/Then sections analyzed separately)
  #   13. Use 20 characters as threshold for short string detection (as recommended in attachments). Apply stricter threshold of 0.85 for short titles to prevent false positives.
  #   14. Yes - weight 'Then' steps 1.5x higher than 'Given/When' in Gherkin Structural analysis. Outcomes (Then) are more critical for determining scenario equivalence than preconditions/actions.
  #
  # EXAMPLES:
  #   1. BUG-1: Comparing two scenarios with empty titles should return 1.0 (both empty = identical) without division by zero crash
  #   2. BUG-2: Steps 'Given user exists, When login, Then success' should have keywords stripped to 'user exists, login, success' before comparison
  #   3. BUG-5: Technical terms like 'OAuth2', 'SHA256', 'Base64' must be extracted as keywords (regex must include numbers)
  #   4. BUG-4: Comparing scenarios with no extractable keywords should return 0 similarity without division by zero crash
  #   5. ISSUE-1: 'Given user exists, Given database connected, When login' should match 100% with 'Given database connected, Given user exists, When login' (reordered steps)
  #   6. ISSUE-2: Scenario A with steps [X, Y, Z, W] should score 75% step similarity with Scenario B with steps [X, Y, Z, Q] (3/4 match)
  #   7. ISSUE-3: 'User login' vs 'User logout' should NOT score 83.3% (false positive due to short strings)
  #   8. Jaro-Winkler: 'User login validation' vs 'User login verification' should score ~0.92 (better than Levenshtein for prefix matching)
  #   9. Token Set Ratio: 'Given user exists and database connected' vs 'Given database connected and user exists' should score 1.0 (word order independence)
  #   10. Trigram: 'authenticate' vs 'authentcate' (typo) should score ~0.85 (typo tolerance)
  #   11. Gherkin Structural: Compare Given/When/Then sections separately with Jaccard similarity, weight Then steps higher (outcomes matter more)
  #   12. Hybrid: Weighted combination of 5 algorithms (Jaro-Winkler 30%, Token Set 25%, Gherkin Structural 20%, Trigram 15%, Jaccard 10%) achieves 88% accuracy
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should algorithm weights (Jaro-Winkler 30%, Token Set 25%, etc.) be configurable via options, or hardcoded based on the analysis recommendations?
  #   A: true
  #
  #   Q: Should we implement Phase 1 only (80% accuracy, simpler) or both Phase 1 + Phase 2 (88% accuracy, more complex) in this work unit?
  #   A: true
  #
  #   Q: Are we okay with zero external dependencies (Phase 1+2 recommendation), or should we consider Phase 3 with Sentence-BERT ML library for 92% accuracy?
  #   A: true
  #
  #   Q: Should the similarity threshold (currently 0.7) be configurable per use case, or should we optimize it based on testing?
  #   A: true
  #
  #   Q: For short string bias handling, what should be the character threshold (attachments propose < 20 chars)? Should this be configurable?
  #   A: true
  #
  #   Q: Should 'Then' steps (outcomes) be weighted higher than 'Given/When' steps in Gherkin Structural analysis, as proposed in the attachments?
  #   A: true
  #
  #   Q: Should we prioritize speed or accuracy if there's a trade-off? (e.g., caching, two-stage filtering for large codebases)
  #   A: true
  #
  #   Q: Should we keep the old Levenshtein algorithm available via a flag for backward compatibility and comparison testing?
  #   A: true
  #
  # ASSUMPTIONS:
  #   1. Configurable via options/parameters - allows dynamic tuning if results aren't optimal
  #   2. Yes - implement both Phase 1 + Phase 2 for 88% accuracy target
  #   3. Investigate semantic-chunking (https://www.npmjs.com/package/semantic-chunking) and other npm packages for semantic similarity as potential enhancement
  #   4. No - threshold optimized based on testing, not user-configurable
  #   5. semantic-chunking provides semantic embeddings via ONNX models (all-MiniLM-L6-v2 at 23MB) and cosine similarity. Can be added as optional Phase 3 enhancement for semantic matching when higher accuracy needed beyond 88% target.
  #   6. Prioritize accuracy over speed for Phase 1+2. All algorithms (Jaro-Winkler, Token Set, Gherkin Structural, Trigram, Jaccard) are O(n*m) or better. For 100+ scenarios, implement two-stage filtering: (1) fast keyword filtering, (2) full similarity on candidates.
  #   7. Yes - keep old Levenshtein algorithm behind a flag for backward compatibility and A/B testing. Allows users to compare old vs new algorithm results during transition period.
  #
  # ========================================
  Background: User Story
    As a developer using fspec for scenario deduplication
    I want to have accurate scenario similarity matching that handles edge cases and different phrasings
    So that I can confidently detect duplicate scenarios and refactoring opportunities without false positives or false negatives

  Scenario: Handle empty scenario titles without division by zero crash
    Given I have two scenarios with empty titles
    When I calculate similarity between them
    Then the similarity score should be 1.0
    And no division by zero error should occur

  Scenario: Strip Gherkin keywords before step comparison
    Given I have a scenario with steps "Given user exists, When login, Then success"
    When I prepare steps for similarity comparison
    Then the keywords should be stripped to "user exists, login, success"
    And Gherkin keywords should not pollute similarity scores

  Scenario: Extract technical terms with numbers as keywords
    Given I have scenarios containing technical terms "OAuth2", "SHA256", "Base64"
    When I extract keywords from scenario text
    Then technical terms with numbers should be included in keyword set
    And keyword extraction regex should match alphanumeric patterns

  Scenario: Handle scenarios with no extractable keywords
    Given I have two scenarios with only stopwords
    When I calculate keyword-based similarity
    Then the similarity score should be 0
    And no division by zero error should occur

  Scenario: Match scenarios with reordered Given steps
    Given I have scenario A with steps "Given user exists, Given database connected, When login"
    And I have scenario B with steps "Given database connected, Given user exists, When login"
    When I calculate similarity between them
    Then the similarity score should be 1.0
    And step reordering should not penalize similarity

  Scenario: Support partial step matching
    Given I have scenario A with steps [X, Y, Z, W]
    And I have scenario B with steps [X, Y, Z, Q]
    When I calculate step similarity
    Then the similarity score should reflect 75% match
    And 3 out of 4 identical steps should score high

  Scenario: Prevent false positives for short scenario titles
    Given I have scenario titled "User login"
    And I have scenario titled "User logout"
    When I calculate title similarity
    Then the similarity score should be below threshold
    And short strings should use stricter threshold of 0.85

  Scenario: Jaro-Winkler algorithm for prefix matching
    Given I have scenario titled "User login validation"
    And I have scenario titled "User login verification"
    When I calculate similarity using Jaro-Winkler algorithm
    Then the similarity score should be approximately 0.92
    And it should outperform Levenshtein for prefix matching

  Scenario: Token Set Ratio for word order independence
    Given I have scenario with steps "Given user exists and database connected"
    And I have scenario with steps "Given database connected and user exists"
    When I calculate similarity using Token Set Ratio algorithm
    Then the similarity score should be 1.0
    And word reordering should not affect similarity

  Scenario: Trigram similarity for typo tolerance
    Given I have scenario with text "authenticate"
    And I have scenario with text "authentcate" (typo)
    When I calculate similarity using Trigram algorithm
    Then the similarity score should be approximately 0.85
    And minor character variations should be tolerated

  Scenario: Gherkin Structural analysis with weighted steps
    Given I have two scenarios with different Given/When/Then structures
    When I calculate similarity using Gherkin Structural algorithm
    Then Given/When/Then sections should be compared separately using Jaccard similarity
    And Then steps should be weighted 1.5x higher than Given/When steps
    And outcomes should matter more than preconditions for equivalence

  Scenario: Hybrid algorithm achieves 88% accuracy
    Given I have configured the hybrid algorithm with 5 algorithms
    And Jaro-Winkler is weighted at 30%
    And Token Set Ratio is weighted at 25%
    And Gherkin Structural is weighted at 20%
    And Trigram similarity is weighted at 15%
    And Jaccard similarity is weighted at 10%
    When I run the hybrid algorithm on test dataset
    Then overall accuracy should reach 88%
    And it should improve from current 60% baseline

  Scenario: Configure algorithm weights dynamically
    Given I have a hybrid similarity matcher with default weights
    When I adjust Jaro-Winkler weight from 30% to 40%
    And I adjust Token Set Ratio weight from 25% to 20%
    Then the algorithm should use the new weights
    And I should be able to tune results based on performance

  Scenario: Maintain backward compatibility with Levenshtein algorithm
    Given I have the new hybrid similarity algorithm
    When I enable legacy mode flag
    Then the old Levenshtein algorithm should be used
    And I should be able to compare old vs new results for A/B testing
