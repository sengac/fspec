@done
@agent-view
@ts-parity
@provider-settings
@tui
@rust
@RPC-161
Feature: Provider settings: filterPrintableChars ASCII 32-126 charset on API-key edit
  """
  Architecture: Add a private helper `fn is_printable_ascii(c: char) -> bool { (32u32..=126).contains(&(c as u32)) }` at the top of rust/fspec-tui/src/views/provider_settings/detail.rs (or in a tiny sibling helper module if cleaner). Modify the `KeyCode::Char(c) =>` arm of `handle_edit_key` (currently lines 135-146) to wrap the `draft.push(c)` and the validation-clearing branch inside `if is_printable_ascii(c) { ... }`. If the char is dropped, still emit ProviderSettingsEvent::Consumed and re-enter the same Detail::EditApiKey { draft } mode unchanged (mirrors the existing `_ =>` fall-through pattern, but keeps the buffer unchanged). This isolates RPC-161 to ~10 lines under detail.rs's 300 LoC ceiling (detail.rs is currently 233 lines).

  Test plan: New integration test file rust/fspec-tui/tests/provider_settings_api_key_charset_rpc161.rs driving ProviderSettingsView::handle_key with KeyCode::Char(c) at the boundary and interior of the printable range (' ', 'A', '~') plus representative dropped chars ('\t', '\x1F', '\x7F', 'é', '✓', '🔑'). Inline rust unit test at the bottom of detail.rs covers is_printable_ascii() directly for branch coverage. All tests have @step comments matching the Gherkin scenarios. Total ~9 integration scenarios + 1 unit test = ~10 tests.

  TS reference: src/tui/utils/providerSettingsHelpers.ts:39-47 (filterPrintableChars), src/tui/inputHandlers/apiKeyEditModeHandler.ts:51-54 (call site).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A char event is appended to the EditApiKey draft IFF its char code lies in the inclusive range 32..=126; chars outside that range are silently dropped — no draft mutation, no status text, no event/Action emitted, ProviderSettingsEvent::Consumed is still returned.
  #   2. The filter is enforced ONLY at the KeyCode::Char(c) arm of handle_edit_key — Backspace/Enter/Esc paths are untouched.
  #   3. Space (32) and tilde (126) are the inclusive boundaries — both MUST be accepted (regression guards against off-by-one).
  #   4. Dropped categories: ASCII control chars (0..=31), DEL (127), non-ASCII chars (>127).
  #   5. Dropping a char never clears the 'API key cannot be empty' validation status — only an ACCEPTED printable char clears that status.
  #   6. is_printable_ascii() is a free function so it can be unit-tested in isolation.
  #
  # EXAMPLES — see scenarios below.
  #
  # ========================================
  Background: User Story
    As a provider settings user
    I want to type characters into the API-key edit form
    So that only printable ASCII (codes 32–126) reaches the draft buffer and control / non-ASCII bytes are silently dropped, matching the TS filterPrintableChars contract

  Scenario: Typing a sequence of printable ASCII characters appends each to the draft
    Given I have opened /provider, selected an api_key provider, and entered the EditApiKey form
    And the draft is empty
    When I type the characters "s", "k", "-", and "t" one at a time
    Then the draft becomes "sk-t" (4 characters)
    And each keystroke emits ProviderSettingsEvent::Consumed
    And no inline validation status is shown
    And no Action is dispatched (no SaveProviderCredentials)

  Scenario: Space (ASCII 32) is accepted as the lower boundary of the printable range
    Given I am in the EditApiKey form with the draft "abc"
    When I press the space key (ASCII code 32)
    Then the draft becomes "abc " (4 characters, with a trailing space)
    And the keystroke emits ProviderSettingsEvent::Consumed

  Scenario: Tilde "~" (ASCII 126) is accepted as the upper boundary of the printable range
    Given I am in the EditApiKey form with the draft "abc"
    When I press the "~" key (ASCII code 126)
    Then the draft becomes "abc~" (4 characters)
    And the keystroke emits ProviderSettingsEvent::Consumed

  Scenario: Tab (ASCII 9) is silently dropped as a control character
    Given I am in the EditApiKey form with the draft "abc"
    When I press a key delivering KeyCode::Char('\t') (ASCII code 9)
    Then the draft remains "abc" (unchanged)
    And the keystroke still emits ProviderSettingsEvent::Consumed
    And no inline validation status is shown

  Scenario: Unit Separator (ASCII 31) is silently dropped as a control character
    Given I am in the EditApiKey form with the draft "abc"
    When I press a key delivering KeyCode::Char('\u{001F}') (ASCII code 31)
    Then the draft remains "abc"
    And the keystroke still emits ProviderSettingsEvent::Consumed

  Scenario: DEL (ASCII 127) is silently dropped as out-of-range
    Given I am in the EditApiKey form with the draft "abc"
    When I press a key delivering KeyCode::Char('\u{007F}') (ASCII code 127, DEL)
    Then the draft remains "abc"
    And the keystroke still emits ProviderSettingsEvent::Consumed

  Scenario: Non-ASCII Latin-1 character "é" (U+00E9) is silently dropped
    Given I am in the EditApiKey form with the draft "abc"
    When I press a key delivering KeyCode::Char('é') (U+00E9, code 233)
    Then the draft remains "abc"
    And the keystroke still emits ProviderSettingsEvent::Consumed

  Scenario: Non-ASCII BMP character "✓" (U+2713) is silently dropped
    Given I am in the EditApiKey form with the draft "abc"
    When I press a key delivering KeyCode::Char('✓') (U+2713)
    Then the draft remains "abc"
    And the keystroke still emits ProviderSettingsEvent::Consumed

  Scenario: Non-BMP emoji "🔑" (U+1F511) is silently dropped
    Given I am in the EditApiKey form with the draft "abc"
    When I press a key delivering KeyCode::Char('🔑') (U+1F511)
    Then the draft remains "abc"
    And the keystroke still emits ProviderSettingsEvent::Consumed

  Scenario: Dropping a non-printable char does NOT clear the empty-key validation status; a subsequent printable char does
    Given I am in the EditApiKey form with the draft ""
    And the inline status reads "API key cannot be empty"
    When I press a key delivering KeyCode::Char('é') (non-ASCII, dropped)
    Then the draft remains ""
    And the inline status still reads "API key cannot be empty"
    When I then press the "s" key (printable, accepted)
    Then the draft becomes "s"
    And the inline status is cleared (empty)

  Scenario: is_printable_ascii() helper classifies characters by ASCII code
    Given the helper function is_printable_ascii(c: char) -> bool exists in views/provider_settings/detail.rs
    When the helper is called with each of the chars ' ' (32), 'A' (65), '~' (126)
    Then it returns true for every one
    When the helper is called with each of the chars '\t' (9), '\u{001F}' (31), '\u{007F}' (127), 'é' (233), '🔑' (128017)
    Then it returns false for every one
