@CLI-019
Feature: CLI-019: Add large write intent detection and chunking guidance
  As a CLI user writing large codebases
  I want to get automatic chunking guidance when requesting large file writes
  So that I can avoid hitting maxOutputTokens limits and have my large writes succeed

  Background:
    Given the codelet CLI application is running
    And an LLM provider is configured

  # Problem statement:
  # Without chunking guidance, LLMs may attempt to write 500+ line files in a single
  # Write call, exceeding maxOutputTokens limits. The user doesn't know to ask the LLM
  # to break work into chunks. Detecting large write intent patterns and injecting
  # guidance solves this transparently.

  # ==========================================
  # PATTERN DETECTION TESTS
  # ==========================================

  Scenario: Detect large write intent from "complete" keywords
    Given a user prompt containing "write a complete REST API with all CRUD operations"
    When the large write intent detection runs
    Then it should detect large write intent
    And the pattern match should be on the keyword "complete"

  Scenario: Detect large write intent from "comprehensive" keywords
    Given a user prompt containing "create a comprehensive test suite for the auth module"
    When the large write intent detection runs
    Then it should detect large write intent
    And the pattern match should be on the keyword "comprehensive"

  Scenario: Detect large write intent from "entire" keywords
    Given a user prompt containing "implement the entire user management system"
    When the large write intent detection runs
    Then it should detect large write intent
    And the pattern match should be on the keyword "entire"

  Scenario: Detect large write intent from "full" keywords
    Given a user prompt containing "build a full authentication system with OAuth"
    When the large write intent detection runs
    Then it should detect large write intent
    And the pattern match should be on the keyword "full"

  Scenario: Do not detect large write intent for small tasks
    Given a user prompt containing "fix the typo on line 5"
    When the large write intent detection runs
    Then it should NOT detect large write intent

  Scenario: Do not detect large write intent for simple edits
    Given a user prompt containing "add a console.log statement to debug"
    When the large write intent detection runs
    Then it should NOT detect large write intent

  Scenario: Detect large write intent with line count indicators
    Given a user prompt containing "write a 500 line module for data processing"
    When the large write intent detection runs
    Then it should detect large write intent
    And the pattern match should be on line count indicator

  # ==========================================
  # SYSTEM REMINDER INJECTION TESTS
  # ==========================================

  Scenario: Inject chunking guidance when large write detected
    Given a user prompt with detected large write intent
    When preparing the prompt for the LLM
    Then a system-reminder should be injected into the conversation
    And the system-reminder should contain chunking guidance
    And the system-reminder should be invisible to the user

  Scenario: System reminder contains specific chunking instructions
    Given a user prompt with detected large write intent
    When a system-reminder is generated
    Then it should instruct the LLM to use multiple Write calls
    And it should recommend incremental file building
    And it should warn about maxOutputTokens limits

  Scenario: No system reminder for small tasks
    Given a user prompt without large write intent
    When preparing the prompt for the LLM
    Then no chunking guidance system-reminder should be injected

  # ==========================================
  # KEYWORD PATTERN CONSTANTS
  # ==========================================

  Scenario: Large write intent patterns are defined as constants
    Given the large_write_intent module constants
    Then LARGE_WRITE_KEYWORDS should include "complete", "comprehensive", "entire", "full"
    And LINE_COUNT_PATTERN should match numeric patterns like "500 lines", "1000+ line"
    And MULTIPLE_FILE_KEYWORDS should include "all files", "multiple files", "system"

  # ==========================================
  # EDGE CASES
  # ==========================================

  Scenario: Case-insensitive pattern matching
    Given a user prompt containing "Write a COMPLETE application"
    When the large write intent detection runs
    Then it should detect large write intent

  Scenario: Partial word matches should not trigger detection
    Given a user prompt containing "completely unrelated task"
    When the large write intent detection runs
    Then it should NOT detect large write intent
    # Because "completely" is not the same as "complete"

  Scenario: Detection works with surrounding context
    Given a user prompt containing "I need you to write a complete module for user authentication including registration, login, and password reset functionality"
    When the large write intent detection runs
    Then it should detect large write intent
