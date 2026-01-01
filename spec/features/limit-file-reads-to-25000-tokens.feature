@caching
@tools
@high
@PROV-002
Feature: Limit File Reads to 25000 Tokens

  """
  Architecture notes:
  - Add tiktoken-rs dependency with cl100k_base encoding for accurate token counting
  - Create shared TokenEstimator in codelet/common/src/token_estimator.rs wrapping tiktoken-rs
  - TokenEstimator provides count_tokens(text: &str) -> usize method
  - Error type: TokenLimit with tool, file_path, estimated_tokens, and max_tokens fields
  - Exempt file types detected by extension AND magic bytes: .png, .jpg, .jpeg, .gif, .webp, .svg, .pdf, .ipynb
  - Configuration via CODELET_MAX_FILE_TOKENS env var (default 25000)
  - Integration: Called in read tool before returning content for text files only

  Migration scope - replace existing byte-based estimation:
  - codelet/cli/src/interactive_helpers.rs: estimate_tokens() function - MIGRATED
  - codelet/napi/src/persistence/storage.rs: estimate_tokens() function - MIGRATED
  - codelet/core/src/compaction/compactor.rs: token estimation - MIGRATED
  - codelet/cli/src/interactive/stream_loop.rs: inline estimation - MIGRATED
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Token estimation uses tiktoken-rs with cl100k_base encoding for accurate token counting
  #   2. File reads exceeding 25,000 tokens throw a descriptive error with file size, estimated tokens, and limit
  #   3. Token limit applies only to text files - images and PDFs are exempt (they have different processing)
  #   4. Token limit is configurable via environment variable or config, defaulting to 25,000
  #   5. Partial reads (offset/limit parameters) are validated after extracting the requested portion, not the full file
  #   6. Create a shared TokenEstimator utility in codelet/core that wraps tiktoken-rs for reuse across the codebase
  #   7. Replace existing byte-based token estimation (text.len() / 3.0 or / 4) with tiktoken-rs throughout the codebase
  #
  # EXAMPLES:
  #   1. A 50KB TypeScript file is read successfully (under 25,000 tokens)
  #   2. A 200KB minified JavaScript file throws error: 'File exceeds token limit'
  #   3. A 5MB PNG image is read successfully (exempt from token limit)
  #   4. A 10MB PDF is read successfully (exempt from token limit)
  #   5. Partial read of lines 1-100 from a 500KB file succeeds if extracted content is under 25,000 tokens
  #   6. Setting CODELET_MAX_FILE_TOKENS=50000 allows reading larger files up to 50,000 tokens
  #   7. Replace estimate_tokens() in interactive_helpers.rs with TokenEstimator::count_tokens()
  #   8. Replace estimate_tokens() in napi/persistence/storage.rs with shared TokenEstimator
  #   9. Update compactor.rs token estimation to use tiktoken-rs instead of len()/4 approximation
  #
  # ========================================

  Background: User Story
    As a developer using codelet for code exploration
    I want to have file reads automatically limited to 25,000 tokens
    So that my context window isn't bloated with excessively large files

  # ----------------------------------------
  # SUCCESS SCENARIOS
  # ----------------------------------------

  Scenario: Read file under token limit
    Given a TypeScript file "/project/src/app.ts" with 50KB of content
    When the read tool is called for "/project/src/app.ts"
    Then the file content should be returned successfully
    And no token limit error should be raised

  Scenario: Read image file exempt from token limit
    Given a PNG image file "/project/assets/logo.png" with 5MB of content
    When the read tool is called for "/project/assets/logo.png"
    Then the image should be processed successfully
    And the token limit check should be skipped

  Scenario: Read PDF file exempt from token limit
    Given a PDF file "/project/docs/manual.pdf" with 10MB of content
    When the read tool is called for "/project/docs/manual.pdf"
    Then the PDF should be processed successfully
    And the token limit check should be skipped

  Scenario: Partial read within token limit
    Given a large file "/project/src/large.ts" with 500KB of content
    When the read tool is called with offset=0 and limit=100
    Then only the requested portion should be checked for token limits
    And the content should be returned if under 25,000 tokens

  # ----------------------------------------
  # ERROR SCENARIOS
  # ----------------------------------------

  Scenario: Read file exceeding token limit throws error
    Given a minified JavaScript file "/project/dist/bundle.js" with 200KB of content
    When the read tool is called for "/project/dist/bundle.js"
    Then a token limit error should be raised
    And the error message should include the estimated token count
    And the error message should include the configured limit of 25,000
    And the error message should include the file path

  # ----------------------------------------
  # CONFIGURATION SCENARIOS
  # ----------------------------------------

  Scenario: Custom token limit via environment variable
    Given the environment variable CODELET_MAX_FILE_TOKENS is set to 50000
    And a JavaScript file "/project/dist/bundle.js" with 150KB of content
    When the read tool is called for "/project/dist/bundle.js"
    Then the file should be read successfully
    And the custom limit of 50,000 tokens should be applied

  # ----------------------------------------
  # TOKEN ESTIMATION WITH TIKTOKEN-RS
  # ----------------------------------------

  Scenario: Token estimation uses tiktoken-rs cl100k_base encoding
    Given a text file with English content
    When the token count is estimated using TokenEstimator
    Then the estimation should use tiktoken-rs cl100k_base encoding
    And the result should be more accurate than byte-based approximation

  Scenario: TokenEstimator is shared across codebase
    Given the TokenEstimator utility in codelet/core
    When token estimation is needed in any module
    Then the shared TokenEstimator should be used
    And no duplicate estimation logic should exist

  # ----------------------------------------
  # MIGRATION SCENARIOS
  # ----------------------------------------

  Scenario: Replace byte-based estimation in interactive_helpers
    Given the existing estimate_tokens() function in interactive_helpers.rs
    When the migration is complete
    Then the function should use TokenEstimator::count_tokens()
    And the APPROX_BYTES_PER_TOKEN constant should be removed

  Scenario: Replace byte-based estimation in persistence storage
    Given the existing estimate_tokens() function in napi/persistence/storage.rs
    When the migration is complete
    Then the function should use the shared TokenEstimator
    And token counts for persisted messages should be more accurate

  Scenario: Replace byte-based estimation in compactor
    Given the inline token estimation in compactor.rs
    When the migration is complete
    Then compaction decisions should use TokenEstimator
    And summary token estimation should be more accurate
