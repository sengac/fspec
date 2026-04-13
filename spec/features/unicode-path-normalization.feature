@BUG-130
Feature: Unicode Path Normalization — TypeScript fspec CLI

  """
  TypeScript-side normalization: src/utils/normalize-path.ts with normalizeFilePath()
  (sync, regex-based) and resolveFilePath() (async, three-phase: exact → normalized
  → directory scan). Applied at NAPI entry point (fspec-callback.ts) for positional
  args and named options. Used in add-attachment and attachment server for robust
  filesystem lookups.
  """

  Background: User Story
    As a user or AI agent
    I want to reference files with Unicode whitespace in their paths (e.g. macOS screenshots with U+202F before am/pm) using regular ASCII spaces
    So that file operations succeed regardless of which whitespace variant is used in the path

  Scenario: Normalize U+202F NARROW NO-BREAK SPACE to ASCII space
    Given a file path containing U+202F between "9.13.45" and "am"
    When I normalize the file path
    Then the U+202F character should be replaced with a regular ASCII space
    And the rest of the path should remain unchanged

  Scenario: Normalize all Unicode whitespace variants
    Given file paths containing U+00A0, U+1680, U+2000-U+200A, U+202F, U+205F, and U+3000
    When I normalize each file path
    Then every Unicode whitespace character should be replaced with ASCII space U+0020

  Scenario: Normalization is idempotent
    Given a file path that has already been normalized
    When I normalize it again
    Then the result should be identical to the first normalization

  Scenario: Path separators are preserved during normalization
    Given a file path with forward slashes and backslashes as separators
    When I normalize the file path
    Then all path separator characters should remain unchanged

  Scenario: ASCII-only paths pass through unchanged
    Given a file path containing only ASCII characters with regular spaces
    When I normalize the file path
    Then the path should be returned unchanged

  Scenario: Resolve file with U+202F when user types regular space
    Given a file on disk named with U+202F in its name
    When I resolve the path using a regular space instead of U+202F
    Then the file should be found via directory scan fallback
    And the returned path should point to the actual file on disk

  Scenario: Resolve file with regular space when user pastes U+00A0
    Given a file on disk named with regular ASCII spaces
    When I resolve the path using U+00A0 NO-BREAK SPACE instead
    Then the file should be found via normalized path lookup

  Scenario: Resolve returns exact path when file exists as-is
    Given a file on disk whose path matches exactly
    When I resolve the file path
    Then the exact original path should be returned without modification

  Scenario: NAPI callback normalizes positional args in both argv and setFspecPositionalArgs
    Given an AI agent invokes fspec via the NAPI callback with positional arguments containing U+202F
    When the callback builds the argv array and sets positional args
    Then both the argv passed to Commander and the args set via setFspecPositionalArgs should contain normalized ASCII-space paths

  Scenario: NAPI callback normalizes named option values through real callback processing
    Given an AI agent invokes fspec via the NAPI callback with named options containing U+00A0
    When the callback processes the named options into argv flags
    Then the option values in argv should contain normalized ASCII-space strings

  Scenario: Attachment server resolves Unicode-encoded paths via HTTP request
    Given a running attachment server and a file on disk with U+202F in its name
    When I make an HTTP GET request with the path URL-encoded using regular spaces
    Then the server should resolve the file via directory scan and return 200 with the file content

  Scenario: add-attachment resolves file with Unicode whitespace via resolveFilePath
    Given a file on disk named with U+202F and a work unit that exists
    When I call addAttachment with the path using a regular ASCII space
    Then the attachment should be added successfully with the file copied to the attachments directory
