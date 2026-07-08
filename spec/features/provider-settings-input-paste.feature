@done
@clipboard
@provider-settings
@tui
@rust
@PROV-137
Feature: Paste support for /provider view input areas (profile form + API key), API key stays masked on paste
  """
  Bracketed paste (EnableBracketedPaste in terminal.rs) delivers a single Event::Paste(String). The Navigator's handle_provider_settings_event (navigator_events.rs) must gain an Event::Paste arm that forwards to ProviderSettingsView::handle_paste, which dispatches like handle_key but only acts in the CreateProfile/EditProfile/EditApiKey input modes. Single-line fields filter each pasted char through the existing charset gate (printable ASCII 32..=126 for API key; (' '..='~') for form fields), which also strips newlines. The API key stays masked because no new render path is added — the draft holds the true bytes and detail.rs already renders '•'.repeat(len). Reuses the role_dialog / multiline_input_paste pattern.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Pasting into the profile form's focused field inserts the pasted text at that field
  #   2. Pasting into the inline API-key entry appends the pasted text to the key draft
  #   3. Newlines and control characters in a pasted payload are stripped for the single-line provider fields
  #   4. After pasting an API key the field stays masked, rendering only bullet dots and never the plaintext secret
  #   5. Pasting while a non-input mode is focused is a no-op
  #
  # EXAMPLES:
  #   1. Focus the Base URL field, paste "https://api.example.com", the field shows that URL
  #   2. Focus the API Key field in the profile form, paste "sk-secret123", the field renders 12 bullet dots and Save stores "sk-secret123"
  #   3. In the inline API-key entry, paste "sk-abc\ndef", the draft becomes "sk-abcdef" (newline stripped) and renders 9 bullet dots
  #   4. Paste while on the provider List, nothing changes
  #
  # ========================================
  Background: User Story
    As a Provider Settings user entering credentials
    I want to paste text into the profile form fields and the API key entry
    So that I can paste API keys and URLs from my password manager without retyping them, and my API key stays masked

  Scenario: Pasting a URL into the focused Base URL field inserts it
    Given the profile create form is open with the Base URL field focused
    When I paste the text "https://api.example.com"
    Then the Base URL field contains "https://api.example.com"

  Scenario: Pasting an API key into the profile form keeps the field masked
    Given the profile create form is open with the API Key field focused
    When I paste the text "sk-secret123"
    Then the API Key field stores the value "sk-secret123"
    And the API Key field renders 12 bullet dots and not the plaintext secret

  Scenario: Pasting a multi-line secret into the inline API-key entry strips newlines and stays masked
    Given the inline API-key entry is open with an empty draft
    When I paste the text "sk-abc\ndef"
    Then the API-key draft contains "sk-abcdef"
    And the inline API-key entry renders 9 bullet dots and not the plaintext secret

  Scenario: Pasting while the provider list is focused does nothing
    Given the provider list is focused with no input field open
    When I paste the text "https://api.example.com"
    Then the paste is ignored and no field value changes
