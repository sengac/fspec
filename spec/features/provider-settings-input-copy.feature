@done
@clipboard
@provider-settings
@tui
@rust
@PROV-138
Feature: Copy support for /provider view input areas (Ctrl+C copies focused field via OSC 52), API key copies masked

  """
  Ctrl+C copy uses the dominant Action-emit pattern: the provider input handler returns ProviderSettingsEvent::Emit(Action::CopyToClipboard(text)) which the existing App::handle_copy_to_clipboard (app/dispatch_scroll.rs) performs via self.clipboard.copy — no new App wiring, and the set_clipboard_writer_for_test seam already covers it. The masking transform ('•'.repeat(value.chars().count()), shared with the render in detail.rs/profile_form_render.rs) is applied in the VIEW before building the action, so the plaintext secret never enters the action bus. Ctrl+C is intercepted in the CreateProfile/EditProfile/EditApiKey input modes BEFORE mod.rs's blanket CONTROL/ALT consume (mod.rs:192-197). Depends on PROV-137 for the view plumbing.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Ctrl+C in the profile form copies the focused field's current value to the clipboard
  #   2. Ctrl+C in the inline API-key entry copies the key draft to the clipboard
  #   3. Copying the API-key field copies the masked bullet-dot value, never the plaintext secret
  #   4. Copying a non-secret field copies its plaintext value
  #   5. Ctrl+C outside an input mode does not copy a field value
  #
  # EXAMPLES:
  #   1. Focus Base URL = "https://api.example.com", press Ctrl+C, the clipboard receives "https://api.example.com"
  #   2. Focus profile-form API Key = "sk-secret123", press Ctrl+C, the clipboard receives the 12 bullet dots and not "sk-secret123"
  #   3. Inline API-key draft = "sk-abcdef", press Ctrl+C, the clipboard receives 9 bullet dots and not the plaintext
  #   4. Ctrl+C on the provider List copies no field value
  #
  # ========================================

  Background: User Story
    As a Provider Settings user entering credentials
    I want to press Ctrl+C to copy the focused input field's value to the clipboard
    So that I can copy a URL or field value out, while my API key is copied masked so the plaintext secret never leaves the field

  Scenario: Copying the focused Base URL field copies its plaintext value
    Given the profile create form is open with the Base URL field focused and containing "https://api.example.com"
    When I press Ctrl+C
    Then the clipboard receives "https://api.example.com"


  Scenario: Copying the profile form API Key field copies the masked value
    Given the profile create form is open with the API Key field focused and containing "sk-secret123"
    When I press Ctrl+C
    Then the clipboard receives 12 bullet dots and not the plaintext secret


  Scenario: Copying the inline API-key entry copies the masked draft
    Given the inline API-key entry is open with the draft "sk-abcdef"
    When I press Ctrl+C
    Then the clipboard receives 9 bullet dots and not the plaintext secret


  Scenario: Pressing Ctrl+C on the provider list copies no field value
    Given the provider list is focused with no input field open
    When I press Ctrl+C
    Then no field value is copied to the clipboard

