@done
@bug-fix
@tool-execution
@tools
@bug
@BUG-143
Feature: Audit Glob/attachment file-reading surfaces under Rhai providers for silent binary drops
  """
  Refactor bash_binary_guard detect_bash_binary_output + BinaryKind into shared module so Edit/apply_patch can reuse the same detector
  Introduce new helper 'detect_binary_file' in codelet_tools::binary_guard (or promote bash_binary_guard → binary_guard module with bash_ prefix removed) + tool-specific formatter for each surface
  Insert guard in validation::read_file_contents? No — keep read_file_contents byte-agnostic. Instead: each tool runs the guard on the raw bytes BEFORE calling from_utf8, so error message can name the specific tool (Edit / apply_patch)
  Read file as bytes (tokio::fs::read), run detect_binary_file on first 8 KiB, then if not binary decode via String::from_utf8
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Audit finding: Glob tool returns only file paths — no binary bytes ever cross the tool boundary, so no guard needed
  #   2. Audit finding: fspec work unit attachments (spec/attachments/**) are managed by fspec CLI commands, not read by any Rhai-exposed tool, so out of scope
  #   3. Audit finding: Grep tool delegates to ripgrep which already detects and skips binary files — no silent-drop risk
  #   4. Edit tool must detect binary content (NUL bytes + magic bytes of PNG/JPEG/GIF/WebP/PDF/ELF/ZIP/gzip) before attempting UTF-8 decode and return a structured ToolError::Validation directing the agent to Read
  #   5. apply_patch Update operation must detect binary content the same way and short-circuit with the same structured error message
  #   6. Binary detection reuses codelet_tools::bash_binary_guard::detect_bash_binary_output for consistency — single source of truth
  #   7. Valid UTF-8 text files continue to pass through unchanged — the guard is a no-op for text
  #
  # EXAMPLES:
  #   1. Agent calls Edit on a PNG file — tool returns structured error 'Edit target is a binary file (detected: PNG image). Use the Read tool to view images.'
  #   2. Agent calls Edit on a plain UTF-8 text file — binary guard is a no-op, existing behaviour (find/replace) proceeds normally
  #   3. Agent submits apply_patch Update targeting a PDF — tool returns structured error naming PDF and suggesting Read tool
  #   4. Agent submits apply_patch Update on a normal source file — guard no-op, patch applies normally
  #   5. Agent calls Edit on a file containing a raw NUL byte but no known magic — guard returns 'Edit target is a binary file (detected: binary data).'
  #   6. Agent calls Edit on an ELF binary — guard returns 'Edit target is a binary file (detected: ELF binary).'
  #   7. Edit guard inspects only the first 8 KiB of file bytes before deciding — no need to slurp huge binaries fully
  #   8. Glob returns ['spec/attachments/WU-001/diagram.png', 'src/main.rs'] — Glob itself does not load file bytes, so no binary-guard needed here
  #
  # ========================================
  Background: User Story
    As a developer using a Rhai custom provider
    I want to have Edit/apply_patch and similar file-reading tools surface a clear 'binary file' error instead of a confusing UTF-8 decode failure
    So that I can quickly understand when I've targeted a binary file and redirect to the Read tool instead of re-trying the same operation blindly

  Scenario: Edit rejects PNG file with named binary-guard error
    Given a file at "/tmp/icon.png" whose first 8 bytes are the PNG magic signature
    When the Edit tool is invoked with file_path "/tmp/icon.png" and any old_string/new_string
    Then the tool returns a ToolError::Validation
    And the error message contains "detected PNG image"
    And the error message instructs the agent to use the Read tool instead
    And the file on disk is unchanged

  Scenario: Edit rejects PDF file with named binary-guard error
    Given a file at "/tmp/report.pdf" whose first 5 bytes are "%PDF-"
    When the Edit tool is invoked with file_path "/tmp/report.pdf" and any old_string/new_string
    Then the tool returns a ToolError::Validation
    And the error message contains "detected PDF document"
    And the error message instructs the agent to use the Read tool instead
    And the file on disk is unchanged

  Scenario: Edit rejects ELF binary with generic binary-guard error
    Given a file at "/tmp/program" whose first 4 bytes are 0x7F 0x45 0x4C 0x46 (ELF)
    When the Edit tool is invoked with file_path "/tmp/program" and any old_string/new_string
    Then the tool returns a ToolError::Validation
    And the error message contains "detected binary content"
    And the error message does not name PNG, JPEG, GIF, WebP, or PDF

  Scenario: Edit rejects file containing raw NUL bytes with generic binary-guard error
    Given a file at "/tmp/blob.bin" whose bytes start with 0x00 0x01 0x02 followed by text
    When the Edit tool is invoked with file_path "/tmp/blob.bin" and any old_string/new_string
    Then the tool returns a ToolError::Validation
    And the error message contains "detected binary content"

  Scenario: Edit on a UTF-8 text file succeeds unchanged
    Given a file at "/tmp/notes.md" containing "# Hello world" as UTF-8 text
    When the Edit tool is invoked with file_path "/tmp/notes.md", old_string "Hello world", new_string "Goodbye"
    Then the tool succeeds
    And the file on disk now contains "# Goodbye"
    And no binary-guard error is emitted

  Scenario: Edit on a UTF-8 text file containing emoji and CJK succeeds unchanged
    Given a file at "/tmp/i18n.txt" containing "café 👋 中文 résumé" as UTF-8 text
    When the Edit tool is invoked with file_path "/tmp/i18n.txt", old_string "café", new_string "CAFE"
    Then the tool succeeds
    And no binary-guard error is emitted

  Scenario: apply_patch Update rejects PDF target with named binary-guard error
    Given a file at "/tmp/report.pdf" whose first 5 bytes are "%PDF-"
    When the apply_patch tool is invoked with an Update operation targeting "/tmp/report.pdf"
    Then the tool returns a ToolError::Validation
    And the error message contains "detected PDF document"
    And the error message instructs the agent to use the Read tool instead
    And the file on disk is unchanged

  Scenario: apply_patch Update rejects PNG target with named binary-guard error
    Given a file at "/tmp/icon.png" whose first 8 bytes are the PNG magic signature
    When the apply_patch tool is invoked with an Update operation targeting "/tmp/icon.png"
    Then the tool returns a ToolError::Validation
    And the error message contains "detected PNG image"
    And the file on disk is unchanged

  Scenario: apply_patch Update on UTF-8 source file applies cleanly
    Given a file at "/tmp/src.rs" containing valid UTF-8 Rust source
    When the apply_patch tool is invoked with an Update operation replacing existing lines
    Then the tool succeeds
    And the file on disk reflects the updated content
    And no binary-guard error is emitted

  Scenario: Binary guard inspects only the first 8 KiB of a large file
    Given a file at "/tmp/big.bin" of 10 MiB whose first 8 bytes are the PNG magic signature
    When the Edit tool is invoked with file_path "/tmp/big.bin"
    Then the tool returns a ToolError::Validation naming PNG
    And the guard does not read more than 8 KiB of bytes from disk for detection purposes

  Scenario: Glob has no binary-guard surface because it never returns file bytes
    Given a directory tree containing "spec/attachments/WU-001/diagram.png" and "src/main.rs"
    When the Glob tool is invoked with pattern "**/*"
    Then the output is a list of file paths
    And no file contents are loaded by the Glob tool
    And no binary-guard error is emitted
