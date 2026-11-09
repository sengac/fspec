@wip
@utils
@safety
@bug-fix
@rust
@BUG-642
Feature: Text cuts land on char boundaries
  """
  Six places cut text at a fixed byte index (&text[..N] or &key[key.len() - 4..]) and panic when the cut falls inside a multi-byte character: the CLI tool-result preview (cli/src/interactive/output.rs), the thinking-exhaustion recovery message (cli/src/interactive/recovery_thinking.rs), the AgentManager quoted-context turns and query paths (agent-loop/src/agent_manager_handler.rs), the TUI /loop list prompt column (fspec-tui/src/app/dispatch_slash_loop.rs) and mask_api_key (providers/src/credentials.rs). File-type sniffing (tools/src/file_type.rs) decodes a fixed byte window as UTF-8 and misses an SVG or ipynb file whose window ends inside a character.
  Each cut now rounds to a char boundary with str::floor_char_boundary (ceil_char_boundary for a suffix); the agent-loop cut is one shared truncate_context_turn for both paths and the CLI preview is tool_result_preview, so tests reach them. The sniffing window drops only an incomplete character at its end (Utf8Error::error_len() is None); an invalid byte inside it still means the data is not text.
  A workspace guard (test-helpers) parses every rust/*/src file with syn and reports each range slice bounded by an integer literal or a constant; each must be on an exact list of slices of ASCII-only text (UUID and hex-hash prefixes) or of non-text data.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A text preview or truncation cuts at the largest char boundary at or below its byte limit (str::floor_char_boundary), so it never panics and never splits a character
  #   2. mask_api_key keeps the first 6 bytes rounded down to a char boundary and the last 4 bytes rounded up to a char boundary, so a non-ASCII key is masked without a panic
  #   3. File-type sniffing decodes the first 256 (SVG) or 1024 (ipynb) bytes as UTF-8; a character cut at the end of that window is dropped, while an invalid byte inside it means the data is not text
  #   4. A workspace guard reports each range slice bounded by an integer literal or a constant unless it is on the exact list of ASCII-only or non-text slices
  #
  # ========================================
  Background: User Story
    As a user whose text holds multi-byte characters
    I want every preview, truncation and mask to cut on a character boundary
    So that no tool result, reasoning, quoted turn, loop prompt or API key crashes fspec

  Scenario: A tool-result preview cuts before a multi-byte character
    Given a tool result of 499 ASCII bytes followed by "é" and more text
    When the CLI builds its preview
    Then the preview is the 499 ASCII bytes followed by "..."

  Scenario: Recovery reasoning cuts before a multi-byte character
    Given captured reasoning of 1999 ASCII bytes followed by "€" and more text
    When the thinking-exhaustion recovery message is built
    Then it quotes exactly the 1999 ASCII bytes of reasoning

  Scenario: A quoted context turn cuts before a multi-byte character
    Given turn content of 1999 ASCII bytes followed by "😀" and more text
    When the content is truncated for a quoted-context block
    Then it is the 1999 ASCII bytes followed by "... [truncated]"

  Scenario: The /loop list prompt column cuts before a multi-byte character
    Given an App whose open session has a loop whose prompt is 24 ASCII bytes followed by "é" and more text
    When /loop list runs
    Then the notice lists the prompt as the 24 ASCII bytes followed by "..."

  Scenario: A non-ASCII API key is masked on char boundaries
    Given the API key "abcde€fghijklm€€"
    When it is masked
    Then the mask is "abcde••••••••€"
    And an ASCII key "sk-ant-abcdefghijk1234" still masks as "sk-ant-••••••••1234"

  Scenario: File-type sniffing ignores a character cut at the window's end
    Given an SVG document whose first 256 bytes end inside a multi-byte character
    And an ipynb document whose first 1024 bytes end inside a multi-byte character
    When their magic bytes are sniffed
    Then they are detected as SVG and ipynb
    And data that starts with "<svg" but holds an invalid UTF-8 byte inside the window is not detected as SVG

  Scenario: A new fixed-length text cut fails the workspace guard
    Given a source file holding "let p = &title[..40];"
    When the guard scans it
    Then it reports the slice "&title[..40]" with its line
    And every literal- or constant-bounded slice in the workspace sources is on the guard's exact list
