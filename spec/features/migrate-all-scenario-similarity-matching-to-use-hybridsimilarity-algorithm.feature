@feature-management
@done
@high
@parser
@refactoring
Feature: Scenario similarity matching with hybrid algorithm

  """
  
  WHAT IS NOT SEMANTIC (Allowed):
  ✅ Tokenization - splitting text into words (language-agnostic pattern matching)
  ✅ Alphanumeric extraction - regex /\b[a-z0-9]+\b/gi (supports OAuth2, SHA256)
  ✅ Case normalization - toLowerCase() (simple transformation)
  ✅ Character-level algorithms - Jaro-Winkler, Trigram (no word meaning required)
  ✅ Set operations - Jaccard on raw tokens (just math on sets)
  
  WHAT IS SEMANTIC (NOT Allowed):
  ❌ Stopword filtering - requires English dictionary knowledge
  ❌ Stemming - 'login' vs 'logs in' (requires linguistic rules)
  ❌ Synonyms - 'authenticate' vs 'login' (requires semantic dictionary)
  ❌ Word sense - 'bank' financial vs riverbank (requires context understanding)
  ❌ Intent detection - 'fix' vs 'refactor' keywords (requires semantic judgment)
  
  RENAME: extractKeywords() → extractTokens()
  Honest naming: tokens are raw words, not semantically-selected keywords.
  
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All scenario similarity calculations use hybridSimilarity() from src/utils/similarity-algorithms.ts
  #   2. The calculateScenarioSimilarity() function uses hybridSimilarity() internally
  #   3. Commands using scenario-similarity.ts (generate-scenarios, audit-scenarios) work without breaking changes
  #   4. Duplicate detection achieves 88% accuracy
  #
  # EXAMPLES:
  #   1. Developer runs 'fspec generate-scenarios AUTH-001'. Internally, calculateScenarioSimilarity() calls hybridSimilarity() with proper config weights. Duplicate detection works with 88% accuracy.
  #   2. Audit-scenarios command runs on 100 scenarios. Uses hybridSimilarity for deduplication. Correctly identifies near-duplicates.
  #
  # ========================================

  Background: User Story
    As a developer using fspec for ACDD
    I want to have accurate scenario duplicate detection
    So that I avoid creating redundant feature files

  Scenario: Scenario similarity uses hybridSimilarity internally
    Given the scenario-similarity module is loaded
    When I call calculateScenarioSimilarity with two scenarios
    Then it should use hybridSimilarity from similarity-algorithms.ts
    And duplicate detection should achieve 88% accuracy

  Scenario: Improved matching accuracy with keyword overlap
    Given I have two scenarios: "User logs in with valid credentials" and "Valid user authentication"
    When I calculate similarity using the hybridSimilarity algorithm
    Then the match score should reflect keyword overlap and structural analysis
    And the score should be greater than 0.5

  Scenario: Audit-scenarios detects near-duplicates
    Given I have 100 scenarios in my feature files
    When I run the audit-scenarios command
    Then it should use hybridSimilarity for deduplication
    And it should correctly identify near-duplicates
    And it should report them for review

  Scenario: generate-scenarios command uses hybridSimilarity for duplicate detection
    Given I run generate-scenarios on a work unit with examples
    When the command calls findMatchingScenarios()
    Then it should use hybridSimilarity internally
    And duplicate detection accuracy should be 88%
    And similar scenarios should be correctly identified

  Scenario: audit-scenarios command uses hybridSimilarity for deduplication
    Given I run audit-scenarios on 100 scenarios
    When the command calls calculateScenarioSimilarity()
    Then it should use hybridSimilarity internally
    And it should detect near-duplicates
    And the results should be based on keyword matching and structural analysis
