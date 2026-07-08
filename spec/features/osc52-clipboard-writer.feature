@done
@rust
@text-selection
@tui
@clipboard
@COPY-001
Feature: OSC 52 clipboard writer
  """
  New module codelet/fspec-tui/src/mouse/clipboard.rs (or clipboard/mod.rs). Public type Osc52Clipboard<W: Write + Send = std::io::Stdout>, mirroring the MouseTrackingToggle testing pattern (generic writer + with_stdout() production ctor + new(writer) test ctor).
  Encoding: use the base64 crate's STANDARD engine (base64::engine::general_purpose::STANDARD.encode(bytes)) on text.as_bytes(). Add base64 as a dependency to fspec-tui/Cargo.toml if not already present.
  Byte format: write b"\x1b]52;c;", then the base64 ascii, then b"\x07" (BEL). Use write!/write_all + flush. Method signature: fn copy(&mut self, text: &str) -> std::io::Result<()>.
  Testing: unit tests in the module inject a Vec<u8>, call copy(...), and assert the full byte slice equals the expected ESC]52;c;<b64>BEL. Cover ascii, empty, multiline, and emoji cases. No real stdout/terminal needed. Follows integration-first/redirect-not-intercept philosophy.
  Non-goals: no native clipboard crate (arboard), no OSC 52 READ/paste, no chunking for very large payloads (documented limitation; terminals may cap OSC 52 size). Consumer is COPY-006 which calls copy() on gesture Commit.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Writing text emits an OSC 52 sequence terminated with BEL: ESC ] 52 ; c ; <base64(utf8)> BEL
  #   2. The clipboard payload is the UTF-8 bytes of the text, standard-base64 encoded with padding
  #   3. The clipboard selection target is 'c' (the system clipboard)
  #   4. The writer is generic over W: Write + Send so tests can capture and assert the exact bytes
  #   5. Multi-byte UTF-8 (emoji, accented chars) is base64-encoded from its raw UTF-8 byte representation without corruption
  #   6. Writing an empty string still emits a well-formed OSC 52 sequence with an empty base64 payload; it does not panic
  #
  # EXAMPLES:
  #   1. Writing the ASCII text "hi" emits ESC ] 52 ; c ; aGk= BEL into the injected buffer
  #   2. Writing the empty string emits ESC ] 52 ; c ; BEL (empty base64 payload)
  #   3. Writing a multi-line string "a\nb" base64-encodes the bytes including the newline (YQpi)
  #   4. Writing an emoji "😀" base64-encodes its 4 UTF-8 bytes (8J+YgA==) rather than a code point
  #   5. The production constructor writes to std::io::stdout(); the test constructor writes to a Vec<u8>
  #
  # ========================================
  Background: User Story
    As a TUI user
    I want to have selected transcript text written to my system clipboard even while the TUI holds mouse capture and over SSH
    So that I can paste it elsewhere without the app needing native clipboard libraries

  Scenario: Writing ASCII text emits a BEL-terminated OSC 52 sequence
    Given an OSC 52 clipboard writer backed by an in-memory byte buffer
    When I copy the text "hi"
    Then the buffer contains the bytes ESC ] 52 ; c ; aGk= BEL

  Scenario: Writing an empty string emits an empty base64 payload
    Given an OSC 52 clipboard writer backed by an in-memory byte buffer
    When I copy the empty string
    Then the buffer contains the bytes ESC ] 52 ; c ; BEL
    And the copy call does not panic

  Scenario: Writing multi-line text base64-encodes the newline byte
    Given an OSC 52 clipboard writer backed by an in-memory byte buffer
    When I copy the text "a\nb"
    Then the base64 payload in the buffer is "YQpi"

  Scenario: Writing an emoji encodes its raw UTF-8 bytes
    Given an OSC 52 clipboard writer backed by an in-memory byte buffer
    When I copy the emoji "😀"
    Then the base64 payload in the buffer is "8J+YgA=="

  Scenario: The production constructor targets stdout and the test constructor targets a buffer
    Given the production constructor with_stdout is available
    And the test constructor new accepts any Write sink
    When I construct a writer over an in-memory byte buffer and copy "hi"
    Then the exact bytes are captured in that buffer without touching the real terminal
