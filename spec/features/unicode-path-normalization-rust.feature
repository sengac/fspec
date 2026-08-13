@BUG-130
Feature: Unicode Path Normalization — Rust Codelet Tools
  """
  macOS uses U+202F (NARROW NO-BREAK SPACE) before am/pm in screenshot filenames.
  When users or AI agents type these paths with regular ASCII spaces, all file
  operations fail with ENOENT. All codelet Rust tools (Read, Write, Edit, Grep,
  Glob, Ls, AstGrep) pass file paths through validate_and_resolve_path() in
  wrapper.rs and require_file_exists() in validation.rs — these must normalize
  Unicode whitespace and perform directory-scan fallback. The Rust normalization
  lives in rust/tools/src/unicode_path.rs. AST research confirms 21 call sites
  for validate_and_resolve_path across all tool files.
  """

  Background: User Story
    As a user or AI agent
    I want to reference files with Unicode whitespace in their paths (e.g. macOS screenshots with U+202F before am/pm) using regular ASCII spaces
    So that the Read, Write, Edit, Grep, Glob, Ls, and AstGrep tools all find the correct file instead of returning 'File not found'

  # ========================================
  # Rust utility: resolve_unicode_path (async, directory scan)
  # ========================================
  Scenario: Resolve file with U+202F on disk when user types regular space
    Given a file on disk named with U+202F in its name
    When I call resolve_unicode_path with a path using a regular space instead of U+202F
    Then the file should be found via parent directory scan
    And the returned path should point to the actual file on disk containing U+202F

  Scenario: Resolve file with regular space on disk when user pastes U+00A0
    Given a file on disk named with regular ASCII spaces
    When I call resolve_unicode_path with U+00A0 NO-BREAK SPACE instead of regular space
    Then the file should be found via normalized path lookup in phase 1b

  # ========================================
  # Integration: validate_and_resolve_path (wrapper.rs)
  # ========================================
  Scenario: validate_and_resolve_path normalizes Unicode whitespace before canonicalization
    Given a directory on disk containing a file with U+202F in its name
    When I call validate_and_resolve_path with a path using regular ASCII spaces
    Then the returned PathBuf should point to the actual file on disk
    And the normalization should have occurred before any canonicalize or exists checks

  # ========================================
  # Integration: require_file_exists directory-scan fallback (validation.rs)
  # ========================================
  Scenario: require_file_exists finds file via directory scan when normalized path also fails
    Given a file on disk named "Screenshot 2026-04-13 at 9.13.45\u202fam.txt"
    When I call require_file_exists with path "Screenshot 2026-04-13 at 9.13.45 am.txt" containing regular space
    Then it should succeed by finding the file via directory scan fallback
    And the resolved path used for subsequent I/O should point to the actual file

  # ========================================
  # Integration: Read tool end-to-end
  # ========================================
  @integration
  Scenario: Read tool reads file with U+202F when user provides regular space path
    Given a text file on disk named with U+202F before "am" containing known content
    When I call ReadTool.call() with file_path using regular ASCII space instead of U+202F
    Then the tool should return the file content successfully
    And the output should contain the known content with line numbers

  @integration
  Scenario: Read tool reads image with U+202F when user provides regular space path
    Given a PNG image file on disk named "Screenshot 2026-04-13 at 9.13.45\u202fam.png"
    When I call ReadTool.call() with file_path "Screenshot 2026-04-13 at 9.13.45 am.png" using regular space
    Then the tool should return base64-encoded image data
    And the media_type should be "image/png"

  # ========================================
  # Integration: Edit tool end-to-end
  # ========================================
  @integration
  Scenario: Edit tool edits file with U+202F when user provides regular space path
    Given a text file on disk named with U+202F containing "old content"
    When I call EditTool.call() with file_path using regular space and old_string "old content" new_string "new content"
    Then the edit should succeed
    And the file on disk should contain "new content"

  # ========================================
  # Integration: Write tool normalizes Unicode in new file path
  # ========================================
  @integration
  Scenario: Write tool normalizes Unicode whitespace in file path for new files
    Given a target directory exists
    When I call WriteTool.call() with file_path containing U+00A0 NO-BREAK SPACE and some content
    Then the file should be created with regular ASCII spaces in its name
    And the file content should be written correctly

  # ========================================
  # Integration: Ls tool with Unicode path
  # ========================================
  @integration
  Scenario: Ls tool lists directory when path contains Unicode whitespace
    Given a directory on disk whose path contains U+202F
    When I call LsTool with the path using regular ASCII space
    Then the directory listing should be returned successfully
