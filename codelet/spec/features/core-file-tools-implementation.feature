@tool-execution
@scaffold
@CORE-002
Feature: Core File Tools Implementation

  """
  Uses std::fs for file operations - no external dependencies required
  Implements tools as structs that impl Tool trait from src/tools/mod.rs
  OUTPUT_LIMITS const: MAX_OUTPUT_CHARS=30000, MAX_LINE_LENGTH=2000, MAX_LINES=2000
  ToolOutput must track truncation state (was_truncated, remaining_count)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All file tools (Read, Write, Edit) must require absolute paths - relative paths must be rejected with an error
  #   2. Read tool must return file contents with 1-based line numbers in format 'N: content'
  #   3. Read tool must support optional offset (1-based line number) and limit (number of lines) parameters
  #   4. All tool output must be truncated at 30000 characters maximum with truncation warning appended
  #   5. Read tool must truncate lines longer than 2000 characters with ellipsis
  #   6. Edit tool must replace only the FIRST occurrence of old_string (no global replace)
  #   7. Edit tool must return an error if old_string is not found in the file
  #   8. Write tool must create parent directories if they do not exist
  #
  # EXAMPLES:
  #   1. Read /home/user/src/index.ts returns numbered lines like '1: import fs from fs;', '2: ...'
  #   2. Read file with offset=50, limit=100 returns lines 50 through 149 only
  #   3. Read file > 2000 lines truncates and appends '... [N lines truncated] ...'
  #   4. Read file with relative path 'src/main.rs' returns error 'Error: file_path must be absolute'
  #   5. Write to /home/user/new.ts creates the file and returns 'Successfully wrote to /home/user/new.ts'
  #   6. Write to existing file /home/user/old.ts overwrites content without backup
  #   7. Edit /home/user/main.rs replacing 'foo' with 'bar' replaces first occurrence only
  #   8. Edit file with old_string='xyz123' not found returns 'Error: old_string not found in file'
  #
  # ========================================

  Background: User Story
    As a AI coding agent
    I want to read, write, and edit files on the filesystem
    So that I can help developers modify code, create new files, and understand existing code

  # ==========================================
  # READ TOOL SCENARIOS
  # ==========================================

  Scenario: Read file returns contents with 1-based line numbers
    Given a file exists at absolute path "/home/user/src/index.ts" with content:
      """
      import fs from 'fs';
      import path from 'path';
      """
    When I execute the Read tool with file_path "/home/user/src/index.ts"
    Then the output should contain "1: import fs from 'fs';"
    And the output should contain "2: import path from 'path';"

  Scenario: Read file with offset and limit returns specified line range
    Given a file exists at absolute path "/home/user/large.ts" with 200 lines
    When I execute the Read tool with file_path "/home/user/large.ts" offset 50 and limit 100
    Then the output should start with line number 50
    And the output should contain exactly 100 lines
    And the output should end with line number 149

  Scenario: Read file exceeding line limit is truncated with warning
    Given a file exists at absolute path "/home/user/huge.ts" with 3000 lines
    When I execute the Read tool with file_path "/home/user/huge.ts"
    Then the output should contain at most 2000 lines
    And the output should end with a truncation warning
    And the truncation warning should indicate the remaining line count

  Scenario: Read file with relative path returns error
    When I execute the Read tool with file_path "src/main.rs"
    Then the output should contain "Error: file_path must be absolute"
    And the tool execution should indicate an error

  Scenario: Read file truncates long lines with ellipsis
    Given a file exists at absolute path "/home/user/wide.ts" with a line exceeding 2000 characters
    When I execute the Read tool with file_path "/home/user/wide.ts"
    Then lines exceeding 2000 characters should be truncated
    And truncated lines should end with "..."

  Scenario: Read non-existent file returns error
    When I execute the Read tool with file_path "/home/user/nonexistent.ts"
    Then the output should contain "Error: File not found"
    And the tool execution should indicate an error

  # ==========================================
  # WRITE TOOL SCENARIOS
  # ==========================================

  Scenario: Write tool creates new file successfully
    Given the file "/home/user/new.ts" does not exist
    When I execute the Write tool with file_path "/home/user/new.ts" and content "export const foo = 1;"
    Then the output should contain "Successfully wrote to /home/user/new.ts"
    And the file "/home/user/new.ts" should exist with the written content

  Scenario: Write tool overwrites existing file
    Given a file exists at absolute path "/home/user/old.ts" with content "old content"
    When I execute the Write tool with file_path "/home/user/old.ts" and content "new content"
    Then the output should contain "Successfully wrote to /home/user/old.ts"
    And the file should contain "new content"
    And the file should not contain "old content"

  Scenario: Write tool creates parent directories if missing
    Given the directory "/home/user/nested/deep/" does not exist
    When I execute the Write tool with file_path "/home/user/nested/deep/file.ts" and content "content"
    Then the output should contain "Successfully wrote"
    And the file "/home/user/nested/deep/file.ts" should exist

  Scenario: Write tool with relative path returns error
    When I execute the Write tool with file_path "relative/path.ts" and content "content"
    Then the output should contain "Error: file_path must be absolute"
    And the tool execution should indicate an error

  # ==========================================
  # EDIT TOOL SCENARIOS
  # ==========================================

  Scenario: Edit tool replaces first occurrence only
    Given a file exists at absolute path "/home/user/main.rs" with content:
      """
      let foo = 1;
      let foo = 2;
      """
    When I execute the Edit tool with file_path "/home/user/main.rs" old_string "foo" and new_string "bar"
    Then the output should contain "Successfully edited"
    And the file should contain "let bar = 1;"
    And the file should contain "let foo = 2;"

  Scenario: Edit tool returns error when old_string not found
    Given a file exists at absolute path "/home/user/test.rs" with content "hello world"
    When I execute the Edit tool with file_path "/home/user/test.rs" old_string "xyz123" and new_string "replacement"
    Then the output should contain "Error: old_string not found in file"
    And the tool execution should indicate an error
    And the file content should be unchanged

  Scenario: Edit tool with relative path returns error
    When I execute the Edit tool with file_path "relative.ts" old_string "a" and new_string "b"
    Then the output should contain "Error: file_path must be absolute"
    And the tool execution should indicate an error

  Scenario: Edit non-existent file returns error
    When I execute the Edit tool with file_path "/home/user/missing.ts" old_string "a" and new_string "b"
    Then the output should contain "Error: File not found"
    And the tool execution should indicate an error
