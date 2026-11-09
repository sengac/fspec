@done
@rust
@code-analysis
@parser
@BUG-616
Feature: Code graph drops every entity of a file whose source is truncated at a multi-byte UTF-8 boundary (metadata.rs:98 panics)
  """
  metadata::extract_source caps a definition's text: the cut is the last newline at or before byte 4096 of the first 100 lines, or str::floor_char_boundary(4096) when that span holds no newline. A newline is ASCII, so both cuts are character boundaries. ast_pipeline::extract_file runs each extractor under catch_unwind, so a panic there used to drop the whole file (no File node, no mtime), which made incremental indexing count the file as new on every run.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A definition's stored source is capped at 100 lines and 4096 bytes and ends at the last complete line inside both caps; when the first line alone is longer than 4096 bytes it ends at the last character boundary at or before byte 4096. The stored source is always a prefix of the definition's text that ends on a character boundary, so capping never panics
  #   2. A definition whose capped source would be cut inside a multi-byte character is indexed together with every other entity of its file
  #   3. An indexed file that has not changed is not re-extracted by incremental indexing
  #
  # EXAMPLES:
  #   1. A Rust file holds fn wide_text, whose first 100 lines are longer than 4096 bytes with a 2-byte character across byte 4096, and fn after_wide; ast_index then ast_search finds both functions, and wide_text's source is truncated, at most 4096 bytes, ends at a complete line and is a prefix of its text
  #   2. A definition whose first line is 5000 bytes of 3-byte characters keeps the first 1365 characters (4095 bytes) and is marked truncated
  #   3. After a full index of that project, an incremental index with no file changed re-extracts 0 files
  #
  # ========================================
  Background: User Story
    As a agent using GraphSearch on a project
    I want to index files whose definitions contain multi-byte UTF-8 text near the source cap
    So that every definition of every file is in the code graph and an unchanged file is not re-read

  Scenario: A definition cut inside a multi-byte character is indexed with the rest of its file
    Given a project whose Rust file holds fn wide_text, whose first 100 lines are longer than 4096 bytes with a 2-byte character across byte 4096, and fn after_wide
    When ast_index runs for the project
    And ast_search lists the project's functions
    Then the results hold wide_text and after_wide
    And wide_text's source is marked truncated, is at most 4096 bytes, ends at a complete line and is a prefix of its text

  Scenario: A one-line definition longer than the byte cap keeps whole characters
    Given a project whose Rust file holds a one-line fn whose 5000 bytes are mostly 3-byte characters
    When ast_index runs for the project
    And ast_search lists the project's functions
    Then the function's source is its first 4095 bytes, ending on a character boundary, and is marked truncated

  Scenario: An unchanged file is not re-extracted by an incremental index
    Given a project whose Rust file holds fn wide_text with a 2-byte character across byte 4096 of its first 100 lines, and a second file
    And ast_index has run for the project
    When ast_index runs incrementally with no file changed
    Then it re-extracts 0 files
