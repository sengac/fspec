@CMPCT-006
Feature: Layer 0 — Structurally Lossless Trimmer Module
  """
  Trimmer struct lives in rust/core/src/compaction/trimmer.rs. Input type is StoredMessage from rust/napi/src/persistence/types.rs with fields: role (String), content (String), metadata (HashMap<String, Value>). Tool information is in metadata. Output is transformed content String.
  Trimmer must be re-exported via rust/core/src/compaction/mod.rs with pub mod trimmer and pub use trimmer::Trimmer. Consumed by CMPCT-010 SessionSearch trimming integration.
  Tool detection uses metadata HashMap keys — tool calls from AssistantContent::ToolUse are serialized with tool name and input parameters in the metadata. Common patterns: metadata contains 'tool_name' or structured tool use blocks that include the tool name (Read, Write, Edit, Bash, Grep, AstGrep, Glob, Ls, etc.).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Read tool results (file content) must be replaced with compact reference: [file: {path}, {lines} lines, {tokens} tok — use Read to retrieve]
  #   2. Write tool parameters (full content) must be replaced with: [Write: {path}, {lines} lines — file persisted to disk]
  #   3. Edit tool parameters (old_string/new_string) must be condensed to: [Edit: {path} — replaced {old_len} chars with {new_len} chars]
  #   4. Bash tool output (stdout/stderr) must be truncated to first 10 lines + '... ({N} lines omitted)' + last 5 lines + exit code
  #   5. Base64 image data must be stripped entirely and replaced with: [image: {W}x{H}, {bytes} bytes, from {path}]
  #   6. Search/Grep tool output must be truncated to first 10 matches + '... ({N} more matches)'
  #   7. User messages must NEVER be modified by trimming — pass through unchanged
  #   8. Assistant reasoning text must NEVER be modified by trimming — pass through unchanged
  #   9. Messages without tool metadata in their metadata HashMap must pass through unchanged
  #   10. Trimming is deterministic — same input always produces same output (no randomness, no LLM calls)
  #   11. All trimming is fully reversible by re-reading the persisted originals via SessionSearch
  #
  # EXAMPLES:
  #   1. Read tool returns 500-line file content → trimmed to '[file: src/main.rs, 500 lines, 12500 tok — use Read to retrieve]' (~70 chars instead of 12500 tokens)
  #   2. Write tool includes full 200-line file body in content → trimmed to '[Write: src/auth.rs, 200 lines — file persisted to disk]'
  #   3. Edit tool has 50-char old_string and 80-char new_string → trimmed to '[Edit: src/auth.rs — replaced 50 chars with 80 chars]'
  #   4. Bash output has 200 lines → trimmed to first 10 lines + '... (185 lines omitted)' + last 5 lines + exit code 0
  #   5. Bash output has only 14 lines (≤15 threshold) → passed through unchanged (no truncation needed)
  #   6. Message content contains base64 image data:image/png;base64,iVBOR... → trimmed to '[image: 800x600, 45000 bytes, from screenshot.png]'
  #   7. Grep tool returns 50 matching lines → trimmed to first 10 matches + '... (40 more matches)'
  #   8. User message 'please fix the login bug' → passed through unchanged (never trimmed)
  #   9. Assistant reasoning 'I need to check the error handling in...' → passed through unchanged (never trimmed)
  #   10. Message with role=user and no tool metadata → passed through with zero transformation
  #
  # ========================================
  Background: User Story
    As a agent performing DAG construction
    I want to receive structurally lossless trimmed tool outputs from SessionSearch
    So that use 20-86% fewer tokens processing retrieved history without losing any semantic information

  Scenario: Trim Read tool result to compact file reference
    Given a StoredMessage with role "user" containing 500 lines of file content
    And the message metadata indicates a Read tool result for path "src/main.rs"
    When the Trimmer processes the message
    Then the content should be replaced with a compact reference "[file: src/main.rs, 500 lines, {tokens} tok — use Read to retrieve]"
    And the trimmed output should be significantly smaller than the original

  Scenario: Trim Write tool parameters to persistence reference
    Given a StoredMessage with role "assistant" containing a Write tool use
    And the Write tool input includes full file content of 200 lines for path "src/auth.rs"
    When the Trimmer processes the message
    Then the file content should be replaced with "[Write: src/auth.rs, 200 lines — file persisted to disk]"

  Scenario: Condense Edit tool parameters to change summary
    Given a StoredMessage with role "assistant" containing an Edit tool use
    And the Edit tool input has a 50-character old_string and 80-character new_string for path "src/auth.rs"
    When the Trimmer processes the message
    Then the edit parameters should be condensed to "[Edit: src/auth.rs — replaced 50 chars with 80 chars]"

  Scenario: Truncate long Bash output with head and tail
    Given a StoredMessage with role "user" containing Bash tool output of 200 lines
    And the message metadata indicates a Bash tool result with exit code 0
    When the Trimmer processes the message
    Then the output should contain the first 10 lines of the original output
    And the output should contain "... (185 lines omitted)"
    And the output should contain the last 5 lines of the original output
    And the output should include the exit code

  Scenario: Short Bash output passes through unchanged
    Given a StoredMessage with role "user" containing Bash tool output of 14 lines
    And the message metadata indicates a Bash tool result
    When the Trimmer processes the message
    Then the content should pass through completely unchanged

  Scenario: Strip base64 image data to metadata placeholder
    Given a StoredMessage containing base64-encoded image data
    And the image metadata indicates dimensions 800x600 and source path "screenshot.png"
    When the Trimmer processes the message
    Then the base64 data should be replaced with "[image: 800x600, {bytes} bytes, from screenshot.png]"

  Scenario: Truncate Search/Grep output to first matches
    Given a StoredMessage with role "user" containing Grep tool output with 50 matching lines
    And the message metadata indicates a Grep tool result
    When the Trimmer processes the message
    Then the output should contain the first 10 matches
    And the output should end with "... (40 more matches)"

  Scenario: User messages pass through unchanged
    Given a StoredMessage with role "user" and content "please fix the login bug"
    And the message has no tool metadata
    When the Trimmer processes the message
    Then the content should be exactly "please fix the login bug"
    And zero transformation should have been applied

  Scenario: Assistant reasoning text passes through unchanged
    Given a StoredMessage with role "assistant" containing reasoning text
    And the content is "I need to check the error handling in the auth module"
    And the message has no tool use metadata
    When the Trimmer processes the message
    Then the content should be exactly "I need to check the error handling in the auth module"

  Scenario: Messages without tool metadata pass through unchanged
    Given a StoredMessage with role "user" and arbitrary content
    And the message metadata HashMap is empty
    When the Trimmer processes the message
    Then the content should be returned unchanged

  Scenario: Trimming is deterministic
    Given a StoredMessage with role "user" containing Bash tool output of 200 lines
    When the Trimmer processes the same message twice
    Then both outputs should be identical byte-for-byte

  Scenario: Trimmed output is measurably smaller than input
    Given a StoredMessage with role "user" containing Read tool output of 1000 lines
    When the Trimmer processes the message
    Then the character count of the trimmed output should be less than 10% of the original
