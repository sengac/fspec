@CORE-010 @tools @ls @agent-tools @cli
Feature: Add LS Tool for directory listing

  """
  Architecture notes:
  - Implements rig::tool::Tool trait like other tools (GlobTool, GrepTool, etc.)
  - Uses std::fs for directory reading and file metadata
  - Output format matches TypeScript version: permissions size mtime name
  - Uses Node.js-equivalent fs.readdirSync with withFileTypes for directory listing
  - Applies consistent 30000 char output limit using shared OUTPUT_LIMITS and truncateOutput utility
  - Returns formatted output with file metadata (permissions, size, mtime, name)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Use std::fs::read_dir to get directory entries with type information
  #   2. Return file metadata including name, size, type (file/directory), and modification time
  #   3. Support optional path parameter to specify directory (defaults to cwd)
  #   4. Apply same output limits as other tools (30000 chars max) for consistency
  #   5. Return 'Directory not found' for non-existent paths
  #   6. Sort entries with directories first, then files, alphabetically within each group
  #   7. Handle permission errors gracefully without crashing
  #
  # EXAMPLES:
  #   1. List current directory returns files and subdirectories with metadata (name, size, type, mtime)
  #   2. List specific path like 'src/' returns only contents of that directory
  #   3. List non-existent directory '/nonexistent' returns 'Directory not found'
  #   4. List directory with many files truncates output at 30000 chars with warning
  #   5. Output format: 'drwxr-xr-x  4096  2025-01-15 10:30  src/' for directories, '-rw-r--r--  1234  2025-01-15 10:30  file.ts' for files
  #
  # ========================================

  Background: User Story
    As a AI coding agent
    I want to list directory contents with file metadata
    So that I can explore directory structure and understand project layout without using slow Bash ls commands

  Scenario: List directory returns files and subdirectories with metadata
    Given a directory with files and subdirectories
    When I list the directory contents
    Then all entries should be returned with metadata
    And directories should be listed before files
    And entries should be sorted alphabetically within each group

  Scenario: List specific path returns only contents of that directory
    Given a project with files in multiple directories
    When I list the directory 'src'
    Then only files from the src directory should be returned
    And parent directory contents should not be included

  Scenario: List non-existent directory returns error
    Given a project directory
    When I list a non-existent directory '/nonexistent'
    Then the result should be 'Directory not found'

  Scenario: Large directory listings are truncated at character limit
    Given a directory with many files exceeding output limit
    When I list the directory contents
    Then the output should be truncated at 30000 characters
    And a truncation warning should be included

  Scenario: Output format shows permissions, size, mtime, and name
    Given a directory with files and subdirectories
    When I list the directory contents
    Then directories should show format like 'drwxr-xr-x  4096  2025-01-15 10:30  dirname/'
    And files should show format like '-rw-r--r--  1234  2025-01-15 10:30  filename.ts'

  Scenario: List empty directory
    Given an empty directory exists
    When I invoke the LS tool on that directory
    Then the output is "(empty directory)"

  Scenario: List path that is a file
    Given a path that points to a file, not a directory
    When I invoke the LS tool on that path
    Then the output contains "Not a directory"

  Scenario: List with no path defaults to current directory
    Given no path is provided
    When I invoke the LS tool
    Then it lists the current working directory

  Scenario: Handle files with special characters in names
    Given a directory with files containing spaces and special characters
    When I list the directory contents
    Then files with spaces should be listed correctly
    And files with dashes should be listed correctly

  Scenario: Show file sizes in bytes
    Given a file with known size
    When I list the directory containing the file
    Then the file size in bytes should be displayed
